use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq)]
pub enum ClipboardChange {
    Local(String),
    Remote(String),
}

pub struct ClipboardSync {
    last_content: Option<String>,
    rx: mpsc::Receiver<ClipboardChange>,
    tx: mpsc::Sender<ClipboardChange>,
    #[allow(dead_code)]
    poll_interval: Duration,
}

impl ClipboardSync {
    pub fn new(poll_interval: Duration) -> Self {
        let (tx, rx) = mpsc::channel(64);
        Self {
            last_content: None,
            rx,
            tx,
            poll_interval,
        }
    }

    pub fn sender(&self) -> mpsc::Sender<ClipboardChange> {
        self.tx.clone()
    }

    pub async fn poll(&mut self) -> Option<ClipboardChange> {
        if let Ok(change) = self.rx.try_recv() {
            match &change {
                ClipboardChange::Remote(content) => {
                    self.last_content = Some(content.clone());
                    if let Err(e) = set_clipboard(content) {
                        tracing::warn!("failed to set clipboard: {e}");
                    }
                    return None;
                }
                ClipboardChange::Local(_) => return Some(change),
            }
        }

        if let Some(content) = get_clipboard() {
            let changed = self
                .last_content
                .as_ref()
                .map_or(true, |last| last != &content);
            if changed {
                self.last_content = Some(content.clone());
                return Some(ClipboardChange::Local(content));
            }
        }

        None
    }
}

fn get_clipboard() -> Option<String> {
    #[cfg(feature = "sync")]
    {
        arboard::Clipboard::new()
            .ok()
            .and_then(|mut c| c.get_text().ok())
    }
    #[cfg(not(feature = "sync"))]
    None
}

fn set_clipboard(_text: &str) -> anyhow::Result<()> {
    #[cfg(feature = "sync")]
    {
        let mut cb = arboard::Clipboard::new()?;
        cb.set_text(_text.to_owned())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_sync() -> ClipboardSync {
        ClipboardSync::new(Duration::from_millis(100))
    }

    #[tokio::test]
    async fn poll_empty_returns_none() {
        let mut sync = new_sync();
        let result = sync.poll().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn remote_update_processed() {
        let mut sync = new_sync();
        sync.sender()
            .send(ClipboardChange::Remote("remote text".into()))
            .await
            .unwrap();
        let result = sync.poll().await;
        // Remote updates are applied and return None
        assert!(result.is_none());
        // last_content should be updated
        assert_eq!(sync.last_content, Some("remote text".into()));
    }

    #[tokio::test]
    async fn multiple_remote_updates() {
        let mut sync = new_sync();
        let sender = sync.sender();
        sender.send(ClipboardChange::Remote("first".into())).await.unwrap();
        let r1 = sync.poll().await;
        assert!(r1.is_none());
        assert_eq!(sync.last_content, Some("first".into()));

        sender.send(ClipboardChange::Remote("second".into())).await.unwrap();
        let r2 = sync.poll().await;
        assert!(r2.is_none());
        assert_eq!(sync.last_content, Some("second".into()));
    }

    #[tokio::test]
    async fn sender_is_cloneable() {
        let sync = new_sync();
        let s1 = sync.sender();
        let s2 = sync.sender();
        s1.send(ClipboardChange::Remote("a".into())).await.unwrap();
        s2.send(ClipboardChange::Remote("b".into())).await.unwrap();

        let mut sync_clone = sync; // move
        let r1 = sync_clone.poll().await;
        assert!(r1.is_none());
        assert_eq!(sync_clone.last_content, Some("a".into()));

        let r2 = sync_clone.poll().await;
        assert!(r2.is_none());
        assert_eq!(sync_clone.last_content, Some("b".into()));
    }

    #[tokio::test]
    async fn remote_update_sets_last_content_before_first_local_poll() {
        let mut sync = new_sync();
        // last_content is initially None
        assert!(sync.last_content.is_none());

        // Send remote before any local poll
        sync.sender()
            .send(ClipboardChange::Remote("initial".into()))
            .await
            .unwrap();
        sync.poll().await;

        // last_content should be set to "initial"
        assert_eq!(sync.last_content, Some("initial".into()));
    }

    #[test]
    fn default_poll_interval() {
        let sync = ClipboardSync::new(Duration::from_millis(500));
        assert_eq!(sync.poll_interval, Duration::from_millis(500));
    }

    #[tokio::test]
    async fn channel_capacity() {
        let sync = new_sync();
        let sender = sync.sender();

        // Send more than channel capacity (64) to verify blocking
        for i in 0..70usize {
            sender
                .send(ClipboardChange::Remote(format!("msg-{i}")))
                .await
                .unwrap_or_else(|_| panic!("channel should accept msg {i}"));
        }
        // The channel capacity is 64 but mpsc channels accept additional messages
        // until the buffer is full; try_send would fail but send awaits
    }
}
