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

    #[cfg(feature = "cg")]
    pub async fn mouse_button(&mut self, button: u8, pressed: bool) -> anyhow::Result<()> {
        let pos = cgevents::MouseEvent::move_to(cgevents::Point::new(0.0, 0.0))
            .build(&cgevents::EventSource::private()?)?
            .location();
        let cg_button = match button {
            0 => cgevents::MouseButton::Left,
            1 => cgevents::MouseButton::Right,
            2 => cgevents::MouseButton::Center,
            n => cgevents::MouseButton::Other(n as u32),
        };
        let event = if pressed {
            cgevents::MouseEvent::button_down(pos, cg_button)
        } else {
            cgevents::MouseEvent::button_up(pos, cg_button)
        };
        event.post(cgevents::TapLocation::Session)?;
        Ok(())
    }

    #[cfg(not(feature = "cg"))]
    pub async fn mouse_button(&mut self, _button: u8, _pressed: bool) -> anyhow::Result<()> {
        Ok(())
    }

    #[cfg(feature = "cg")]
    pub async fn mouse_scroll(&mut self, dx: f64, dy: f64) -> anyhow::Result<()> {
        let scroll = cgevents::ScrollEvent::lines_2d(dy as i32, dx as i32);
        scroll.post(cgevents::TapLocation::Session)?;
        Ok(())
    }

    #[cfg(not(feature = "cg"))]
    pub async fn mouse_scroll(&mut self, _dx: f64, _dy: f64) -> anyhow::Result<()> {
        Ok(())
    }

    #[cfg(feature = "cg")]
    pub async fn key_event(&mut self, keycode: u32, pressed: bool, modifiers: u32) -> anyhow::Result<()> {
        let vk = evdev_to_macos_vkey(keycode);
        let mut key_event = if pressed {
            cgevents::KeyEvent::down(vk)
        } else {
            cgevents::KeyEvent::up(vk)
        };
        let mut flags = cgevents::CGEventFlags::empty();
        if modifiers & 1 != 0 {
            flags |= cgevents::CGEventFlags::SHIFT;
        }
        if modifiers & 2 != 0 {
            flags |= cgevents::CGEventFlags::CONTROL;
        }
        if modifiers & 4 != 0 {
            flags |= cgevents::CGEventFlags::ALTERNATE;
        }
        if modifiers & 8 != 0 {
            flags |= cgevents::CGEventFlags::COMMAND;
        }
        if !flags.is_empty() {
            key_event = key_event.with_modifiers(flags);
        }
        key_event.post(cgevents::TapLocation::Session)?;
        Ok(())
    }

    #[cfg(not(feature = "cg"))]
    pub async fn key_event(&mut self, _keycode: u32, _pressed: bool, _modifiers: u32) -> anyhow::Result<()> {
        Ok(())
    }
}

fn evdev_to_macos_vkey(keycode: u32) -> u16 {
    match keycode {
        // Letters
        30 => 0x00,  // A
        31 => 0x01,  // S
        32 => 0x02,  // D
        33 => 0x03,  // F
        35 => 0x04,  // H
        34 => 0x05,  // G
        44 => 0x06,  // Z
        45 => 0x07,  // X
        46 => 0x08,  // C
        47 => 0x09,  // V
        48 => 0x0B,  // B
        16 => 0x0C,  // Q
        17 => 0x0D,  // W
        18 => 0x0E,  // E
        19 => 0x0F,  // R
        21 => 0x10,  // Y
        20 => 0x11,  // T
        24 => 0x1F,  // O
        22 => 0x20,  // U
        23 => 0x22,  // I
        25 => 0x23,  // P
        38 => 0x25,  // L
        36 => 0x26,  // J
        37 => 0x28,  // K
        49 => 0x2D,  // N
        50 => 0x2E,  // M

        // Numbers
        2 => 0x12,   // 1
        3 => 0x13,   // 2
        4 => 0x14,   // 3
        5 => 0x15,   // 4
        6 => 0x17,   // 5
        7 => 0x16,   // 6
        8 => 0x1A,   // 7
        9 => 0x1C,   // 8
        10 => 0x19,  // 9
        11 => 0x1D,  // 0

        // Symbols
        12 => 0x1B,  // Minus
        13 => 0x18,  // Equal
        26 => 0x21,  // LeftBracket
        27 => 0x1E,  // RightBracket
        43 => 0x2A,  // Backslash
        39 => 0x29,  // Semicolon
        40 => 0x27,  // Quote
        41 => 0x32,  // Grave
        51 => 0x2B,  // Comma
        52 => 0x2F,  // Period
        53 => 0x2C,  // Slash

        // Special keys
        28 => 0x24,  // Enter/Return
        15 => 0x30,  // Tab
        57 => 0x31,  // Space
        14 => 0x33,  // Backspace/Delete
        1 => 0x35,   // Escape
        125 => 0x37, // Left Command
        126 => 0x37, // Right Command
        42 => 0x38,  // Left Shift
        54 => 0x3C,  // Right Shift
        58 => 0x39,  // Caps Lock
        56 => 0x3A,  // Left Option/Alt
        100 => 0x3D, // Right Option/Alt
        29 => 0x3B,  // Left Control
        97 => 0x3E,  // Right Control

        // F-keys
        59 => 0x7A,  // F1
        60 => 0x78,  // F2
        61 => 0x63,  // F3
        62 => 0x76,  // F4
        63 => 0x60,  // F5
        64 => 0x61,  // F6
        65 => 0x62,  // F7
        66 => 0x64,  // F8
        67 => 0x65,  // F9
        68 => 0x6D,  // F10
        87 => 0x67,  // F11
        88 => 0x6F,  // F12

        // Navigation
        103 => 0x7E, // Up Arrow
        108 => 0x7D, // Down Arrow
        105 => 0x7B, // Left Arrow
        106 => 0x7C, // Right Arrow
        102 => 0x73, // Home
        107 => 0x77, // End
        104 => 0x74, // Page Up
        109 => 0x79, // Page Down
        110 => 0x72, // Insert
        111 => 0x75, // Forward Delete

        // Keypad
        82 => 0x52,  // Keypad 0
        79 => 0x53,  // Keypad 1
        80 => 0x54,  // Keypad 2
        81 => 0x55,  // Keypad 3
        75 => 0x56,  // Keypad 4
        76 => 0x57,  // Keypad 5
        77 => 0x58,  // Keypad 6
        71 => 0x59,  // Keypad 7
        72 => 0x5B,  // Keypad 8
        73 => 0x5C,  // Keypad 9
        98 => 0x4B,  // Keypad Divide
        55 => 0x43,  // Keypad Multiply
        74 => 0x45,  // Keypad Plus
        78 => 0x4E,  // Keypad Minus
        83 => 0x41,  // Keypad Decimal
        96 => 0x4C,  // Keypad Enter

        _ => 0xFFFF, // unmapped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_emulation() {
        let mut emu = InputEmulation::new().await.unwrap();
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

    #[test]
    fn evdev_to_vkey_letters() {
        assert_eq!(evdev_to_macos_vkey(30), 0x00); // A
        assert_eq!(evdev_to_macos_vkey(31), 0x01); // S
        assert_eq!(evdev_to_macos_vkey(44), 0x06); // Z
        assert_eq!(evdev_to_macos_vkey(50), 0x2E); // M
    }

    #[test]
    fn evdev_to_vkey_numbers() {
        assert_eq!(evdev_to_macos_vkey(2), 0x12);  // 1
        assert_eq!(evdev_to_macos_vkey(11), 0x1D); // 0
    }

    #[test]
    fn evdev_to_vkey_special() {
        assert_eq!(evdev_to_macos_vkey(28), 0x24);  // Enter
        assert_eq!(evdev_to_macos_vkey(57), 0x31);  // Space
        assert_eq!(evdev_to_macos_vkey(1), 0x35);   // Escape
        assert_eq!(evdev_to_macos_vkey(14), 0x33);  // Backspace
        assert_eq!(evdev_to_macos_vkey(15), 0x30);  // Tab
    }

    #[test]
    fn evdev_to_vkey_modifiers() {
        assert_eq!(evdev_to_macos_vkey(42), 0x38);  // Left Shift
        assert_eq!(evdev_to_macos_vkey(54), 0x3C);  // Right Shift
        assert_eq!(evdev_to_macos_vkey(29), 0x3B);  // Left Control
        assert_eq!(evdev_to_macos_vkey(97), 0x3E);  // Right Control
        assert_eq!(evdev_to_macos_vkey(56), 0x3A);  // Left Alt
        assert_eq!(evdev_to_macos_vkey(100), 0x3D); // Right Alt
        assert_eq!(evdev_to_macos_vkey(125), 0x37); // Left Command
        assert_eq!(evdev_to_macos_vkey(126), 0x37); // Right Command
    }

    #[test]
    fn evdev_to_vkey_fkeys() {
        assert_eq!(evdev_to_macos_vkey(59), 0x7A); // F1
        assert_eq!(evdev_to_macos_vkey(60), 0x78); // F2
        assert_eq!(evdev_to_macos_vkey(88), 0x6F); // F12
    }

    #[test]
    fn evdev_to_vkey_arrows() {
        assert_eq!(evdev_to_macos_vkey(103), 0x7E); // Up
        assert_eq!(evdev_to_macos_vkey(108), 0x7D); // Down
        assert_eq!(evdev_to_macos_vkey(105), 0x7B); // Left
        assert_eq!(evdev_to_macos_vkey(106), 0x7C); // Right
    }

    #[test]
    fn evdev_to_vkey_unmapped_returns_ffff() {
        assert_eq!(evdev_to_macos_vkey(9999), 0xFFFF);
        assert_eq!(evdev_to_macos_vkey(u32::MAX), 0xFFFF);
    }
}
