/// Map Windows virtual key code to evdev keycode.
pub fn vk_to_evdev(vk: u32) -> u32 {
    match vk {
        0x41 => 30,  // VK_A
        0x42 => 48,  // VK_B
        0x43 => 46,  // VK_C
        0x44 => 32,  // VK_D
        0x45 => 18,  // VK_E
        0x46 => 33,  // VK_F
        0x47 => 34,  // VK_G
        0x48 => 35,  // VK_H
        0x49 => 23,  // VK_I
        0x4A => 36,  // VK_J
        0x4B => 37,  // VK_K
        0x4C => 38,  // VK_L
        0x4D => 50,  // VK_M
        0x4E => 49,  // VK_N
        0x4F => 24,  // VK_O
        0x50 => 25,  // VK_P
        0x51 => 16,  // VK_Q
        0x52 => 19,  // VK_R
        0x53 => 31,  // VK_S
        0x54 => 20,  // VK_T
        0x55 => 22,  // VK_U
        0x56 => 47,  // VK_V
        0x57 => 17,  // VK_W
        0x58 => 45,  // VK_X
        0x59 => 21,  // VK_Y
        0x5A => 44,  // VK_Z

        0x30 => 11,  // 0
        0x31 => 2,   // 1
        0x32 => 3,   // 2
        0x33 => 4,   // 3
        0x34 => 5,   // 4
        0x35 => 6,   // 5
        0x36 => 7,   // 6
        0x37 => 8,   // 7
        0x38 => 9,   // 8
        0x39 => 10,  // 9

        0xBD => 12,  // OEM_MINUS  -
        0xBB => 13,  // OEM_PLUS   =
        0xDB => 26,  // OEM_4      [
        0xDD => 27,  // OEM_6      ]
        0xDC => 43,  // OEM_5      \
        0xBA => 39,  // OEM_1      ;
        0xDE => 40,  // OEM_7      '
        0xC0 => 41,  // OEM_3      `
        0xBC => 51,  // OEM_COMMA  ,
        0xBE => 52,  // OEM_PERIOD .
        0xBF => 53,  // OEM_2      /

        0x0D => 28,  // VK_RETURN
        0x09 => 15,  // VK_TAB
        0x20 => 57,  // VK_SPACE
        0x08 => 14,  // VK_BACK
        0x1B => 1,   // VK_ESCAPE

        0x10 => 42,  // VK_SHIFT (maps to left shift)
        0xA0 => 42,  // VK_LSHIFT
        0xA1 => 54,  // VK_RSHIFT
        0x14 => 58,  // VK_CAPITAL
        0x11 => 29,  // VK_CONTROL (maps to left control)
        0xA2 => 29,  // VK_LCONTROL
        0xA3 => 97,  // VK_RCONTROL
        0x12 => 56,  // VK_MENU (maps to left alt)
        0xA4 => 56,  // VK_LMENU
        0xA5 => 100, // VK_RMENU
        0x5B => 125, // VK_LWIN
        0x5C => 126, // VK_RWIN

        // F-keys
        0x70 => 59,  // VK_F1
        0x71 => 60,  // VK_F2
        0x72 => 61,  // VK_F3
        0x73 => 62,  // VK_F4
        0x74 => 63,  // VK_F5
        0x75 => 64,  // VK_F6
        0x76 => 65,  // VK_F7
        0x77 => 66,  // VK_F8
        0x78 => 67,  // VK_F9
        0x79 => 68,  // VK_F10
        0x7A => 87,  // VK_F11
        0x7B => 88,  // VK_F12

        // Navigation
        0x26 => 103, // VK_UP
        0x28 => 108, // VK_DOWN
        0x25 => 105, // VK_LEFT
        0x27 => 106, // VK_RIGHT
        0x24 => 102, // VK_HOME
        0x23 => 107, // VK_END
        0x21 => 104, // VK_PRIOR (Page Up)
        0x22 => 109, // VK_NEXT  (Page Down)
        0x2D => 110, // VK_INSERT
        0x2E => 111, // VK_DELETE

        // Keypad
        0x60 => 82,  // VK_NUMPAD0
        0x61 => 79,  // VK_NUMPAD1
        0x62 => 80,  // VK_NUMPAD2
        0x63 => 81,  // VK_NUMPAD3
        0x64 => 75,  // VK_NUMPAD4
        0x65 => 76,  // VK_NUMPAD5
        0x66 => 77,  // VK_NUMPAD6
        0x67 => 71,  // VK_NUMPAD7
        0x68 => 72,  // VK_NUMPAD8
        0x69 => 73,  // VK_NUMPAD9
        0x6F => 98,  // VK_DIVIDE
        0x6A => 55,  // VK_MULTIPLY
        0x6B => 74,  // VK_ADD
        0x6D => 78,  // VK_SUBTRACT
        0x6E => 83,  // VK_DECIMAL
        0x6C => 96,  // VK_SEPARATOR (Enter)

        _ => 0,
    }
}

/// Map evdev keycode to Windows virtual key code.
pub fn evdev_to_vk(ev: u32) -> u16 {
    match ev {
        30 => 0x41,  // A
        48 => 0x42,  // B
        46 => 0x43,  // C
        32 => 0x44,  // D
        18 => 0x45,  // E
        33 => 0x46,  // F
        34 => 0x47,  // G
        35 => 0x48,  // H
        23 => 0x49,  // I
        36 => 0x4A,  // J
        37 => 0x4B,  // K
        38 => 0x4C,  // L
        50 => 0x4D,  // M
        49 => 0x4E,  // N
        24 => 0x4F,  // O
        25 => 0x50,  // P
        16 => 0x51,  // Q
        19 => 0x52,  // R
        31 => 0x53,  // S
        20 => 0x54,  // T
        22 => 0x55,  // U
        47 => 0x56,  // V
        17 => 0x57,  // W
        45 => 0x58,  // X
        21 => 0x59,  // Y
        44 => 0x5A,  // Z

        11 => 0x30,  // 0
        2 => 0x31,   // 1
        3 => 0x32,   // 2
        4 => 0x33,   // 3
        5 => 0x34,   // 4
        6 => 0x35,   // 5
        7 => 0x36,   // 6
        8 => 0x37,   // 7
        9 => 0x38,   // 8
        10 => 0x39,  // 9

        12 => 0xBD,  // -
        13 => 0xBB,  // =
        26 => 0xDB,  // [
        27 => 0xDD,  // ]
        43 => 0xDC,  // \
        39 => 0xBA,  // ;
        40 => 0xDE,  // '
        41 => 0xC0,  // `
        51 => 0xBC,  // ,
        52 => 0xBE,  // .
        53 => 0xBF,  // /

        28 => 0x0D,  // Return
        15 => 0x09,  // Tab
        57 => 0x20,  // Space
        14 => 0x08,  // Backspace
        1 => 0x1B,   // Escape

        42 => 0xA0,  // Left Shift
        54 => 0xA1,  // Right Shift
        58 => 0x14,  // Caps Lock
        29 => 0xA2,  // Left Control
        97 => 0xA3,  // Right Control
        56 => 0xA4,  // Left Alt
        100 => 0xA5, // Right Alt
        125 => 0x5B, // Left Win
        126 => 0x5C, // Right Win

        59 => 0x70,  // F1
        60 => 0x71,  // F2
        61 => 0x72,  // F3
        62 => 0x73,  // F4
        63 => 0x74,  // F5
        64 => 0x75,  // F6
        65 => 0x76,  // F7
        66 => 0x77,  // F8
        67 => 0x78,  // F9
        68 => 0x79,  // F10
        87 => 0x7A,  // F11
        88 => 0x7B,  // F12

        103 => 0x26, // Up
        108 => 0x28, // Down
        105 => 0x25, // Left
        106 => 0x27, // Right
        102 => 0x24, // Home
        107 => 0x23, // End
        104 => 0x21, // Page Up
        109 => 0x22, // Page Down
        110 => 0x2D, // Insert
        111 => 0x2E, // Delete

        82 => 0x60,  // Numpad 0
        79 => 0x61,  // Numpad 1
        80 => 0x62,  // Numpad 2
        81 => 0x63,  // Numpad 3
        75 => 0x64,  // Numpad 4
        76 => 0x65,  // Numpad 5
        77 => 0x66,  // Numpad 6
        71 => 0x67,  // Numpad 7
        72 => 0x68,  // Numpad 8
        73 => 0x69,  // Numpad 9
        98 => 0x6F,  // Divide
        55 => 0x6A,  // Multiply
        74 => 0x6B,  // Add
        78 => 0x6D,  // Subtract
        83 => 0x6E,  // Decimal
        96 => 0x6C,  // Numpad Enter

        _ => 0,
    }
}
