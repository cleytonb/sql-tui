//! History panel keyboard handlers

use crate::app::{App, ActivePanel};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// History panel handler
    pub(crate) fn handle_history(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.history_selected = self.history_selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.history.len().saturating_sub(1);
                if self.history_selected < max {
                    self.history_selected += 1;
                }
            }
            // Smooth scroll: Ctrl+D (down) / Ctrl+U (up)
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.pending_scroll += 10;
                return Ok(());
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.pending_scroll -= 10;
                return Ok(());
            }
            KeyCode::Enter => {
                self.load_history_entry();
            }
            KeyCode::Esc => {
                self.active_panel = ActivePanel::QueryEditor;
            }
            // Clear history with Ctrl+L
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.history.clear();
                self.history_selected = 0;
                self.history_scroll_offset = 0;
                self.message = Some("Histórico limpo".to_string());
            }
            _ => {}
        }
        Ok(())
    }
}
