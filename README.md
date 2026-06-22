# borderless-mouse

**Software KVM — share a single mouse and keyboard across multiple computers.**

borderless-mouse lets you control a secondary Mac from your primary Linux machine as if its screen were an extension of your desktop. Move the cursor past the edge of your Linux display, and it seamlessly appears on the Mac. Keyboard input, mouse clicks, and clipboard content follow.

No proprietary hardware, no subscription, no cloud dependency. A pure Rust, cross-platform daemon and desktop app.

---

## Architecture

```
┌──────────────────────┐          TCP/24800          ┌──────────────────────┐
│   Linux (Server)     │  ◄────── JSON-over-TCP ───►  │   macOS (Client)     │
│                      │                              │                      │
│  ┌────────────────┐  │                              │  ┌────────────────┐  │
│  │ Wayland Portal  │  │                              │  │  CGEvent API   │  │
│  │ (libei via reis)│  │                              │  │ (Core Graphics)│  │
│  └────────┬───────┘  │                              │  └───────┬────────┘  │
│           │          │                              │          │           │
│  ┌────────▼───────┐  │   MouseMove, Button, Key,    │  ┌───────▼────────┐  │
│  │  Event Loop    │──┼── Clipboard  ────────────────┼─►│  Event Loop    │  │
│  └────────────────┘  │                              │  └────────────────┘  │
│                      │                              │                      │
│  ┌────────────────┐  │                              │  ┌────────────────┐  │
│  │ Clipboard Poll │  │                              │  │ Clipboard Poll │  │
│  │  (arboard)     │  │                              │  │  (arboard)     │  │
│  └────────────────┘  │                              │  └────────────────┘  │
└──────────────────────┘                              └──────────────────────┘
```

### How it works

1. The **server** (your Linux machine) captures local input events through the **XDG Remote Desktop Portal** — the same portal used by screen-sharing and remote-desktop tools. No kernel modules, no root, no proprietary drivers.

2. Captured events (mouse movement, clicks, scrolls, keystrokes) are serialized as JSON and sent over a TCP connection to the **client** (your Mac).

3. The **client** receives each event and replays it locally using macOS Core Graphics (`CGEventPost`). To the Mac, the input appears identical to a directly connected mouse and keyboard.

4. **Clipboard synchronization** runs in both directions. Each machine polls its local clipboard every 500ms and broadcasts changes, so copying on one machine pastes on the other.

---

## Features

- **Seamless cursor movement** — mouse leaves one screen and appears on the next
- **Keyboard forwarding** — type on either machine as if it's directly connected
- **Mouse button forwarding** — clicks, right-clicks, and scrolling work across machines
- **Bi-directional clipboard sync** — copy on Linux, paste on macOS and vice versa
- **Desktop GUI** — egui-based native application for starting/stopping and monitoring
- **Headless mode** — CLI subcommands for server and client without a graphical environment
- **TOML configuration** — config file at `~/.config/borderless-mouse/config.toml`
- **TLS-ready wire format** — length-prefixed JSON frames are simple to wrap with TLS (planned)

---

## Requirements

### Linux (Server)
| Requirement | Minimum |
|-------------|---------|
| Display server | **Wayland** (X11 is not supported) |
| Desktop environment | GNOME 45+, KDE 6.1+, or any compositor with **InputCapture portal** support |
| Portal backend | `xdg-desktop-portal` + `xdg-desktop-portal-gnome` or `xdg-desktop-portal-kde` |

> The server captures input via the **Remote Desktop Portal**, which requires an active logind session. SSH sessions without `logind` will not work.

### macOS (Client)
| Requirement | Minimum |
|-------------|---------|
| OS | macOS 12.0+ (Monterey) |
| Permission | **Accessibility** access in System Settings → Privacy & Security → Accessibility |

> On macOS the client requires **Accessibility permissions** because it uses `CGEventPost` to inject input events. A prompt will appear on first run.

---

## Quick start

### Build

```bash
# Build everything (Linux with Wayland capture, macOS stubs)
cargo build --release

# On macOS, build with real CGEvent support:
cargo build --release --features "bm-input-macos/cg,bm-clipboard/sync"
```

The release binary is at `target/release/borderless-mouse`.

### Generate config

```bash
borderless-mouse init-config
```

Creates `~/.config/borderless-mouse/config.toml` with default values.

### Run — Desktop GUI

```bash
borderless-mouse
# or equivalently:
borderless-mouse gui
```

Opens the egui application window with mode selection, connection configuration, and a live log viewer.

### Run — Headless server (Linux)

```bash
borderless-mouse server --bind 0.0.0.0 --port 24800
```

Listens for client connections on port 24800 and begins capturing input via the Wayland Remote Desktop portal.

### Run — Headless client (macOS)

```bash
borderless-mouse client --connect 192.168.1.100 --port 24800
```

Connects to the server at the given address and starts emulating received events via Core Graphics.

---

## Usage

### Subcommands

| Command | Alias | Description |
|---------|-------|-------------|
| `gui` | `ui` | Open the desktop GUI (default) |
| `server` | | Start headless KVM server |
| `client` | | Start headless KVM client |
| `init-config` | | Create default config file |
| `config-path` | | Print config file path |

### Server options

```
borderless-mouse server [OPTIONS]

Options:
  --bind <BIND>      Address to listen on [default: 0.0.0.0]
  --port <PORT>      Port to listen on [default: 24800]
  --secret <SECRET>  Optional shared secret for authentication
```

### Client options

```
borderless-mouse client [OPTIONS]

Options:
  --connect <CONNECT>  Server address [default: 127.0.0.1]
  --port <PORT>        Server port [default: 24800]
  --secret <SECRET>    Optional shared secret for authentication
```

---

## Configuration

Config is stored in `~/.config/borderless-mouse/config.toml` (XDG-compliant path, determined by the `directories` crate).

```toml
[server]
bind_addr = "0.0.0.0"
port = 24800
secret = "optional-shared-key"

[[server.screens]]
left = "macbook"
right = null
top = null
bottom = null

[client]
connect_addr = "192.168.1.100"
port = 24800
secret = "optional-shared-key"
```

The `[[server.screens]]` section defines screen-edge-to-hostname mappings. When the cursor reaches a screen edge, the server looks up the target and forwards input. This is the mechanism that enables seamless multi-monitor-like behavior across machines.

---

## Wire Protocol

Messages are **JSON-encoded** and **length-prefixed** for framing over TCP.

```
┌──────────────────────────────┬──────────────────────────────┐
│        4 bytes (LE)          │        variable length        │
│       payload length         │      JSON-encoded Event       │
├──────────────────────────────┼──────────────────────────────┤
│         0x0000_002E          │  {"Ping": [42]}               │
└──────────────────────────────┴──────────────────────────────┘
```

**Maximum frame size:** 16 MB.

### Event types

| Event | Direction | Payload | Description |
|-------|-----------|---------|-------------|
| `Hello` | Client → Server | `{ version, hostname, display_size }` | Handshake initiation |
| `HelloAck` | Server → Client | `{ version, hostname, display_size }` | Handshake response |
| `MouseMove` | Bidirectional | `{ x: f64, y: f64 }` | Absolute pointer position |
| `MouseMoveRel` | Bidirectional | `{ dx: f64, dy: f64 }` | Relative pointer delta |
| `MouseButton` | Bidirectional | `{ button: "Left"|"Right"|..., pressed: bool }` | Button press/release |
| `MouseScroll` | Bidirectional | `{ dx: f64, dy: f64 }` | Scroll delta |
| `KeyEvent` | Bidirectional | `{ keycode: u32, pressed: bool, modifiers: u32 }` | Keyboard event |
| `ClipboardChanged` | Bidirectional | `{ content: string }` | Clipboard text broadcast |
| `CursorEnter` | Server → Client | — | Cursor has entered client screen |
| `CursorLeave` | Server → Client | — | Cursor has left client screen |
| `Disconnect` | Bidirectional | `{ reason: string }` | Graceful connection close |
| `Ping`/`Pong` | Bidirectional | `u64` id | Keep-alive / latency |

### Handshake flow

```
Client                          Server
  │                               │
  ├────── Hello ────────────────► │
  │       { version: 1,          │
  │         hostname: "Mac",     │
  │         display_size: [0,0] }│
  │                               │
  │ ◄────── HelloAck ────────────┤
  │         { version: 1,        │
  │           hostname: "Linux", │
  │           display_size: [0,0]│
  │                               │
  │   ◄────── Event stream ─────►│
  │       (mouse, keyboard,      │
  │        clipboard, ...)       │
```

---

## Project structure

```
borderless-mouse/
├── Cargo.toml              # Workspace root + main binary
├── src/
│   └── main.rs             # CLI entrypoint, server + client event loops
├── crates/
│   ├── bm-core/            # Protocol types, TCP transport, config, InputEvent
│   ├── bm-input-linux/     # Wayland capture via reis + ashpd
│   ├── bm-input-macos/     # CGEvent emulation stubs (macOS feature-gated)
│   ├── bm-clipboard/       # arboard-based polling clipboard sync
│   └── bm-gui/             # egui/eframe desktop GUI
└── target/                 # Build artifacts
```

### Crate responsibilities

| Crate | Depends on | Key details |
|-------|------------|-------------|
| `bm-core` | tokio, serde, serde_json, toml | Defines `Event` enum, `Connection` transport, `AppConfig` + `ScreenEdge` types, `InputEvent` enum. Re-exports everything at the crate root. |
| `bm-input-linux` | bm-core, reis, ashpd | Opens Remote Desktop portal session, connects to EIS, runs capture loop on `spawn_blocking` (reis types are not `Send`). |
| `bm-input-macos` | bm-core | No-op stubs on Linux. Real CGEvent emulation behind `feature = "cg"` (macOS-only). |
| `bm-clipboard` | bm-core, arboard | Polls local clipboard every 500ms. Receives remote updates via `mpsc` channel. Behind `feature = "sync"`. |
| `bm-gui` | eframe, egui, tokio, tracing-subscriber | `BorderlessApp` implements `eframe::App`. Registers custom `tracing` subscriber for in-app log display. |

---

## Development

### Build

```bash
# Full workspace build
cargo build

# Type-check only (faster)
cargo check

# Release build
cargo build --release
```

### macOS-specific build

```bash
# On macOS, enable real CGEvent support:
cargo build --features "bm-input-macos/cg,bm-clipboard/sync"
```

### Code style

This project uses Rust edition 2024. There is no formatter or linter configuration — `rustfmt` and `clippy` will use their defaults.

### Verification

```bash
cargo build  # sole verification command
cargo check  # type-check only
```

There are **no tests** in the repository at this stage.

---

## Limitations

- **X11 is not supported.** Input capture requires the Wayland Remote Desktop Portal (libei/portal protocol).
- **Edge detection is not yet wired** in the Wayland capture loop. The `InputEvent::EdgeReached` and `EdgeLeft` variants are defined in the protocol, and `find_target()` in `main.rs` reads `ScreenEdge` config entries, but the capture loop does not currently emit these events. Input forwarding is toggled by the presence of incoming remote mouse movement.
- **Only Linux → macOS direction** is currently implemented. Return movement (Mac → Linux) would require `CGEventTap` on macOS and a sender EIS context on the Linux side.
- **No TLS encryption** yet. The wire format is plain JSON over TCP. TLS via `rustls` is planned.
- **Clipboard is text-only.** Images and rich content are not synchronized.
- **No latency measurement or adaptive throttling.** The protocol has `Ping`/`Pong` but no RTT-based event filtering.

---

## Security

- The protocol is currently **unencrypted**. Do not expose the server port to untrusted networks.
- A shared-secret authentication field is reserved in the config and protocol (`--secret` / `Event::Authenticate`) but not yet enforced.
- The Remote Desktop portal shows a permission dialog when the server starts. The user must explicitly grant capture permission.

---

## License

MIT
