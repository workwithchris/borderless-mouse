use tokio::sync::mpsc;

pub enum BackgroundEvent {
    Started,
    Stopped,
    Error(String),
    Status(String),
    Log(String, String),
}

pub struct BackgroundTask {
    handle: Option<tokio::task::JoinHandle<()>>,
    stop_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl BackgroundTask {
    pub fn new() -> Self {
        Self {
            handle: None,
            stop_tx: None,
        }
    }

    pub fn is_running(&self) -> bool {
        self.handle.is_some()
    }

    pub fn start_server(
        &mut self,
        bind_addr: String,
        port: u16,
        events: mpsc::Sender<BackgroundEvent>,
    ) {
        let _ = events.blocking_send(BackgroundEvent::Started);
        let _ = events.blocking_send(BackgroundEvent::Status("starting server...".into()));

        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();

        let handle = tokio::spawn(async move {
            let addr = format!("{bind_addr}:{port}");
            let listener = match tokio::net::TcpListener::bind(&addr).await {
                Ok(l) => {
                    let _ = events.send(BackgroundEvent::Log("info".into(), format!("listening on {addr}"))).await;
                    let _ = events.send(BackgroundEvent::Status("listening".into())).await;
                    l
                }
                Err(e) => {
                    let _ = events.send(BackgroundEvent::Error(format!("bind failed: {e}"))).await;
                    return;
                }
            };

            let mut stop_rx = stop_rx;
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        let (stream, peer) = match result {
                            Ok(v) => v,
                            Err(e) => {
                                let _ = events.send(BackgroundEvent::Log("error".into(), format!("accept error: {e}"))).await;
                                continue;
                            }
                        };
                        let _ = events.send(BackgroundEvent::Log("info".into(), format!("connection from {peer}"))).await;

                        let ev = events.clone();
                        tokio::spawn(async move {
                            handle_client(stream, ev).await;
                        });
                    }
                    _ = &mut stop_rx => {
                        let _ = events.send(BackgroundEvent::Log("info".into(), "server stopped".into())).await;
                        break;
                    }
                }
            }
        });

        self.handle = Some(handle);
        self.stop_tx = Some(stop_tx);
    }

    pub fn start_client(
        &mut self,
        connect_addr: String,
        port: u16,
        events: mpsc::Sender<BackgroundEvent>,
    ) {
        let _ = events.blocking_send(BackgroundEvent::Started);
        let _ = events.blocking_send(BackgroundEvent::Status("connecting...".into()));

        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();

        let handle = tokio::spawn(async move {
            let addr = format!("{connect_addr}:{port}");
            let stream = match tokio::net::TcpStream::connect(&addr).await {
                Ok(s) => s,
                Err(e) => {
                    let _ = events.send(BackgroundEvent::Error(format!("connection failed: {e}"))).await;
                    return;
                }
            };
            let _ = events.send(BackgroundEvent::Log("info".into(), format!("connected to {addr}"))).await;
            let _ = events.send(BackgroundEvent::Status("connected".into())).await;

            let mut stop_rx = stop_rx;
            loop {
                tokio::select! {
                    _ = stream.readable() => {}
                    _ = &mut stop_rx => {
                        break;
                    }
                }
            }
            let _ = events.send(BackgroundEvent::Log("info".into(), "client stopped".into())).await;
        });

        self.handle = Some(handle);
        self.stop_tx = Some(stop_tx);
    }

    pub async fn stop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.await;
        }
    }
}

async fn handle_client(stream: tokio::net::TcpStream, events: mpsc::Sender<BackgroundEvent>) {
    use bm_core::transport::Connection;
    use bm_core::protocol::{Event, PROTOCOL_VERSION};

    let mut conn = match Connection::from_stream(stream) {
        Ok(c) => c,
        Err(e) => {
            let _ = events.send(BackgroundEvent::Error(format!("connection setup failed: {e}"))).await;
            return;
        }
    };

    let msg = match conn.read().await {
        Ok(Some(m)) => m,
        _ => return,
    };

    if let Event::Hello { version, hostname, .. } = msg {
        let _ = events.send(BackgroundEvent::Log("info".into(), format!("client hello: {hostname} v{version}"))).await;
        let _ = conn.write(&Event::HelloAck {
            version: PROTOCOL_VERSION,
            hostname: whoami(),
            display_size: (0, 0),
        }).await;
    }
}

fn whoami() -> String {
    std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into())
}
