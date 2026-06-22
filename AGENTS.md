# borderless-mouse

## Commands

- `cargo build` — full workspace build (root binary + 5 crates). Only verification command available.
- `cargo check` — type-check only. No tests, no CI, no linter or formatter config.

## Crate map

| Crate | Role | Key quirks |
|-------|------|------------|
| `bm-core` | Protocol types, TCP transport, TOML config, InputEvent enum | Re-exports everything via `pub use *`. Event enum variants are **struct variants** (`Event::Hello { version, hostname }`), NOT tuple variants. |
| `bm-input-linux` | Wayland input capture via `reis` + `ashpd` | **Not `Send`** — reis handshake types must stay on one thread. Uses `spawn_blocking` + `mpsc::blocking_send`. Behind `feature = "wayland"` (default on). |
| `bm-input-macos` | CGEvent emulation stubs | Behind `feature = "cg"` which requires `cgevents` (macOS-only). On Linux only no-op stubs compile. |
| `bm-clipboard` | `arboard`-based polling clipboard sync | Behind `feature = "sync"`. Polls every 500ms via `ClipboardSync::poll()`. Reads from `mpsc` for remote updates. |
| `bm-gui` | egui/eframe 0.31 desktop GUI | Uses `egui::Button::new(...).fill(...)` pattern (not `ui.button().fill()`). Registers a custom `tracing` subscriber (`LogCollector::init_as_global_subscriber()`) for in-app log display. |

## Protocol

- JSON-over-TCP, port **24800** (`DEFAULT_PORT`).
- Wire format: `[u32 LE length][JSON bytes]`, max 16 MB per frame.
- Handshake: Client sends `Event::Hello { version, hostname, display_size }` → server replies `Event::HelloAck { version, hostname, display_size }`.

## Wayland capture

- Requires GNOME 45+/KDE 6.1+ with InputCapture portal.
- Uses `ashpd::desktop::remote_desktop::RemoteDesktop` → EIS fd → `reis::ei::Context`.
- Capture loop always runs on `spawn_blocking` because reis types don't implement `Send`.
- Events flow: `reis` → `mpsc::blocking_send` → `InputCapture::next_event()` → server event loop.

## macOS

- `bm-input-macos` compiles as stubs on Linux. Full CGEvent emulation requires macOS + `--features "bm-input-macos/cg,bm-clipboard/sync"`.
- Client requires Accessibility permissions for `CGEventPost`.

## Entrypoints

- `src/main.rs` — CLI. No args → opens GUI. Subcommands: `gui` (alias `ui`), `server`, `client`, `init-config`, `config-path`.
- `crates/bm-gui/src/app.rs` — `BorderlessApp` implements `eframe::App`. Starts background tokio runtime on Start, stops via oneshot channel.

## State

- **Screen edge detection** (`InputEvent::EdgeReached` / `EdgeLeft`) is defined in the protocol but not yet wired in the wayland capture loop. `find_target()` in main.rs reads `ScreenEdge` config to decide forwarding targets.
- **No tests exist anywhere** in the repo.
- Config stored via `directories` at `~/.config/borderless-mouse/config.toml`.
