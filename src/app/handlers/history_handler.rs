//! History panel keyboard handlers

use crate::app::{App, ActivePanel};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    /// History panel handler
    pub(crate) fn handle_history(&mut self, key: KeyEvent) -> Result<()> {
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
}
