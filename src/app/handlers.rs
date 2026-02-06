//! Event handlers for the application - SIMPLIFIED VERSION

use crate::app::{App, ActivePanel, ResultsTab, SPINNER_FRAMES};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::prelude::*;
use std::time::Duration;

impl App {
    /// Main event loop
    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            // Check for query completion
            self.check_query_completion();

            // Advance spinner animation when loading
            if self.is_loading {
                self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
            }

            terminal.draw(|f| crate::ui::draw(f, self))?;

            // Use shorter poll time for smoother spinner animation
            let poll_duration = if self.is_loading {
                Duration::from_millis(80)
            } else {
                Duration::from_millis(100)
            };

            if event::poll(poll_duration)? {
                match event::read()? {
                    Event::Key(key) => {
                        self.handle_key(key)?;
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse(mouse)?;
                    }
                    _ => {}
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Handle keyboard input - SIMPLIFIED!
    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Don't process keys while loading (except quit)
        if self.is_loading {
            match (key.code, key.modifiers) {
                (KeyCode::Char('c'), KeyModifiers::CONTROL) |
                (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                    self.should_quit = true;
                }
                _ => {}
            }
            return Ok(());
        }

        // Clear messages on any keypress
        if key.code != KeyCode::Enter {
            self.message = None;
        }

        // Quit shortcuts - always work
        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) |
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return Ok(());
            }
            _ => {}
        }

        // Help toggle
        if key.code == KeyCode::F(1) {
            self.show_help = !self.show_help;
            return Ok(());
        }

        if self.show_help {
            if key.code == KeyCode::Esc {
                self.show_help = false;
            }
            return Ok(());
        }

        // Ctrl+Tab or Backtab to switch panels (Tab inserts indent in query editor)
        if key.code == KeyCode::Tab && key.modifiers.contains(KeyModifiers::CONTROL)
           || key.code == KeyCode::BackTab {
            self.active_panel = match self.active_panel {
                ActivePanel::QueryEditor => ActivePanel::Results,
                ActivePanel::Results => ActivePanel::SchemaExplorer,
                ActivePanel::SchemaExplorer => ActivePanel::History,
                ActivePanel::History => ActivePanel::QueryEditor,
            };
            return Ok(());
        }

        // Tab in non-query panels switches panels
        if key.code == KeyCode::Tab && self.active_panel != ActivePanel::QueryEditor {
            self.active_panel = match self.active_panel {
                ActivePanel::QueryEditor => ActivePanel::Results,
                ActivePanel::Results => ActivePanel::SchemaExplorer,
                ActivePanel::SchemaExplorer => ActivePanel::History,
                ActivePanel::History => ActivePanel::QueryEditor,
            };
            return Ok(());
        }

        // Handle based on active panel
        match self.active_panel {
            ActivePanel::QueryEditor => self.handle_query_editor(key)?,
            ActivePanel::Results => self.handle_results(key)?,
            ActivePanel::SchemaExplorer => self.handle_schema(key)?,
            ActivePanel::History => self.handle_history(key)?,
        }

        Ok(())
    }

    /// Handle mouse input (scroll events)
    fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        // Don't process mouse while loading
        if self.is_loading {
            return Ok(());
        }

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.scroll_up(3); // Scroll 3 lines at a time
            }
            MouseEventKind::ScrollDown => {
                self.scroll_down(3); // Scroll 3 lines at a time
            }
            _ => {}
        }

        Ok(())
    }

    /// Scroll up in the current panel
    fn scroll_up(&mut self, amount: usize) {
        match self.active_panel {
            ActivePanel::Results => {
                match self.results_tab {
                    ResultsTab::Data => {
                        self.results_selected = self.results_selected.saturating_sub(amount);
                    }
                    ResultsTab::Columns => {
                        // Columns tab shows columns vertically, so scroll vertically
                        self.results_selected = self.results_selected.saturating_sub(amount);
                    }
                    ResultsTab::Stats => {
                        // Stats view doesn't need scrolling (it's short)
                    }
                }
            }
            ActivePanel::SchemaExplorer => {
                self.schema_selected = self.schema_selected.saturating_sub(amount);
            }
            ActivePanel::History => {
                self.history_selected = self.history_selected.saturating_sub(amount);
            }
            ActivePanel::QueryEditor => {
                // Scroll query view
                self.query_scroll_y = self.query_scroll_y.saturating_sub(amount);
            }
        }
    }

    /// Scroll down in the current panel
    fn scroll_down(&mut self, amount: usize) {
        match self.active_panel {
            ActivePanel::Results => {
                match self.results_tab {
                    ResultsTab::Data => {
                        let max_rows = self.result.rows.len().saturating_sub(1);
                        self.results_selected = (self.results_selected + amount).min(max_rows);
                    }
                    ResultsTab::Columns => {
                        // Columns tab shows columns vertically, so scroll vertically
                        let max_cols = self.result.columns.len().saturating_sub(1);
                        self.results_selected = (self.results_selected + amount).min(max_cols);
                    }
                    ResultsTab::Stats => {
                        // Stats view doesn't need scrolling
                    }
                }
            }
            ActivePanel::SchemaExplorer => {
                let max = self.get_visible_schema_nodes().len().saturating_sub(1);
                self.schema_selected = (self.schema_selected + amount).min(max);
            }
            ActivePanel::History => {
                let max = self.history.len().saturating_sub(1);
                self.history_selected = (self.history_selected + amount).min(max);
            }
            ActivePanel::QueryEditor => {
                // Scroll query view
                let max_scroll = self.query.lines().count().saturating_sub(1);
                self.query_scroll_y = (self.query_scroll_y + amount).min(max_scroll);
            }
        }
    }

    /// Query Editor - Type and press Enter to run!
    fn handle_query_editor(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            // // ENTER = RUN QUERY!
            // KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
            //     // Shift+Enter = new line
            //     self.query.insert(self.cursor_pos, '\n');
            //     self.cursor_pos += 1;
            // }
            // KeyCode::Enter => {
            //     // Plain Enter = RUN QUERY!
            //     self.start_query();
            // }
            KeyCode::Enter => {
                // Shift+Enter = new line
                self.query.insert(self.cursor_pos, '\n');
                self.cursor_pos += 1;
            }
            // F5 also runs query
            KeyCode::F(5) => {
                self.start_query();
            }
            // Ctrl+F = Format SQL
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.format_sql();
            }
            // Tab = insert 4 spaces for indentation
            KeyCode::Tab => {
                let indent = "    "; // 4 spaces
                for c in indent.chars() {
                    self.query.insert(self.cursor_pos, c);
                    self.cursor_pos += 1;
                }
            }
            // Typing
            KeyCode::Char(c) => {
                self.query.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
            }
            // Backspace
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.query.remove(self.cursor_pos);
                }
            }
            // Delete
            KeyCode::Delete => {
                if self.cursor_pos < self.query.len() {
                    self.query.remove(self.cursor_pos);
                }
            }
            // Arrow keys for cursor movement
            KeyCode::Left => {
                self.cursor_pos = self.cursor_pos.saturating_sub(1);
            }
            KeyCode::Right => {
                self.cursor_pos = (self.cursor_pos + 1).min(self.query.len());
            }
            KeyCode::Up => {
                // Move cursor up one line
                self.move_cursor_up();
            }
            KeyCode::Down => {
                // Move cursor down one line
                self.move_cursor_down();
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
            }
            KeyCode::End => {
                self.cursor_pos = self.query.len();
            }
            // Esc clears query
            KeyCode::Esc => {
                self.query.clear();
                self.cursor_pos = 0;
            }
            _ => {}
        }
        Ok(())
    }

    /// Results panel navigation
    fn handle_results(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            // Tab switching with number keys 1, 2, 3
            KeyCode::Char('1') => {
                self.results_tab = ResultsTab::Data;
            }
            KeyCode::Char('2') => {
                self.results_tab = ResultsTab::Columns;
            }
            KeyCode::Char('3') => {
                self.results_tab = ResultsTab::Stats;
            }
            // Tab switching with Tab key
            KeyCode::Tab => {
                self.results_tab = match self.results_tab {
                    ResultsTab::Data => ResultsTab::Columns,
                    ResultsTab::Columns => ResultsTab::Stats,
                    ResultsTab::Stats => ResultsTab::Data,
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.results_selected = self.results_selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max_rows = match self.results_tab {
                    ResultsTab::Data => self.result.rows.len(),
                    ResultsTab::Columns => self.result.columns.len(),
                    ResultsTab::Stats => 10, // Fixed stats count
                };
                if self.results_selected < max_rows.saturating_sub(1) {
                    self.results_selected += 1;
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.results_col_selected = self.results_col_selected.saturating_sub(1);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let max_col = self.result.columns.len().saturating_sub(1);
                if self.results_col_selected < max_col {
                    self.results_col_selected += 1;
                }
            }
            KeyCode::PageUp => {
                self.results_selected = self.results_selected.saturating_sub(20);
            }
            KeyCode::PageDown => {
                let max_rows = match self.results_tab {
                    ResultsTab::Data => self.result.rows.len(),
                    ResultsTab::Columns => self.result.columns.len(),
                    ResultsTab::Stats => 10,
                };
                self.results_selected = (self.results_selected + 20)
                    .min(max_rows.saturating_sub(1));
            }
            KeyCode::Home => {
                self.results_selected = 0;
                self.results_col_selected = 0;
            }
            KeyCode::End => {
                let max_rows = match self.results_tab {
                    ResultsTab::Data => self.result.rows.len(),
                    ResultsTab::Columns => self.result.columns.len(),
                    ResultsTab::Stats => 10,
                };
                self.results_selected = max_rows.saturating_sub(1);
            }
            // Copy cell
            KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.copy_current_cell();
            }
            // Export CSV (Ctrl+E)
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.export_results_csv();
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.export_results_json();
            }
            // Copy row as INSERT statement
            KeyCode::Char('i') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.copy_row_as_insert();
            }
            // Enter/Esc goes back to query editor
            KeyCode::Enter | KeyCode::Esc => {
                self.active_panel = ActivePanel::QueryEditor;
            }
            _ => {}
        }
        Ok(())
    }

    /// Export results to CSV file
    fn export_results_csv(&mut self) {
        if self.result.rows.is_empty() {
            self.error = Some("No results to export".to_string());
            return;
        }

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("export_{}.csv", timestamp);

        match self.export_csv(&filename) {
            Ok(()) => {
                self.message = Some(format!("✓ Exported {} rows to {}", self.result.rows.len(), filename));
            }
            Err(e) => {
                self.error = Some(format!("Export failed: {}", e));
            }
        }
    }

    /// Export results to JSON file
    fn export_results_json(&mut self) {
        if self.result.rows.is_empty() {
            self.error = Some("No results to export".to_string());
            return;
        }

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("export_{}.json", timestamp);

        match self.export_json(&filename) {
            Ok(()) => {
                self.message = Some(format!("✓ Exported {} rows to {}", self.result.rows.len(), filename));
            }
            Err(e) => {
                self.error = Some(format!("Export failed: {}", e));
            }
        }
    }

    /// Copy current row as INSERT statement
    fn copy_row_as_insert(&mut self) {
        if self.result.rows.is_empty() || self.result.columns.is_empty() {
            return;
        }

        if let Some(row) = self.result.rows.get(self.results_selected) {
            let columns: Vec<String> = self.result.columns.iter()
                .map(|c| format!("[{}]", c.name))
                .collect();

            let values: Vec<String> = row.iter()
                .map(|cell| {
                    match cell {
                        crate::db::CellValue::Null => "NULL".to_string(),
                        crate::db::CellValue::String(s) => format!("'{}'", s.replace('\'', "''")),
                        crate::db::CellValue::DateTime(s) => format!("'{}'", s),
                        crate::db::CellValue::Int(n) => n.to_string(),
                        crate::db::CellValue::Float(n) => n.to_string(),
                        crate::db::CellValue::Bool(b) => if *b { "1" } else { "0" }.to_string(),
                        crate::db::CellValue::Binary(b) => format!("0x{}", b.iter().map(|x| format!("{:02X}", x)).collect::<String>()),
                    }
                })
                .collect();

            let insert = format!(
                "INSERT INTO [TableName] ({}) VALUES ({});",
                columns.join(", "),
                values.join(", ")
            );

            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(&insert);
                self.message = Some("✓ Copied INSERT statement to clipboard".to_string());
            }
        }
    }

    /// Schema explorer
    fn handle_schema(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up => {
                self.schema_selected = self.schema_selected.saturating_sub(1);
            }
            KeyCode::Down => {
                let max = self.get_visible_schema_nodes().len().saturating_sub(1);
                if self.schema_selected < max {
                    self.schema_selected += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let visible = self.get_visible_schema_nodes();
                if let Some((_, node)) = visible.get(self.schema_selected) {
                    if !node.children.is_empty() || node.node_type == crate::app::SchemaNodeType::Folder {
                        self.toggle_schema_node();
                    } else {
                        self.insert_schema_object();
                    }
                }
            }
            KeyCode::Esc => {
                self.active_panel = ActivePanel::QueryEditor;
            }
            _ => {}
        }
        Ok(())
    }

    /// History panel
    fn handle_history(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up => {
                self.history_selected = self.history_selected.saturating_sub(1);
            }
            KeyCode::Down => {
                let max = self.history.len().saturating_sub(1);
                if self.history_selected < max {
                    self.history_selected += 1;
                }
            }
            KeyCode::Enter => {
                self.load_history_entry();
            }
            KeyCode::Esc => {
                self.active_panel = ActivePanel::QueryEditor;
            }
            _ => {}
        }
        Ok(())
    }

    /// Move cursor up one line in query
    fn move_cursor_up(&mut self) {
        let text_before: String = self.query.chars().take(self.cursor_pos).collect();
        if let Some(last_newline) = text_before.rfind('\n') {
            let col = self.cursor_pos - last_newline - 1;
            let before_that: String = text_before.chars().take(last_newline).collect();
            if let Some(prev_newline) = before_that.rfind('\n') {
                let prev_line_len = last_newline - prev_newline - 1;
                self.cursor_pos = prev_newline + 1 + col.min(prev_line_len);
            } else {
                self.cursor_pos = col.min(last_newline);
            }
        }
    }

    /// Move cursor down one line in query
    fn move_cursor_down(&mut self) {
        let text_before: String = self.query.chars().take(self.cursor_pos).collect();
        let text_after: String = self.query.chars().skip(self.cursor_pos).collect();

        let col = if let Some(last_newline) = text_before.rfind('\n') {
            self.cursor_pos - last_newline - 1
        } else {
            self.cursor_pos
        };

        if let Some(next_newline) = text_after.find('\n') {
            let next_line_start = self.cursor_pos + next_newline + 1;
            let remaining: String = self.query.chars().skip(next_line_start).collect();
            let next_line_len = remaining.find('\n').unwrap_or(remaining.len());
            self.cursor_pos = next_line_start + col.min(next_line_len);
        }
    }

    fn copy_current_cell(&mut self) {
        if let Some(row) = self.result.rows.get(self.results_selected) {
            if let Some(cell) = row.get(self.results_col_selected) {
                let text = cell.to_string();
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(&text);
                    self.message = Some(format!("Copied: {}", text));
                }
            }
        }
    }

    fn export_csv(&self, filename: &str) -> Result<()> {
        let mut wtr = csv::Writer::from_path(filename)?;
        let headers: Vec<String> = self.result.columns.iter().map(|c| c.name.clone()).collect();
        wtr.write_record(&headers)?;
        for row in &self.result.rows {
            let record: Vec<String> = row.iter().map(|c| c.to_string()).collect();
            wtr.write_record(&record)?;
        }
        wtr.flush()?;
        Ok(())
    }

    fn export_json(&self, filename: &str) -> Result<()> {
        let mut rows: Vec<serde_json::Map<String, serde_json::Value>> = Vec::new();
        for row in &self.result.rows {
            let mut obj = serde_json::Map::new();
            for (i, col) in self.result.columns.iter().enumerate() {
                if let Some(cell) = row.get(i) {
                    obj.insert(col.name.clone(), serde_json::Value::String(cell.to_string()));
                }
            }
            rows.push(obj);
        }
        let json = serde_json::to_string_pretty(&rows)?;
        std::fs::write(filename, json)?;
        Ok(())
    }
}
