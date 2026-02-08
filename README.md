# sql-tui

A high-performance SQL Server TUI client built with Rust. Navigate databases, write queries with vim keybindings, and explore results — all from your terminal.

![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

## Features

- **Vim-style editor** — Normal, Insert, Visual, and Command modes with motions (`w`, `b`, `e`, `f`, `t`), text objects (`iw`, `i"`, `a(`), operators (`d`, `c`, `y`), and undo/redo
- **SQL autocomplete** — Context-aware suggestions for keywords, tables, columns, schemas, and stored procedures
- **Schema explorer** — Browse tables, views, and procedures organized by schema
- **Results table** — Scrollable with Data, Columns, and Stats tabs
- **Query history** — Persistent across sessions with timestamps
- **Export** — CSV, JSON, and INSERT statements
- **SQL syntax highlighting** — Keywords, strings, numbers, comments
- **Connection manager** — Save and switch between multiple SQL Server connections
- **i18n** — English and Portuguese (pt-BR), auto-detected from system locale
- **Mouse support** — Scroll in all panels

## Install

```bash
git clone https://github.com/cleytonb/sql-tui.git
cd sql-tui
cargo build --release
```

The binary will be at `target/release/sql-tui`.

## Keyboard Shortcuts

### Global

| Key | Action |
|-----|--------|
| `Ctrl+Q` | Quit |
| `F1` | Help |
| `Tab` | Next panel |
| `<Space>q` | Query editor panel |
| `<Space>r` | Query results panel |
| `<Space>s` | Schema explorer panel |
| `<Space>h` | History panel |
| `<Space>c` | Connection manager |

### Query Editor — Normal Mode

| Key | Action |
|-----|--------|
| `i` / `a` / `o` / `O` | Enter insert mode |
| `I` / `A` | Insert at line start / end |
| `h` `j` `k` `l` | Cursor movement |
| `w` / `b` / `e` | Word forward / backward / end |
| `0` / `$` | Line start / end |
| `^` | First non-whitespace |
| `gg` / `G` | Document start / end |
| `f` / `F` / `t` / `T` | Find / till character |
| `x` | Delete character |
| `d` | Delete line |
| `c` | Change character |
| `v` | Visual mode |
| `u` / `Ctrl+R` | Undo / Redo |
| `y` | Yank |
| `p` | Paste |

### Query Editor — Insert Mode

| Key | Action |
|-----|--------|
| `Ctrl+E` | Execute query |
| `Tab` | Accept completion / Indent |
| `Esc` | Back to normal mode |

### Results Panel

| Key | Action |
|-----|--------|
| `1` / `2` / `3` | Data / Columns / Stats tab |
| `j` / `k` | Navigate rows |
| `h` / `l` | Navigate columns |
| `Ctrl+U` / `Ctrl+D` | Smooth scroll |
| `Home` / `End` | First / Last row |
| `Ctrl+Y` | Copy cell value |
| `Ctrl+E` | Export CSV |
| `Ctrl+S` | Export JSON |
| `Ctrl+I` | Copy row as INSERT |

## Architecture

Event-driven async state machine using **ratatui** + **crossterm** for the TUI and **tiberius** for SQL Server connectivity over TDS.

```
src/
├── main.rs                    # Entry point, terminal setup
├── app/
│   ├── state.rs               # Central App state (single source of truth)
│   ├── actions.rs             # Async business logic, query execution
│   ├── handlers/              # Event handlers per panel
│   │   ├── query_editor.rs    # Vim modes + key handling
│   │   ├── results.rs         # Results navigation + export
│   │   ├── schema.rs          # Schema tree navigation
│   │   ├── history_handler.rs # History panel
│   │   └── connection.rs      # Connection modal
│   ├── editor/                # Vim engine
│   │   ├── motions.rs         # w, b, e, f, t, gg, G, etc.
│   │   ├── operations.rs      # d, c, y operators
│   │   └── text_objects.rs    # iw, i", a(, etc.
│   └── undo.rs                # Undo/redo stack
├── completion/                # SQL autocomplete engine
│   ├── context.rs             # Cursor context parsing
│   └── candidates.rs          # Suggestion generation
├── db/                        # Database layer (tiberius)
│   ├── connection.rs          # Arc<Mutex<Client>> wrapper
│   ├── query.rs               # Query execution, type mapping
│   └── schema.rs              # Schema/table/proc loading
├── ui/
│   ├── theme.rs               # Color scheme
│   ├── layout.rs              # Panel layout
│   └── widgets/               # One widget per panel
└── config.rs                  # Connection config persistence
```

## Tech Stack

- [ratatui](https://github.com/ratatui/ratatui) — Terminal UI framework
- [tiberius](https://github.com/prisma/tiberius) — SQL Server TDS driver
- [tokio](https://tokio.rs) — Async runtime
- [crossterm](https://github.com/crossterm-rs/crossterm) — Terminal backend
- [rust-i18n](https://github.com/longbridge/rust-i18n) — Internationalization

## Acknowledgments

Originally based on [alrajhi-sql-tui](https://github.com/hszkf/alrajhi-sql-tui) by [@hszkf](https://github.com/hszkf).

## License

[MIT](LICENSE)
