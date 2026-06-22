pub struct InputEmulation;

impl InputEmulation {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(Self)
    }

    #[cfg(feature = "cg")]
    pub async fn mouse_move(&mut self, x: f64, y: f64) -> anyhow::Result<()> {
        cgevents::MouseEvent::move_to(cgevents::Point::new(x, y))
            .post(cgevents::TapLocation::Session)?;
        Ok(())
    }

    #[cfg(not(feature = "cg"))]
    pub async fn mouse_move(&mut self, _x: f64, _y: f64) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn mouse_button(&mut self, _button: u8, _pressed: bool) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn mouse_scroll(&mut self, _dx: f64, _dy: f64) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn key_event(&mut self, _keycode: u32, _pressed: bool, _modifiers: u32) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_emulation() {
        let emu = InputEmulation::new().await.unwrap();
        assert_eq!(emu.mouse_move(0.0, 0.0).await.unwrap(), ());
        assert_eq!(emu.mouse_button(0, true).await.unwrap(), ());
        assert_eq!(emu.mouse_scroll(0.0, 0.0).await.unwrap(), ());
        assert_eq!(emu.key_event(0, false, 0).await.unwrap(), ());
    }

    #[tokio::test]
    async fn mouse_move_values() {
        let mut emu = InputEmulation::new().await.unwrap();
        emu.mouse_move(1920.0, 1080.0).await.unwrap();
        emu.mouse_move(-1.0, -1.0).await.unwrap();
        emu.mouse_move(f64::MAX, f64::MIN).await.unwrap();
    }

    #[tokio::test]
    async fn mouse_button_all_types() {
        let mut emu = InputEmulation::new().await.unwrap();
        for btn in 0..=5u8 {
            emu.mouse_button(btn, true).await.unwrap();
            emu.mouse_button(btn, false).await.unwrap();
        }
    }

    #[tokio::test]
    async fn key_event_variations() {
        let mut emu = InputEmulation::new().await.unwrap();
        emu.key_event(42, true, 0).await.unwrap();
        emu.key_event(42, false, 0).await.unwrap();
        emu.key_event(0, true, 0xFFFFFFFF).await.unwrap();
        emu.key_event(u32::MAX, true, u32::MAX).await.unwrap();
    }
}
