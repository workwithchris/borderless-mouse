use std::time::Duration;

use bm_clipboard::{ClipboardChange, ClipboardSync};
#[allow(unused_imports)]
use bm_core::config::{config_path, load_config, save_config, AppConfig};
#[allow(unused_imports)]
use bm_core::input::{Direction, InputEvent};
#[allow(unused_imports)]
use bm_core::protocol::{Event, MouseButton, PROTOCOL_VERSION, DEFAULT_PORT};
#[allow(unused_imports)]
use bm_core::transport::{bind, Connection};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "borderless-mouse", about = "Share mouse, keyboard and clipboard between computers")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Open the desktop GUI
    #[command(visible_alias = "ui")]
    Gui,
    /// Run as KVM server (machine with physical keyboard/mouse)
    Server {
        #[arg(long, default_value = "0.0.0.0")]
        bind: String,
        #[arg(long, default_value_t = DEFAULT_PORT)]
        port: u16,
        #[arg(long)]
        secret: Option<String>,
    },
    /// Run as KVM client (secondary machine)
    Client {
        #[arg(long)]
        connect: Option<String>,
        #[arg(long, default_value_t = DEFAULT_PORT)]
        port: u16,
        #[arg(long)]
        secret: Option<String>,
    },
    /// Generate default config file
    InitConfig,
    /// Print current config path
    ConfigPath,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        None | Some(Command::Gui) => {
            let options = eframe::NativeOptions {
                viewport: egui::ViewportBuilder::default()
                    .with_inner_size([800.0, 600.0])
                    .with_min_inner_size([600.0, 400.0])
                    .with_title("borderless-mouse"),
                ..Default::default()
            };
            Ok(eframe::run_native(
                "borderless-mouse",
                options,
                Box::new(|_cc| Ok(Box::<bm_gui::BorderlessApp>::default())),
            )?)
        }
        Some(Command::Server { bind, port, secret: _ }) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(run_server(&bind, port))?;
            Ok(())
        }
        Some(Command::Client { connect, port, secret: _ }) => {
            let addr = connect.unwrap_or_else(|| "127.0.0.1".into());
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(run_client(&addr, port))?;
            Ok(())
        }
        Some(Command::InitConfig) => {
            let path = config_path();
            if path.exists() {
                tracing::warn!("config already exists at {path:?}");
            } else {
                save_config(&path, &AppConfig::default()).expect("failed to save config");
                tracing::info!("created default config at {path:?}");
            }
            Ok(())
        }
        Some(Command::ConfigPath) => {
            println!("{}", config_path().display());
            Ok(())
        }
    }
}

async fn run_server(bind_addr: &str, port: u16) -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    {
        run_server_impl(bind_addr, port).await
    }

    #[cfg(not(target_os = "linux"))]
    {
        run_server_lite(bind_addr, port).await
    }
}

#[cfg(target_os = "linux")]
async fn run_server_impl(bind_addr: &str, port: u16) -> anyhow::Result<()> {
    let addr = format!("{bind_addr}:{port}");
    let listener = bind(&addr).await?;
    tracing::info!("server listening on {addr}");

    let config = load_config(&config_path()).ok();
    let screens = config
        .as_ref()
        .and_then(|c| c.server.as_ref())
        .map(|s| s.screens.clone())
        .unwrap_or_default();

    loop {
        let (stream, peer) = listener.accept().await?;
        tracing::info!("connection from {peer}");

        let mut conn = Connection::from_stream(stream)?;

        let event = conn.read().await?.unwrap_or(Event::Disconnect {
            reason: "connection closed".into(),
        });

        match event {
            Event::Ping(id) => {
                conn.write(&Event::Pong(id)).await?;
                continue;
            }
            Event::Hello { version, hostname, .. } => {
                tracing::info!("client hello: {hostname} v{version}");
                conn.write(&Event::HelloAck {
                    version: PROTOCOL_VERSION,
                    hostname: hostname_(),
                    display_size: (0, 0),
                })
                .await?;
            }
            other => {
                tracing::warn!("unexpected first message: {other:?}");
                conn.write(&Event::Disconnect {
                    reason: "expected Hello".into(),
                })
                .await?;
                continue;
            }
        }

        let screens = screens.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_server_client(&mut conn, &screens).await {
                tracing::error!("client handler error: {e}");
            }
        });
    }
}

#[cfg(not(target_os = "linux"))]
async fn run_server_lite(bind_addr: &str, port: u16) -> anyhow::Result<()> {
    let addr = format!("{bind_addr}:{port}");
    let listener = bind(&addr).await?;
    tracing::info!("server listening on {addr} (lite mode — no input capture)");

    loop {
        let (stream, peer) = listener.accept().await?;
        tracing::info!("connection from {peer}");

        let mut conn = Connection::from_stream(stream)?;

        let event = conn.read().await?.unwrap_or(Event::Disconnect {
            reason: "connection closed".into(),
        });

        match event {
            Event::Ping(id) => {
                conn.write(&Event::Pong(id)).await?;
                continue;
            }
            Event::Hello { version, hostname, .. } => {
                tracing::info!("client hello: {hostname} v{version}");
                conn.write(&Event::HelloAck {
                    version: PROTOCOL_VERSION,
                    hostname: hostname_(),
                    display_size: (0, 0),
                })
                .await?;
            }
            other => {
                tracing::warn!("unexpected first message: {other:?}");
                conn.write(&Event::Disconnect {
                    reason: "expected Hello".into(),
                })
                .await?;
                continue;
            }
        }

        tokio::spawn(async move {
            if let Err(e) = handle_client_events(&mut conn).await {
                tracing::error!("client handler error: {e}");
            }
        });
    }
}

#[cfg(target_os = "linux")]
async fn handle_server_client(
    conn: &mut Connection,
    screens: &[bm_core::config::ScreenEdge],
) -> anyhow::Result<()> {
    let mut capture = bm_input_linux::InputCapture::new().await?;
    let mut clipboard = ClipboardSync::new(Duration::from_millis(500));

    tracing::info!("entering server event loop");

    loop {
        tokio::select! {
            local_event = capture.next_event() => {
                match local_event {
                    Some(InputEvent::EdgeReached(dir)) => {
                        tracing::info!("cursor reached edge: {dir:?}");
                        let target = find_target(screens, dir);
                        if let Some(target_name) = target {
                            tracing::info!("forwarding to: {target_name}");
                            conn.write(&Event::CursorEnter).await?;
                        }
                    }
                    Some(InputEvent::EdgeLeft) => {
                        conn.write(&Event::CursorLeave).await?;
                    }
                    Some(InputEvent::MouseMove(x, y)) => {
                        if capture.is_forwarding() {
                            conn.write(&Event::MouseMove { x, y }).await?;
                        }
                    }
                    Some(InputEvent::MouseButton(btn, pressed)) => {
                        if capture.is_forwarding() {
                            conn.write(&Event::MouseButton {
                                button: MouseButton::from(btn),
                                pressed,
                            }).await?;
                        }
                    }
                    Some(InputEvent::MouseScroll(dx, dy)) => {
                        if capture.is_forwarding() {
                            conn.write(&Event::MouseScroll { dx, dy }).await?;
                        }
                    }
                    Some(InputEvent::KeyEvent(keycode, pressed, modifiers)) => {
                        if capture.is_forwarding() {
                            conn.write(&Event::KeyEvent { keycode, pressed, modifiers }).await?;
                        }
                    }
                    None => {
                        tokio::time::sleep(Duration::from_millis(5)).await;
                    }
                }
            }

            remote = conn.read() => {
                match remote {
                    Ok(Some(Event::MouseMove { .. })) => {
                        if !capture.is_forwarding() {
                            capture.set_forwarding(true);
                        }
                    }
                    Ok(Some(Event::CursorLeave)) => {
                        capture.set_forwarding(false);
                    }
                    Ok(Some(Event::Disconnect { reason })) => {
                        tracing::info!("client disconnected: {reason}");
                        return Ok(());
                    }
                    Ok(Some(Event::Ping(id))) => {
                        conn.write(&Event::Pong(id)).await?;
                    }
                    Ok(None) => {
                        tracing::info!("client connection closed");
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::error!("connection error: {e}");
                        return Ok(());
                    }
                    _ => {}
                }
            }

            _ = tokio::time::sleep(Duration::from_millis(300)) => {
                if let Some(change) = clipboard.poll().await {
                    match change {
                        ClipboardChange::Local(content) => {
                            conn.write(&Event::ClipboardChanged { content }).await?;
                        }
                        ClipboardChange::Remote(_) => {}
                    }
                }
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
async fn handle_client_events(conn: &mut Connection) -> anyhow::Result<()> {
    let mut clipboard = ClipboardSync::new(Duration::from_millis(500));

    tracing::info!("entering server event loop (lite — clipboard + relay only)");

    loop {
        tokio::select! {
            remote = conn.read() => {
                match remote {
                    Ok(Some(Event::Disconnect { reason })) => {
                        tracing::info!("client disconnected: {reason}");
                        return Ok(());
                    }
                    Ok(Some(Event::Ping(id))) => {
                        conn.write(&Event::Pong(id)).await?;
                    }
                    Ok(None) => {
                        tracing::info!("client connection closed");
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::error!("connection error: {e}");
                        return Ok(());
                    }
                    _ => {}
                }
            }

            _ = tokio::time::sleep(Duration::from_millis(300)) => {
                if let Some(change) = clipboard.poll().await {
                    match change {
                        ClipboardChange::Local(content) => {
                            conn.write(&Event::ClipboardChanged { content }).await?;
                        }
                        ClipboardChange::Remote(_) => {}
                    }
                }
            }
        }
    }
}

async fn run_client(connect_addr: &str, port: u16) -> anyhow::Result<()> {
    let addr = format!("{connect_addr}:{port}");
    tracing::info!("connecting to server at {addr}");

    let mut conn = Connection::connect(&addr).await?;

    conn.write(&Event::Hello {
        version: PROTOCOL_VERSION,
        hostname: hostname_(),
        display_size: (0, 0),
    })
    .await?;

    let response = conn.read().await?;
    match response {
        Some(Event::HelloAck { version, hostname, .. }) => {
            tracing::info!("connected to server: {hostname} v{version}");
        }
        Some(Event::Disconnect { reason }) => {
            anyhow::bail!("server rejected connection: {reason}");
        }
        Some(other) => {
            anyhow::bail!("unexpected server response: {other:?}");
        }
        None => {
            anyhow::bail!("server closed connection during handshake");
        }
    }

    let mut emulation = bm_input_macos::InputEmulation::new().await?;
    let mut clipboard = ClipboardSync::new(Duration::from_millis(500));

    tracing::info!("entering client event loop");

    loop {
        tokio::select! {
            event = conn.read() => {
                match event {
                    Ok(Some(Event::MouseMove { x, y })) => {
                        emulation.mouse_move(x, y).await?;
                    }
                    Ok(Some(Event::MouseButton { button, pressed })) => {
                        emulation.mouse_button(button.into(), pressed).await?;
                    }
                    Ok(Some(Event::MouseScroll { dx, dy })) => {
                        emulation.mouse_scroll(dx, dy).await?;
                    }
                    Ok(Some(Event::KeyEvent { keycode, pressed, modifiers })) => {
                        emulation.key_event(keycode, pressed, modifiers).await?;
                    }
                    Ok(Some(Event::CursorEnter)) => {
                        tracing::info!("cursor entered client screen");
                    }
                    Ok(Some(Event::CursorLeave)) => {
                        tracing::info!("cursor left client screen");
                    }
                    Ok(Some(Event::ClipboardChanged { content })) => {
                        if let Err(e) = clipboard.sender().try_send(ClipboardChange::Remote(content)) {
                            tracing::warn!("clipboard channel error: {e}");
                        }
                    }
                    Ok(Some(Event::Ping(id))) => {
                        conn.write(&Event::Pong(id)).await?;
                    }
                    Ok(Some(Event::Disconnect { reason })) => {
                        tracing::info!("server disconnected: {reason}");
                        return Ok(());
                    }
                    Ok(None) => {
                        tracing::info!("server connection closed");
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::error!("connection error: {e}");
                        return Ok(());
                    }
                    _ => {}
                }
            }

            _ = tokio::time::sleep(Duration::from_millis(300)) => {
                if let Some(change) = clipboard.poll().await {
                    match change {
                        ClipboardChange::Local(content) => {
                            conn.write(&Event::ClipboardChanged { content }).await?;
                        }
                        ClipboardChange::Remote(_) => {}
                    }
                }
            }
        }
    }
}

fn hostname_() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| "unknown".into())
}

fn find_target(
    screens: &[bm_core::config::ScreenEdge],
    direction: Direction,
) -> Option<&str> {
    for screen in screens {
        let target = match direction {
            Direction::Left => screen.left.as_deref(),
            Direction::Right => screen.right.as_deref(),
            Direction::Top => screen.top.as_deref(),
            Direction::Bottom => screen.bottom.as_deref(),
        };
        if target.is_some() {
            return target;
        }
    }
    None
}
