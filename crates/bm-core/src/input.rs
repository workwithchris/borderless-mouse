use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Left,
    Right,
    Top,
    Bottom,
}

pub const MOD_SHIFT: u32 = 1 << 0;
pub const MOD_CONTROL: u32 = 1 << 1;
pub const MOD_ALT: u32 = 1 << 2;
pub const MOD_SUPER: u32 = 1 << 3;

#[derive(Debug, Clone)]
pub enum InputEvent {
    MouseMove(f64, f64),
    MouseButton(u8, bool),
    MouseScroll(f64, f64),
    KeyEvent(u32, bool, u32),
    EdgeReached(Direction),
    EdgeLeft,
}
