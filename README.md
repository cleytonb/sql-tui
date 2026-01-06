# Alrajhi Bank SQL Studio

High-performance SQL Server Terminal UI built with Rust for Alrajhi Bank.

## Features

- **Beautiful TUI** - Professional corporate design with Alrajhi Bank branding
- **Fast & Efficient** - Built in Rust for maximum performance
- **SQL Syntax Highlighting** - Color-coded SQL keywords, strings, and numbers
- **Schema Explorer** - Browse tables, views, and stored procedures
- **Query History** - Persistent history with timestamps
- **Results Table** - Scrollable with row numbers, type indicators, NULL highlighting
- **Tabbed Results** - View Data, Columns info, and Query Stats
- **Export** - CSV, JSON, and INSERT statement export
- **Mouse Support** - Scroll with mouse wheel in all panels

## Quick Install

### One-Liner (Recommended)

```bash
git clone https://github.com/hszkf/alrajhi-sql-tui.git && cd alrajhi-sql-tui && ./setup.sh
```

This will:
1. Clone the repository
2. Build the release binary
3. Prompt for database credentials
4. Install `atui` command

### After Installation

```bash
atui              # Run from anywhere
atui update       # Update to latest version
atui config       # Change database credentials
```

### Manual Install

```bash
# 1. Clone
git clone https://github.com/hszkf/alrajhi-sql-tui.git
cd alrajhi-sql-tui

# 2. Build
cargo build --release

# 3. Configure (create .env file)
cp .env.example .env
nano .env  # Edit with your credentials

# 4. Run
source .env && ./target/release/alrajhi_sql_tui
```

### Update

```bash
atui update
# Or manually:
cd ~/alrajhi-sql-tui && git pull && cargo build --release
```

## Keyboard Shortcuts

### Global
| Key | Action |
|-----|--------|
| `Ctrl+Q` | Quit application |
| `F1` | Toggle help popup |
| `Ctrl+Tab` | Next panel |
| `Shift+Tab` | Previous panel |

### Query Editor
| Key | Action |
|-----|--------|
| `Enter` | Execute query |
| `Shift+Enter` | New line |
| `Tab` | Insert 4 spaces (indent) |
| `Ctrl+F` | Format SQL |
| `F5` | Execute query |
| `Esc` | Clear query |
| Arrow keys | Move cursor |

### Results Panel
| Key | Action |
|-----|--------|
| `1` / `2` / `3` | Switch to Data/Columns/Stats tab |
| `Tab` | Cycle through tabs |
| `j/k` or `Up/Down` | Navigate rows |
| `h/l` or `Left/Right` | Navigate columns |
| `PageUp/PageDown` | Fast scroll (20 rows) |
| `Home/End` | First/Last row |
| `Ctrl+Y` | Copy cell value |
| `Ctrl+E` | Export to CSV |
| `Ctrl+S` | Export to JSON |
| `Ctrl+I` | Copy row as INSERT |
| Mouse scroll | Scroll through results |

### Schema Explorer
| Key | Action |
|-----|--------|
| `Up/Down` | Navigate |
| `Enter/Space` | Expand folder / Insert table name |
| Mouse scroll | Scroll through schema |

### History Panel
| Key | Action |
|-----|--------|
| `Up/Down` | Navigate |
| `Enter` | Load query |
| Mouse scroll | Scroll through history |

## Configuration

Environment variables (set in `.env` file):

| Variable | Default | Description |
|----------|---------|-------------|
| `DB_HOST` | localhost | SQL Server hostname or IP |
| `DB_PORT` | 1433 | SQL Server port |
| `DB_USER` | sa | Database username |
| `DB_PASSWORD` | (empty) | Database password |
| `DB_DATABASE` | master | Default database |

Example `.env` file:
```bash
DB_HOST=10.200.224.42
DB_PORT=1433
DB_USER=your_username
DB_PASSWORD=your_password
DB_DATABASE=Staging
```

## Project Structure

```
src/
├── main.rs           # Entry point
├── app/              # Application state & handlers
│   ├── mod.rs
│   ├── state.rs      # App state
│   ├── handlers.rs   # Keyboard & mouse handlers
│   └── history.rs    # Query history
├── db/               # Database layer
│   ├── mod.rs
│   ├── connection.rs # SQL Server connection
│   ├── query.rs      # Query execution with DATE handling
│   └── schema.rs     # Schema explorer
└── ui/               # User interface
    ├── mod.rs
    ├── theme.rs      # Alrajhi Bank colors
    ├── layout.rs     # Panel layout
    └── widgets.rs    # UI components with tabs
```

## Technologies

- **ratatui 0.26** - Terminal UI framework
- **tiberius** - SQL Server driver (TDS protocol)
- **tokio** - Async runtime
- **crossterm 0.27** - Cross-platform terminal handling
- **arboard** - Clipboard support

## Troubleshooting

### Build Errors
```bash
# Update Rust toolchain
rustup update

# Clean and rebuild
cargo clean
cargo build --release
```

### Connection Issues
- Verify SQL Server is accessible on port 1433
- Check firewall settings
- Run `atui config` to update credentials

---
Built with Rust for Alrajhi Bank IT Team
