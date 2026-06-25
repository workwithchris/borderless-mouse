use std::sync::{Mutex, OnceLock};

use bm_core::input::{Direction, InputEvent, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_SUPER};
use tokio::sync::mpsc;
use windows::Win32::Foundation::{BOOL, HINSTANCE, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, HHOOK,
    MSG, WH_KEYBOARD_LL, WH_MOUSE_LL,
};

use crate::keymap;

static EVENT_TX: OnceLock<mpsc::Sender<InputEvent>> = OnceLock::new();
static STATE: OnceLock<Mutex<CaptureState>> = OnceLock::new();

struct CaptureState {
    abs_x: f64,
    abs_y: f64,
    prev_x: f64,
    prev_y: f64,
    was_outside: bool,
    modifiers: u32,
    screen_width: f64,
    screen_height: f64,
}

#[repr(C)]
struct MouseHookData {
    pt: POINT,
    mouse_data: u32,
    flags: u32,
    time: u32,
    dw_extra_info: usize,
}

#[repr(C)]
struct KbdHookData {
    vk_code: u32,
    scan_code: u32,
    flags: u32,
    time: u32,
    dw_extra_info: usize,
}

pub fn run_message_pump(
    tx: mpsc::Sender<InputEvent>,
    screen_width: f64,
    screen_height: f64,
) -> anyhow::Result<()> {
    EVENT_TX
        .set(tx)
        .map_err(|_| anyhow::anyhow!("EVENT_TX already set"))?;

    STATE
        .set(Mutex::new(CaptureState {
            abs_x: screen_width / 2.0,
            abs_y: screen_height / 2.0,
            prev_x: screen_width / 2.0,
            prev_y: screen_height / 2.0,
            was_outside: false,
            modifiers: 0,
            screen_width,
            screen_height,
        }))
        .map_err(|_| anyhow::anyhow!("STATE already set"))?;

    unsafe {
        let hmod: HINSTANCE = GetModuleHandleW(None)
            .map_err(|e| anyhow::anyhow!("GetModuleHandleW failed: {e}"))?
            .into();

        let mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), hmod, 0)
            .map_err(|_| anyhow::anyhow!("SetWindowsHookExW(WH_MOUSE_LL) failed"))?;
        if mouse_hook.0.is_null() {
            anyhow::bail!("SetWindowsHookExW(WH_MOUSE_LL) returned null");
        }

        let _kbd_hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(kbd_proc), hmod, 0)
            .map_err(|_| anyhow::anyhow!("SetWindowsHookExW(WH_KEYBOARD_LL) failed"))?;
        if _kbd_hook.0.is_null() {
            anyhow::bail!("SetWindowsHookExW(WH_KEYBOARD_LL) returned null");
        }

        let mut msg = MSG::default();
        loop {
            let ret = GetMessageW(&mut msg, None, 0, 0);
            if ret == BOOL(0) {
                break;
            }
            if ret.0 == -1 {
                break;
            }
            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
        }
    }

    Ok(())
}

unsafe extern "system" fn mouse_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if ncode >= 0 {
        let data = &*(lparam.0 as *const MouseHookData);
        if let Some(tx) = EVENT_TX.get() {
            if let Some(state_mutex) = STATE.get() {
                let wm_msg = wparam.0 as u32;
                process_mouse_message(wm_msg, data, tx, state_mutex);
            }
        }
    }
    CallNextHookEx(HHOOK::default(), ncode, wparam, lparam)
}

unsafe extern "system" fn kbd_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if ncode >= 0 {
        let data = &*(lparam.0 as *const KbdHookData);
        if let Some(tx) = EVENT_TX.get() {
            if let Some(state_mutex) = STATE.get() {
                let wm_msg = wparam.0 as u32;
                process_kbd_message(wm_msg, data, tx, state_mutex);
            }
        }
    }
    CallNextHookEx(HHOOK::default(), ncode, wparam, lparam)
}

fn process_mouse_message(
    msg: u32,
    data: &MouseHookData,
    tx: &mpsc::Sender<InputEvent>,
    state_mutex: &Mutex<CaptureState>,
) {
    let abs_x = data.pt.x as f64;
    let abs_y = data.pt.y as f64;

    let mut s = state_mutex.lock().unwrap();
    let dx = abs_x - s.prev_x;
    let dy = abs_y - s.prev_y;
    s.abs_x = abs_x;
    s.abs_y = abs_y;
    s.prev_x = abs_x;
    s.prev_y = abs_y;

    match msg {
        0x0200 => {
            check_edge(tx, &mut s, abs_x, abs_y);
            let _ = tx.blocking_send(InputEvent::MouseMove(dx, dy));
        }
        0x0201 => {
            let _ = tx.blocking_send(InputEvent::MouseMove(dx, dy));
            let _ = tx.blocking_send(InputEvent::MouseButton(0, true));
        }
        0x0202 => {
            let _ = tx.blocking_send(InputEvent::MouseMove(dx, dy));
            let _ = tx.blocking_send(InputEvent::MouseButton(0, false));
        }
        0x0204 => {
            let _ = tx.blocking_send(InputEvent::MouseMove(dx, dy));
            let _ = tx.blocking_send(InputEvent::MouseButton(1, true));
        }
        0x0205 => {
            let _ = tx.blocking_send(InputEvent::MouseMove(dx, dy));
            let _ = tx.blocking_send(InputEvent::MouseButton(1, false));
        }
        0x0207 => {
            let _ = tx.blocking_send(InputEvent::MouseMove(dx, dy));
            let _ = tx.blocking_send(InputEvent::MouseButton(2, true));
        }
        0x0208 => {
            let _ = tx.blocking_send(InputEvent::MouseMove(dx, dy));
            let _ = tx.blocking_send(InputEvent::MouseButton(2, false));
        }
        0x020A => {
            let delta = ((data.mouse_data >> 16) as i16) as f64 / 120.0;
            let _ = tx.blocking_send(InputEvent::MouseScroll(0.0, delta));
        }
        0x020E => {
            let delta = ((data.mouse_data >> 16) as i16) as f64 / 120.0;
            let _ = tx.blocking_send(InputEvent::MouseScroll(delta, 0.0));
        }
        _ => {}
    }
}

fn process_kbd_message(
    msg: u32,
    data: &KbdHookData,
    tx: &mpsc::Sender<InputEvent>,
    state_mutex: &Mutex<CaptureState>,
) {
    match msg {
        0x0100 | 0x0101 => {
            let pressed = msg == 0x0100;
            let evdev = keymap::vk_to_evdev(data.vk_code);
            if evdev == 0 {
                return;
            }

            let mod_flag = match data.vk_code {
                0xA0 | 0xA1 | 0x10 => MOD_SHIFT,
                0xA2 | 0xA3 | 0x11 => MOD_CONTROL,
                0xA4 | 0xA5 | 0x12 => MOD_ALT,
                0x5B | 0x5C => MOD_SUPER,
                _ => 0,
            };

            let mut s = state_mutex.lock().unwrap();
            if mod_flag != 0 {
                if pressed {
                    s.modifiers |= mod_flag;
                } else {
                    s.modifiers &= !mod_flag;
                }
            }
            let mods = s.modifiers;
            drop(s);

            let _ = tx.blocking_send(InputEvent::KeyEvent(evdev, pressed, mods));
        }
        _ => {}
    }
}

fn check_edge(
    tx: &mpsc::Sender<InputEvent>,
    state: &mut CaptureState,
    abs_x: f64,
    abs_y: f64,
) {
    let outside = abs_x <= 0.0
        || abs_x >= state.screen_width
        || abs_y <= 0.0
        || abs_y >= state.screen_height;

    if outside && !state.was_outside {
        let dir = if abs_x <= 0.0 {
            Direction::Left
        } else if abs_x >= state.screen_width {
            Direction::Right
        } else if abs_y <= 0.0 {
            Direction::Top
        } else {
            Direction::Bottom
        };
        let _ = tx.blocking_send(InputEvent::EdgeReached(dir, abs_x, abs_y));
    } else if !outside && state.was_outside {
        let _ = tx.blocking_send(InputEvent::EdgeLeft);
    }
    state.was_outside = outside;
}
