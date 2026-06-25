#[cfg(feature = "wayland")]
mod wayland;

use bm_core::InputEvent;
use tokio::sync::mpsc;

pub struct InputCapture {
    rx: mpsc::Receiver<InputEvent>,
    screen_width: f64,
    screen_height: f64,
    forwarding: bool,
    #[allow(dead_code)]
    wayland_handle: Option<tokio::task::JoinHandle<anyhow::Result<()>>>,
}

impl InputCapture {
    #[cfg(feature = "wayland")]
    pub async fn new() -> anyhow::Result<Self> {
        Self::new_with_size(1920.0, 1080.0).await
    }

    #[cfg(feature = "wayland")]
    pub async fn new_with_size(screen_width: f64, screen_height: f64) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel(256);

        let wayland = crate::wayland::WaylandCapture::spawn(tx, screen_width, screen_height).await?;

        Ok(Self {
            rx,
            screen_width,
            screen_height,
            forwarding: false,
            wayland_handle: Some(wayland.handle),
        })
    }

    #[cfg(not(feature = "wayland"))]
    pub async fn new() -> anyhow::Result<Self> {
        Self::new_with_size(1920.0, 1080.0).await
    }

    #[cfg(not(feature = "wayland"))]
    pub async fn new_with_size(screen_width: f64, screen_height: f64) -> anyhow::Result<Self> {
        let (_tx, rx) = mpsc::channel(256);
        Ok(Self {
            rx,
            screen_width,
            screen_height,
            forwarding: false,
            wayland_handle: None,
        })
    }

    pub fn set_screen_size(&mut self, w: f64, h: f64) {
        self.screen_width = w;
        self.screen_height = h;
    }

    pub async fn next_event(&mut self) -> Option<InputEvent> {
        self.rx.recv().await
    }

    pub fn is_forwarding(&self) -> bool {
        self.forwarding
    }

    pub fn set_forwarding(&mut self, active: bool) {
        self.forwarding = active;
    }
}
