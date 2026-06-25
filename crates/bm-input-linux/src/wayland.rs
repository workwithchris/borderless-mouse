use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use ashpd::desktop::remote_desktop::{DeviceType, RemoteDesktop, SelectDevicesOptions};
use reis::ei::button::ButtonState;
use reis::ei::handshake::ContextType;
use reis::ei::keyboard::KeyState;
use reis::ei::Context;
use reis::event::{DeviceCapability, EiEvent};
use tokio::sync::mpsc;

use bm_core::input::{Direction, InputEvent, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_SUPER};

pub struct WaylandCapture {
    pub handle: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl WaylandCapture {
    pub async fn spawn(
        tx: mpsc::Sender<InputEvent>,
        screen_width: f64,
        screen_height: f64,
    ) -> anyhow::Result<Self> {
        let proxy = RemoteDesktop::new().await?;
        let session = proxy.create_session(Default::default()).await?;

        proxy
            .select_devices(
                &session,
                SelectDevicesOptions::default()
                    .set_devices(DeviceType::Keyboard | DeviceType::Pointer),
            )
            .await?;

        let response = proxy
            .start(&session, None, Default::default())
            .await?
            .response()?;

        tracing::info!("remote desktop session started: {:?}", response.devices());

        let fd: OwnedFd = proxy.connect_to_eis(&session, Default::default()).await?;
        let raw_fd = fd.as_raw_fd();

        let dup = unsafe { libc::dup(raw_fd) };
        if dup < 0 {
            anyhow::bail!("failed to dup eis fd");
        }
        let stream = unsafe { std::os::unix::net::UnixStream::from_raw_fd(dup) };
        let context = Context::new(stream)?;

        // Spawn a dedicated OS thread for the capture loop.
        // The reis context types are not Send, so they must stay on one thread.
        let handle = tokio::task::spawn_blocking(move || {
            run_capture_loop_blocking(context, tx, screen_width, screen_height)
        });

        Ok(Self { handle })
    }
}

fn run_capture_loop_blocking(
    context: Context,
    tx: mpsc::Sender<InputEvent>,
    screen_width: f64,
    screen_height: f64,
) -> anyhow::Result<()> {
    let (_connection, mut iter) = context
        .handshake_blocking("borderless-mouse", ContextType::Receiver)?;

    tracing::info!("connected to EIS implementation (receiver mode)");

    let mut modifiers: u32 = 0;
    let mut abs_x: f64 = screen_width / 2.0;
    let mut abs_y: f64 = screen_height / 2.0;
    let mut was_outside = false;

    while let Some(ev_result) = iter.next() {
        let event = match ev_result {
            Ok(event) => event,
            Err(e) => {
                tracing::error!("EIS event error: {e}");
                continue;
            }
        };
        match event {
            EiEvent::Disconnected(_) => {
                tracing::info!("EIS disconnected");
                break;
            }
            EiEvent::SeatAdded(seat) => {
                tracing::info!("seat added, binding to pointer + keyboard");
                seat.seat.bind_capabilities(
                    DeviceCapability::Pointer
                        | DeviceCapability::PointerAbsolute
                        | DeviceCapability::Button
                        | DeviceCapability::Scroll
                        | DeviceCapability::Keyboard,
                );
            }
            EiEvent::DeviceAdded(device) => {
                tracing::info!("device added, starting emulation");
                device.device.device().start_emulating(0, 0);
            }
            EiEvent::PointerMotion(motion) => {
                abs_x = (abs_x + motion.dx as f64).clamp(-1.0, screen_width + 1.0);
                abs_y = (abs_y + motion.dy as f64).clamp(-1.0, screen_height + 1.0);
                check_edge(&tx, &mut was_outside, abs_x, abs_y, screen_width, screen_height);
                let _ = tx.blocking_send(InputEvent::MouseMove(
                    motion.dx as f64,
                    motion.dy as f64,
                ));
            }
            EiEvent::PointerMotionAbsolute(motion) => {
                abs_x = motion.dx_absolute as f64;
                abs_y = motion.dy_absolute as f64;
                check_edge(&tx, &mut was_outside, abs_x, abs_y, screen_width, screen_height);
                let _ = tx.blocking_send(InputEvent::MouseMove(
                    motion.dx_relative as f64,
                    motion.dy_relative as f64,
                ));
            }
            EiEvent::Button(btn) => {
                let pressed = matches!(btn.state, ButtonState::Press);
                let _ = tx
                    .blocking_send(InputEvent::MouseButton(btn.button as u8, pressed));
            }
            EiEvent::ScrollDelta(scroll) => {
                let _ = tx
                    .blocking_send(InputEvent::MouseScroll(scroll.dx as f64, scroll.dy as f64));
            }
            EiEvent::KeyboardKey(kb) => {
                let pressed = matches!(kb.state, KeyState::Press);
                let mod_flag = match kb.key {
                    42 | 54 => MOD_SHIFT,
                    29 | 97 => MOD_CONTROL,
                    56 | 100 => MOD_ALT,
                    125 | 126 => MOD_SUPER,
                    _ => 0,
                };
                if mod_flag != 0 {
                    if pressed {
                        modifiers |= mod_flag;
                    } else {
                        modifiers &= !mod_flag;
                    }
                }
                let _ = tx.blocking_send(InputEvent::KeyEvent(kb.key, pressed, modifiers));
            }
            _ => {}
        }
    }

    tracing::info!("EIS event loop ended");
    Ok(())
}

fn check_edge(
    tx: &mpsc::Sender<InputEvent>,
    was_outside: &mut bool,
    abs_x: f64,
    abs_y: f64,
    screen_w: f64,
    screen_h: f64,
) {
    let outside = abs_x <= 0.0 || abs_x >= screen_w || abs_y <= 0.0 || abs_y >= screen_h;

    if outside && !*was_outside {
        let dir = if abs_x <= 0.0 {
            Direction::Left
        } else if abs_x >= screen_w {
            Direction::Right
        } else if abs_y <= 0.0 {
            Direction::Top
        } else {
            Direction::Bottom
        };
        let _ = tx.blocking_send(InputEvent::EdgeReached(dir, abs_x, abs_y));
    } else if !outside && *was_outside {
        let _ = tx.blocking_send(InputEvent::EdgeLeft);
    }
    *was_outside = outside;
}
