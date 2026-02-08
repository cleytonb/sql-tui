# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build                # Dev build
cargo build --release      # Optimized release build (LTO enabled, stripped)
cargo test                 # Run all tests
cargo test <test_name>     # Run a single test
cargo clippy               # Lint (no custom config, uses defaults)
cargo fmt                  # Format (no custom config, uses defaults)
```

## Architecture

**SQL Server TUI client** built with Rust — a terminal-based SQL Studio using ratatui + crossterm for the UI and tiberius for SQL Server connectivity over TDS protocol.

### Event Loop & State Machine

The app follows an **event-driven async state machine** pattern:

1. `main.rs` sets up the terminal (raw mode, alternate screen, mouse capture) and runs the tokio async runtime
2. `app/handlers/mod.rs` runs the main event loop — polls crossterm events at 100ms (10ms during animations)
3. Events dispatch to panel-specific handlers in `app/handlers/` based on `app.active_panel`
4. Handlers mutate `App` state directly (`app/state.rs` is the central state struct)
5. `ui/mod.rs` renders the current state each frame

### Key Modules

- **`app/state.rs`** — The `App` struct holds ALL application state (editor, results, schema, connection, UI flags). This is the single source of truth.
- **`app/actions.rs`** — Async business logic: query execution, schema loading, connection management. Queries run in background tasks via tokio, results arrive through `oneshot` channels.
- **`app/handlers/`** — One handler per panel: `query_editor.rs`, `results.rs`, `schema.rs`, `history_handler.rs`, `connection.rs`
- **`app/editor/`** — Vim-like text editor: `motions.rs` (cursor movement), `operations.rs` (delete/yank/change), `text_objects.rs` (word/quote selections)
- **`db/`** — Database layer wrapping tiberius. `DbConnection` holds `Arc<Mutex<Client>>` for thread-safe async access. `query.rs` handles type conversion (CellValue enum). `schema.rs` loads tables/views/procs.
- **`ui/widgets/`** — Each panel is a widget: query_editor, results_table, schema_tree, history_list, completion_popup, connection_modal
- **`completion/`** — Context-aware SQL autocomplete: parses cursor context to suggest keywords, tables, columns, or procedures

### Vim Modes

The query editor implements Normal, Insert, Visual, and Command modes (`InputMode` enum). Insert mode handles character input and autoclose. Normal mode supports motions, operators, and text objects.

### Database Connection

- Connections saved in `~/.config/sqltui/config.json`
- `.env` file for default connection (DB_HOST, DB_PORT, DB_USER, DB_PASSWORD, DB_DATABASE)
- `DbConnection` wraps tiberius `Client<Compat<TcpStream>>` in `Arc<Mutex<>>`

### i18n

Uses `rust-i18n` with locale files in `locales/`. Supports English and Portuguese (Brazil). System locale auto-detected via `sys-locale`.

## Conventions

- 4-space indentation in Rust source
- Handler functions follow `handle_{panel_name}(&mut self, key: KeyEvent) -> Result<()>` pattern
- Undo state saved before any text mutation: call `self.save_undo_state()` before modifying `self.query`
- UI theme uses Alrajhi Bank brand colors defined in `ui/theme.rs`
