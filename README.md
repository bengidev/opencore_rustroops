# opencore_rustroops

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
- **Build:** Cargo

The project is in early development. The core application shell and the three-mode UI are planned; the current codebase is a minimal Rust scaffold.

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

## Project Structure

```
opencore_rustroops/
├── Cargo.toml      # Package manifest and dependencies
├── src/
│   └── main.rs     # Application entry point
└── README.md
```

## License

MIT — see [LICENSE](LICENSE).
