# opencore_rustroops

[![CI](https://github.com/bengidev/opencore_rustroops/actions/workflows/ci.yml/badge.svg)](https://github.com/bengidev/opencore_rustroops/actions/workflows/ci.yml)

A desktop application that integrates frontier AI capabilities to help users complete tasks on their machine. The app combines three workspaces—editor, chat, and terminal—so you can edit content, converse with AI, and run commands in one environment.

## Background

Modern AI tools are powerful but fragmented: you often switch between an editor, a chat window, and a terminal to get work done. **opencore_rustroops** brings these together in a single desktop app, using AI to assist across editing, conversation, and command execution.

## Features

The application is organized around three main modes:

| Mode | Purpose |
|------|---------|
| **Editor** | Code and content editing with AI-assisted workflows |
| **Chat** | Conversational AI for questions, planning, and task guidance |
| **Terminal** | Command execution and shell interaction alongside AI support |

Together, these modes let you move between writing, asking, and doing without leaving the app.

## Tech Stack

- **Language:** Rust (edition 2024)
- **Desktop UI:** [GPUI](https://github.com/zed-industries/zed) with [gpui-component](https://github.com/longbridge/gpui-component)
- **Build:** Cargo

The project boots a native GPUI desktop window: an onboarding flow (galaxy orb, theme toggle) routes into a three-zone shell (Editor, Chat, Terminal). On macOS, the window is centered on screen at launch and after onboarding completes.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain with Cargo)

## Setup

Clone the repository and build from the project root:

```bash
git clone https://github.com/bengidev/opencore_rustroops.git
cd opencore_rustroops
cargo build
```

## Usage

Run the application:

```bash
cargo run
```

Run tests:

```bash
cargo test
```

## CI

GitHub Actions runs on every push to `master`/`main` and on pull requests:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all-targets`

Jobs run on **Ubuntu**, **macOS**, and **Windows** (Linux installs GPUI system dependencies via `apt`).

## Project Structure

```
opencore_rustroops/
├── Cargo.toml
├── src/
│   ├── main.rs           # Entry point
│   ├── app/              # GPUI desktop app (onboarding, shell, state)
│   └── shared/           # Preferences, theme tokens
└── README.md
```

## License

MIT — see [LICENSE](LICENSE).
