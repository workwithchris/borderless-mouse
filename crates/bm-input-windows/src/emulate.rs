use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, MOUSEINPUT, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEEVENTF_WHEEL, SendInput, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::SetCursorPos;

use crate::keymap;

pub fn mouse_move(x: f64, y: f64) -> anyhow::Result<()> {
    unsafe {
        SetCursorPos(x as i32, y as i32)
            .map_err(|e| anyhow::anyhow!("SetCursorPos failed: {e}"))?;
    }
    Ok(())
}

pub fn mouse_button(button: u8, pressed: bool) -> anyhow::Result<()> {
    let (down_flag, up_flag) = match button {
        0 => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        1 => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
        2 => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
        _ => return Ok(()),
    };

    let flag = if pressed { down_flag } else { up_flag };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flag,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    unsafe {
        if SendInput(&[input], std::mem::size_of::<INPUT>() as i32) == 0 {
            anyhow::bail!("SendInput (mouse button) failed");
        }
    }
    Ok(())
}

pub fn mouse_scroll(dx: f64, dy: f64) -> anyhow::Result<()> {
    if dy != 0.0 {
        let delta = (dy * 120.0) as i32;
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: delta as u32,
                    dwFlags: MOUSEEVENTF_WHEEL,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        unsafe {
            if SendInput(&[input], std::mem::size_of::<INPUT>() as i32) == 0 {
                anyhow::bail!("SendInput (vscroll) failed");
            }
        }
    }
    if dx != 0.0 {
        let delta = (dx * 120.0) as i32;
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: delta as u32,
                    dwFlags: MOUSEEVENTF_HWHEEL,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        unsafe {
            if SendInput(&[input], std::mem::size_of::<INPUT>() as i32) == 0 {
                anyhow::bail!("SendInput (hscroll) failed");
            }
        }
    }
    Ok(())
}

pub fn key_event(keycode: u32, pressed: bool, _modifiers: u32) -> anyhow::Result<()> {
    let vk = keymap::evdev_to_vk(keycode);
    if vk == 0 {
        return Ok(());
    }

    let mut flags = KEYBD_EVENT_FLAGS(0);
    if !pressed {
        flags = KEYEVENTF_KEYUP;
    }

    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    unsafe {
        if SendInput(&[input], std::mem::size_of::<INPUT>() as i32) == 0 {
            anyhow::bail!("SendInput (key) failed");
        }
    }
    Ok(())
}
