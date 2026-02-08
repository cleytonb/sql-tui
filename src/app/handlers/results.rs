//! Results panel keyboard handlers

use crate::app::{App, ActivePanel, ResultsTab};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rust_i18n::t;

impl App {
    /// Results panel navigation
    pub(crate) fn handle_results(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            // Tab switching with number keys 1, 2, 3
            KeyCode::Char('1') => {
                self.results_tab = ResultsTab::Data;
                self.results_scroll = 0;
                self.results_selected = 0;
            }
            KeyCode::Char('2') => {
                self.results_tab = ResultsTab::Columns;
                self.results_scroll = 0;
                self.results_selected = 0;
            }
            KeyCode::Char('3') => {
                self.results_tab = ResultsTab::Stats;
                self.results_scroll = 0;
                self.results_selected = 0;
            }
            // Tab switching with Tab key
            KeyCode::Tab => {
                self.results_tab = match self.results_tab {
                    ResultsTab::Data => ResultsTab::Columns,
                    ResultsTab::Columns => ResultsTab::Stats,
                    ResultsTab::Stats => ResultsTab::Data,
                };
                self.results_scroll = 0;
                self.results_selected = 0;
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.pending_scroll += 10;
                return Ok(());
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.pending_scroll -= 10;
                return Ok(());
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
                if self.results_col_selected > 0 {
                    self.results_col_selected -= 1;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let max_col = self.result.columns.len().saturating_sub(1);
                if self.results_col_selected < max_col {
                    self.results_col_selected += 1;
                }
            }
            KeyCode::Home => {
                self.results_selected = 0;
                self.results_col_selected = 0;
                self.results_col_scroll = 0;
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

    /// Copy current cell to clipboard
    pub(crate) fn copy_current_cell(&mut self) {
        if let Some(row) = self.result.rows.get(self.results_selected) {
            if let Some(cell) = row.get(self.results_col_selected) {
                let text = cell.to_string();
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(&text);
                    self.message = Some(t!("copied", text = text).to_string());
                }
            }
        }
    }

    /// Copy current row as INSERT statement
    pub(crate) fn copy_row_as_insert(&mut self) {
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
                self.message = Some(t!("copied_insert").to_string());
            }
        }
    }
}
