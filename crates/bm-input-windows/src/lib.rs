#[cfg(windows)]
mod capture;
#[cfg(windows)]
mod emulate;
#[cfg(windows)]
mod keymap;

use bm_core::InputEvent;
use tokio::sync::mpsc;

pub struct InputCapture {
    rx: mpsc::Receiver<InputEvent>,
    screen_width: f64,
    screen_height: f64,
    forwarding: bool,
    #[allow(dead_code)]
    handle: Option<tokio::task::JoinHandle<anyhow::Result<()>>>,
}

impl InputCapture {
    #[cfg(not(windows))]
    pub async fn new() -> anyhow::Result<Self> {
        Self::new_with_size(1920.0, 1080.0).await
    }

    #[cfg(not(windows))]
    pub async fn new_with_size(_w: f64, _h: f64) -> anyhow::Result<Self> {
        let (_tx, rx) = mpsc::channel(256);
        Ok(Self {
            rx,
            screen_width: 0.0,
            screen_height: 0.0,
            forwarding: false,
            handle: None,
        })
    }

    #[cfg(windows)]
    pub async fn new() -> anyhow::Result<Self> {
        Self::new_with_size(1920.0, 1080.0).await
    }

    #[cfg(windows)]
    pub async fn new_with_size(w: f64, h: f64) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel(256);
        let handle = tokio::task::spawn_blocking(move || {
            capture::run_message_pump(tx, w, h)
        });
        Ok(Self {
            rx,
            screen_width: w,
            screen_height: h,
            forwarding: false,
            handle: Some(handle),
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

pub struct InputEmulation;

impl InputEmulation {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(Self)
    }

    #[cfg(windows)]
    pub async fn mouse_move(&mut self, x: f64, y: f64) -> anyhow::Result<()> {
        emulate::mouse_move(x, y)
    }

    #[cfg(not(windows))]
    pub async fn mouse_move(&mut self, _x: f64, _y: f64) -> anyhow::Result<()> {
        Ok(())
    }

    #[cfg(windows)]
    pub async fn mouse_button(&mut self, button: u8, pressed: bool) -> anyhow::Result<()> {
        emulate::mouse_button(button, pressed)
    }

    #[cfg(not(windows))]
    pub async fn mouse_button(&mut self, _button: u8, _pressed: bool) -> anyhow::Result<()> {
        Ok(())
    }

    #[cfg(windows)]
    pub async fn mouse_scroll(&mut self, dx: f64, dy: f64) -> anyhow::Result<()> {
        emulate::mouse_scroll(dx, dy)
    }

    #[cfg(not(windows))]
    pub async fn mouse_scroll(&mut self, _dx: f64, _dy: f64) -> anyhow::Result<()> {
        Ok(())
    }

    #[cfg(windows)]
    pub async fn key_event(&mut self, keycode: u32, pressed: bool, modifiers: u32) -> anyhow::Result<()> {
        emulate::key_event(keycode, pressed, modifiers)
    }

    #[cfg(not(windows))]
    pub async fn key_event(&mut self, _keycode: u32, _pressed: bool, _modifiers: u32) -> anyhow::Result<()> {
        Ok(())
    }
}
