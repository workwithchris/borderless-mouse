use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    pub version: u32,
    pub hostname: String,
    pub display_size: (u32, u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloAck {
    pub version: u32,
    pub hostname: String,
    pub display_size: (u32, u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenInfo {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Button4,
    Button5,
    Unknown(u8),
}

impl From<u8> for MouseButton {
    fn from(b: u8) -> Self {
        match b {
            0 => MouseButton::Left,
            1 => MouseButton::Right,
            2 => MouseButton::Middle,
            3 => MouseButton::Button4,
            4 => MouseButton::Button5,
            _ => MouseButton::Unknown(b),
        }
    }
}

impl From<MouseButton> for u8 {
    fn from(b: MouseButton) -> u8 {
        match b {
            MouseButton::Left => 0,
            MouseButton::Right => 1,
            MouseButton::Middle => 2,
            MouseButton::Button4 => 3,
            MouseButton::Button5 => 4,
            MouseButton::Unknown(v) => v,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    Ping(u64),
    Pong(u64),

    Hello {
        version: u32,
        hostname: String,
        display_size: (u32, u32),
    },
    HelloAck {
        version: u32,
        hostname: String,
        display_size: (u32, u32),
    },

    MouseMove { x: f64, y: f64 },
    MouseMoveRel { dx: f64, dy: f64 },
    MouseButton { button: MouseButton, pressed: bool },
    MouseScroll { dx: f64, dy: f64 },

    KeyEvent { keycode: u32, pressed: bool, modifiers: u32 },

    ClipboardChanged { content: String },
    ClipboardRequest,
    ClipboardData { mime: Option<String>, data: Vec<u8> },

    ScreenLayout { screens: Vec<ScreenInfo> },
    CursorEnter,
    CursorLeave,
    Disconnect { reason: String },
}

pub const PROTOCOL_VERSION: u32 = 1;
pub const DEFAULT_PORT: u16 = 24800;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_hello_roundtrip() {
        let event = Event::Hello {
            version: 1,
            hostname: "test-host".into(),
            display_size: (1920, 1080),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: Event = serde_json::from_str(&json).unwrap();
        match back {
            Event::Hello { version, hostname, display_size } => {
                assert_eq!(version, 1);
                assert_eq!(hostname, "test-host");
                assert_eq!(display_size, (1920, 1080));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn serde_hello_ack_roundtrip() {
        let event = Event::HelloAck {
            version: 1,
            hostname: "server".into(),
            display_size: (2560, 1440),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: Event = serde_json::from_str(&json).unwrap();
        match back {
            Event::HelloAck { hostname, .. } => assert_eq!(hostname, "server"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn serde_mouse_move_roundtrip() {
        let event = Event::MouseMove { x: 100.5, y: 200.3 };
        let json = serde_json::to_string(&event).unwrap();
        let back: Event = serde_json::from_str(&json).unwrap();
        match back {
            Event::MouseMove { x, y } => {
                assert!((x - 100.5).abs() < 1e-10);
                assert!((y - 200.3).abs() < 1e-10);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn serde_mouse_move_rel_roundtrip() {
        let event = Event::MouseMoveRel { dx: -5.0, dy: 3.0 };
        let json = serde_json::to_string(&event).unwrap();
        let back: Event = serde_json::from_str(&json).unwrap();
        match back {
            Event::MouseMoveRel { dx, dy } => {
                assert!((dx + 5.0).abs() < 1e-10);
                assert!((dy - 3.0).abs() < 1e-10);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn serde_mouse_button_roundtrip() {
        for (btn, pressed) in [
            (MouseButton::Left, true),
            (MouseButton::Right, false),
            (MouseButton::Middle, true),
            (MouseButton::Button4, false),
            (MouseButton::Button5, true),
            (MouseButton::Unknown(99), false),
        ] {
            let event = Event::MouseButton { button: btn.clone(), pressed };
            let json = serde_json::to_string(&event).unwrap();
            let back: Event = serde_json::from_str(&json).unwrap();
            match back {
                Event::MouseButton { button: b, pressed: p } => {
                    assert_eq!(b, btn);
                    assert_eq!(p, pressed);
                }
                _ => panic!("wrong variant"),
            }
        }
    }

    #[test]
    fn serde_mouse_scroll_roundtrip() {
        let event = Event::MouseScroll { dx: 0.0, dy: -3.0 };
        let json = serde_json::to_string(&event).unwrap();
        let back: Event = serde_json::from_str(&json).unwrap();
        match back {
            Event::MouseScroll { dx, dy } => {
                assert!((dx).abs() < 1e-10);
                assert!((dy + 3.0).abs() < 1e-10);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn serde_key_event_roundtrip() {
        let event = Event::KeyEvent { keycode: 42, pressed: true, modifiers: 4 };
        let json = serde_json::to_string(&event).unwrap();
        let back: Event = serde_json::from_str(&json).unwrap();
        match back {
            Event::KeyEvent { keycode, pressed, modifiers } => {
                assert_eq!(keycode, 42);
                assert!(pressed);
                assert_eq!(modifiers, 4);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn serde_clipboard_roundtrip() {
        let event = Event::ClipboardChanged { content: "hello world".into() };
        let json = serde_json::to_string(&event).unwrap();
        let back: Event = serde_json::from_str(&json).unwrap();
        match back {
            Event::ClipboardChanged { content } => assert_eq!(content, "hello world"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn serde_clipboard_request() {
        let json = serde_json::to_string(&Event::ClipboardRequest).unwrap();
        let back: Event = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, Event::ClipboardRequest));
    }

    #[test]
    fn serde_cursor_enter_leave() {
        let events = [Event::CursorEnter, Event::CursorLeave];
        for event in &events {
            let json = serde_json::to_string(event).unwrap();
            let back: Event = serde_json::from_str(&json).unwrap();
            match event {
                Event::CursorEnter => assert!(matches!(back, Event::CursorEnter)),
                Event::CursorLeave => assert!(matches!(back, Event::CursorLeave)),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn serde_disconnect() {
        let event = Event::Disconnect { reason: "test shutdown".into() };
        let json = serde_json::to_string(&event).unwrap();
        let back: Event = serde_json::from_str(&json).unwrap();
        match back {
            Event::Disconnect { reason } => assert_eq!(reason, "test shutdown"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn serde_screen_layout() {
        let event = Event::ScreenLayout {
            screens: vec![
                ScreenInfo {
                    name: "eDP-1".into(),
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                    is_primary: true,
                },
                ScreenInfo {
                    name: "DP-1".into(),
                    x: 1920,
                    y: 0,
                    width: 2560,
                    height: 1440,
                    is_primary: false,
                },
            ],
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: Event = serde_json::from_str(&json).unwrap();
        match back {
            Event::ScreenLayout { screens } => {
                assert_eq!(screens.len(), 2);
                assert_eq!(screens[0].name, "eDP-1");
                assert_eq!(screens[1].width, 2560);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn serde_ping_pong() {
        let ping = Event::Ping(42);
        let pong = Event::Pong(42);
        for (event, expected) in [(&ping, "Ping"), (&pong, "Pong")] {
            let json = serde_json::to_string(event).unwrap();
            let back: Event = serde_json::from_str(&json).unwrap();
            match back {
                Event::Ping(n) => assert_eq!(n, 42),
                Event::Pong(n) => assert_eq!(n, 42),
                _ => panic!("expected {expected}"),
            }
        }
    }

    #[test]
    fn serde_mouse_button_conversion() {
        for i in 0..=5u8 {
            let btn = MouseButton::from(i);
            let back: u8 = btn.clone().into();
            assert_eq!(back, i, "roundtrip failed for {i}");
        }
    }

    #[test]
    fn serde_mouse_button_unknown_roundtrip() {
        let btn = MouseButton::from(255);
        assert_eq!(btn, MouseButton::Unknown(255));
        let back: u8 = btn.into();
        assert_eq!(back, 255);
    }

    #[test]
    fn deserialize_invalid_event() {
        let result: Result<Event, _> = serde_json::from_str("{\"BadVariant\": {}}");
        assert!(result.is_err());
    }

    #[test]
    fn constants_are_correct() {
        assert_eq!(PROTOCOL_VERSION, 1);
        assert_eq!(DEFAULT_PORT, 24800);
    }
}
