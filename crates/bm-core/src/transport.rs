use crate::protocol::Event;
use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};

const MAX_MESSAGE_SIZE: u32 = 16 * 1024 * 1024;

pub struct Connection {
    reader: OwnedReadHalf,
    writer: OwnedWriteHalf,
}

impl Connection {
    pub async fn connect(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (reader, writer) = stream.into_split();
        Ok(Self { reader, writer })
    }

    pub fn from_stream(stream: TcpStream) -> Result<Self> {
        let (reader, writer) = stream.into_split();
        Ok(Self { reader, writer })
    }

    pub async fn write(&mut self, event: &Event) -> Result<()> {
        let data = serde_json::to_vec(event)?;
        let len = data.len() as u32;
        self.writer.write_u32_le(len).await?;
        self.writer.write_all(&data).await?;
        Ok(())
    }

    pub async fn read(&mut self) -> Result<Option<Event>> {
        let len = match self.reader.read_u32_le().await {
            Ok(len) => len,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        if len > MAX_MESSAGE_SIZE {
            anyhow::bail!("message too large: {len} bytes");
        }

        let mut buf = vec![0u8; len as usize];
        self.reader.read_exact(&mut buf).await?;
        let event: Event = serde_json::from_slice(&buf)?;
        Ok(Some(event))
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.writer.shutdown().await?;
        Ok(())
    }
}

pub async fn bind(addr: &str) -> Result<TcpListener> {
    let listener = TcpListener::bind(addr).await?;
    Ok(listener)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Event, MouseButton};
    use tokio::net::TcpListener as TokioListener;

    fn new_rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap()
    }

    fn run<F>(f: F)
    where
        F: std::future::Future<Output = ()>,
    {
        new_rt().block_on(f)
    }

    /// Create a connected server+client pair within the current runtime.
    async fn pair() -> (Connection, Connection) {
        let listener = TokioListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (server_stream, client_result) = tokio::join!(
            async { listener.accept().await.unwrap().0 },
            TcpStream::connect(addr),
        );
        let server = Connection::from_stream(server_stream).unwrap();
        let client = Connection::from_stream(client_result.unwrap()).unwrap();
        (server, client)
    }

    #[test]
    fn ping_pong() {
        run(async {
            let (mut server, mut client) = pair().await;
            tokio::join!(
                async {
                    let event = server.read().await.unwrap().unwrap();
                    match event {
                        Event::Ping(id) => server.write(&Event::Pong(id)).await.unwrap(),
                        _ => panic!("expected Ping"),
                    }
                },
                async {
                    client.write(&Event::Ping(42)).await.unwrap();
                    let resp = client.read().await.unwrap().unwrap();
                    assert!(matches!(resp, Event::Pong(42)));
                },
            );
        });
    }

    #[test]
    fn write_then_read() {
        run(async {
            let (mut server, mut client) = pair().await;
            let events = vec![
                Event::Hello {
                    version: 1,
                    hostname: "alpha".into(),
                    display_size: (1920, 1080),
                },
                Event::MouseMove { x: 500.0, y: 300.0 },
                Event::MouseButton { button: MouseButton::Left, pressed: true },
                Event::MouseScroll { dx: 0.0, dy: -5.0 },
                Event::KeyEvent { keycode: 42, pressed: true, modifiers: 2 },
                Event::ClipboardChanged { content: "test data".into() },
                Event::Disconnect { reason: "done".into() },
            ];

            for event in &events {
                client.write(event).await.unwrap();
            }
            for expected in &events {
                let received = server.read().await.unwrap().unwrap();
                assert_eq!(
                    serde_json::to_string(&received).unwrap(),
                    serde_json::to_string(expected).unwrap()
                );
            }
        });
    }

    #[test]
    fn batch_100() {
        run(async {
            let (mut server, mut client) = pair().await;
            let n = 100u64;
            tokio::join!(
                async {
                    for i in 0..n {
                        let event = server.read().await.unwrap().unwrap();
                        assert!(matches!(event, Event::Ping(id) if id == i));
                    }
                },
                async {
                    for i in 0..n {
                        client.write(&Event::Ping(i)).await.unwrap();
                    }
                },
            );
        });
    }

    #[test]
    fn interleaved() {
        run(async {
            let (mut server, mut client) = pair().await;
            tokio::join!(
                async {
                    let event = server.read().await.unwrap().unwrap();
                    assert!(matches!(event, Event::Ping(1)));
                    server.write(&Event::Pong(1)).await.unwrap();
                    server.write(&Event::Ping(2)).await.unwrap();
                    let event = server.read().await.unwrap().unwrap();
                    assert!(matches!(event, Event::Pong(2)));
                },
                async {
                    client.write(&Event::Ping(1)).await.unwrap();
                    let event = client.read().await.unwrap().unwrap();
                    assert!(matches!(event, Event::Pong(1)));
                    let event = client.read().await.unwrap().unwrap();
                    assert!(matches!(event, Event::Ping(2)));
                    client.write(&Event::Pong(2)).await.unwrap();
                },
            );
        });
    }

    #[test]
    fn oversized_message_constant() {
        assert_eq!(MAX_MESSAGE_SIZE, 16 * 1024 * 1024);
    }
}