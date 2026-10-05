# CH. Clicker

A native auto clicker for Windows 10/11 by Chenoli Labs. The core is written in Rust (input injection through `SendInput` and a high-precision scheduler) and the interface is React on Tauri 2.

- **Patterns:** single point, multiple points, wiggle and area scan
- **Profiles:** global hotkeys, import/export (`.chprofile`), tray menu and an always-on-top HUD
- **Themes and languages:** four themes; English, Brazilian Portuguese and Latin American Spanish
- **Safety:** an emergency stop that can never be disabled, and the app never clicks its own window

## Installation

Download `CH Clicker_*_x64-setup.exe` from [Releases](https://github.com/chenoli/chenoli-auto-clicker/releases). It installs for your user only, without admin rights, and the app keeps itself up to date. The installer asks for your language and the app starts in it.

> **Not code-signed yet.** Windows shows "Unknown publisher": click **More info → Run anyway**. With **Smart App Control** turned on, Windows 11 blocks the app until it is signed. We are applying for free open-source signing; see [`docs/release.md`](docs/release.md).

> **Online games with anti-cheat** detect automated input and may suspend or ban your account. CH. Clicker does not hide its activity. Do not use it where the terms of service forbid automation.

## Development

### Requirements

- Windows 10 1803+ or Windows 11
- [Rust](https://rustup.rs) stable (MSVC) and Visual Studio Build Tools with the "Desktop development with C++" workload
- Node.js 24 and pnpm 12
- WebView2 (preinstalled on Windows 11)

Windows 11 **Smart App Control** blocks the executables produced by local builds, so it must be off for development.

### Commands

| Command                                                                  | What it does                                                                    |
| ------------------------------------------------------------------------ | ------------------------------------------------------------------------------- |
| `pnpm install`                                                           | Installs the frontend dependencies                                              |
| `pnpm tauri dev`                                                         | Runs the app in development mode                                                |
| `pnpm dev`                                                               | Frontend only, in the browser (gallery at `http://localhost:1420/gallery.html`) |
| `pnpm test`                                                              | Frontend tests with coverage (80% minimum)                                      |
| `pnpm test:visual`                                                       | Visual parity with Playwright                                                   |
| `pnpm lint` / `pnpm lint:fix`                                            | Strict ESLint (zero warnings)                                                   |
| `pnpm format`                                                            | Prettier                                                                        |
| `pnpm typecheck`                                                         | TypeScript                                                                      |
| `pnpm tokens`                                                            | Regenerates the themes from `design/tokens.json`                                |
| `cargo test --workspace`                                                 | Rust tests (also regenerates `ui/src/ipc/bindings.ts`)                          |
| `cargo clippy --workspace --all-targets -- -D warnings`                  | Rust lint                                                                       |
| `cargo llvm-cov --workspace --exclude ch-clicker --exclude jitter-bench` | Rust coverage (80% minimum)                                                     |
| `cargo test -p ch-win --test injection -- --ignored`                     | Tests that inject real clicks (only with games and sensitive apps closed)       |
| `cargo run -p jitter-bench --release -- --seconds 10`                    | Scheduler jitter benchmark (report in `docs/bench/`)                            |

### Layout

```
crates/ch-core     pure domain (no Windows): models, patterns, scheduler
crates/ch-win      Windows layer: input, timers, DPI, hotkeys, capture
crates/ch-engine   engine runtime
crates/ch-storage  persistence in %APPDATA%
tools/jitter-bench scheduler timing benchmark
src-tauri/         Tauri app: commands, events, windows
ui/                React interface (one page per window)
design/            design tokens (source of truth) and icon
tools/             token generator and local lint rules
```

More detail:

- Architecture: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- Phased backlog: [`docs/TASKS.md`](docs/TASKS.md)
- Releasing: [`docs/release.md`](docs/release.md)

## Contributing

Issues and pull requests are welcome. Read [`CONTRIBUTING.md`](CONTRIBUTING.md) first. To report a vulnerability, follow [`SECURITY.md`](SECURITY.md) instead of opening a public issue. This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).

## License

[MIT](LICENSE) © 2026 Chenoli Labs. The Syne and Space Grotesk fonts are distributed under the SIL Open Font License 1.1.
