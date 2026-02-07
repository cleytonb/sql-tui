//! Query editor keyboard handlers

use crate::app::{App, InputMode};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Query Editor handler
    pub(crate) fn handle_query_editor(&mut self, key: KeyEvent) -> Result<()> {
        // Comandos que funcionam em ambos os modos
        match key.code {
            // Ctrl+E = executar query (Run)
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.start_query();
                return Ok(());
            }
            // Ctrl+F = Format SQL
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.format_sql();
                return Ok(());
            }
            // Ctrl+D = Smooth scroll down
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.pending_scroll += 10;
                return Ok(());
            }
            // Ctrl+U = Smooth scroll up
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.pending_scroll -= 10;
                return Ok(());
            }
            _ => {}
        }

        match self.input_mode {
            InputMode::Insert => self.handle_insert_mode(key),
            InputMode::Normal => self.handle_normal_mode(key),
            InputMode::Visual => self.handle_visual_mode(key),
            InputMode::Command => Ok(()), // Not implemented yet
        }
    }

    /// Handle Insert mode - normal typing
    fn handle_insert_mode(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Enter => {
                self.query.insert(self.cursor_pos, '\n');
                self.cursor_pos += 1;
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
                self.move_cursor_up();
            }
            KeyCode::Down => {
                self.move_cursor_down();
            }
            KeyCode::Home => {
                // Go to start of current line
                let text_before: String = self.query.chars().take(self.cursor_pos).collect();
                if let Some(last_newline) = text_before.rfind('\n') {
                    self.cursor_pos = last_newline + 1;
                } else {
                    self.cursor_pos = 0;
                }
            }
            KeyCode::End => {
                // Go to end of current line
                let text_after: String = self.query.chars().skip(self.cursor_pos).collect();
                if let Some(next_newline) = text_after.find('\n') {
                    self.cursor_pos += next_newline;
                } else {
                    self.cursor_pos = self.query.len();
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle Normal mode - vim commands
    fn handle_normal_mode(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            // Movement
            KeyCode::Char('h') | KeyCode::Left => {
                self.cursor_pos = self.cursor_pos.saturating_sub(1);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.cursor_pos = (self.cursor_pos + 1).min(self.query.len().saturating_sub(1));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_cursor_up();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_cursor_down();
            }
            KeyCode::Char('p') => {
                // Paste from system clipboard
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    if let Ok(text) = clipboard.get_text() {
                        for c in text.chars() {
                            self.query.insert(self.cursor_pos, c);
                            self.cursor_pos += 1;
                        }
                    }
                }
            }
            // Line start/end
            KeyCode::Char('0') | KeyCode::Home => {
                let text_before: String = self.query.chars().take(self.cursor_pos).collect();
                if let Some(last_newline) = text_before.rfind('\n') {
                    self.cursor_pos = last_newline + 1;
                } else {
                    self.cursor_pos = 0;
                }
            }
            KeyCode::Char('$') | KeyCode::End => {
                let text_after: String = self.query.chars().skip(self.cursor_pos).collect();
                if let Some(next_newline) = text_after.find('\n') {
                    self.cursor_pos += next_newline;
                } else {
                    self.cursor_pos = self.query.len();
                }
            }
            // First non-whitespace character
            KeyCode::Char('^') => {
                let text_before: String = self.query.chars().take(self.cursor_pos).collect();
                let line_start = if let Some(last_newline) = text_before.rfind('\n') {
                    last_newline + 1
                } else {
                    0
                };
                let line: String = self.query.chars().skip(line_start).collect();
                let first_non_white = line.find(|c: char| !c.is_whitespace() || c == '\n')
                    .unwrap_or(0);
                self.cursor_pos = line_start + first_non_white;
            }
            // Word forward
            KeyCode::Char('w') => {
                let chars: Vec<char> = self.query.chars().collect();
                let mut pos = self.cursor_pos;
                // Skip current word characters
                while pos < chars.len() && chars[pos].is_alphanumeric() {
                    pos += 1;
                }
                // Skip whitespace
                while pos < chars.len() && chars[pos].is_whitespace() && chars[pos] != '\n' {
                    pos += 1;
                }
                self.cursor_pos = pos.min(chars.len().saturating_sub(1));
            }
            // Word backward
            KeyCode::Char('b') => {
                let chars: Vec<char> = self.query.chars().collect();
                let mut pos = self.cursor_pos.saturating_sub(1);
                // Skip whitespace
                while pos > 0 && chars[pos].is_whitespace() {
                    pos -= 1;
                }
                // Skip word characters
                while pos > 0 && chars[pos - 1].is_alphanumeric() {
                    pos -= 1;
                }
                self.cursor_pos = pos;
            }
            // Document start/end
            KeyCode::Char('g') => {
                self.cursor_pos = 0;
            }
            KeyCode::Char('G') => {
                self.cursor_pos = self.query.len().saturating_sub(1);
            }
            // Delete character
            KeyCode::Char('x') => {
                if self.cursor_pos < self.query.len() {
                    self.query.remove(self.cursor_pos);
                    if self.cursor_pos >= self.query.len() && self.cursor_pos > 0 {
                        self.cursor_pos -= 1;
                    }
                }
            }
            // Delete line
            KeyCode::Char('d') => {
                let text_before: String = self.query.chars().take(self.cursor_pos).collect();
                let line_start = if let Some(last_newline) = text_before.rfind('\n') {
                    last_newline + 1
                } else {
                    0
                };
                let text_after: String = self.query.chars().skip(self.cursor_pos).collect();
                let line_end = if let Some(next_newline) = text_after.find('\n') {
                    self.cursor_pos + next_newline + 1
                } else {
                    self.query.len()
                };
                self.query.drain(line_start..line_end);
                self.cursor_pos = line_start.min(self.query.len().saturating_sub(1));
            }
            // Append (insert after cursor)
            KeyCode::Char('a') => {
                if self.cursor_pos < self.query.len() {
                    self.cursor_pos += 1;
                }
                self.input_mode = InputMode::Insert;
            }
            // Append at end of line
            KeyCode::Char('A') => {
                let text_after: String = self.query.chars().skip(self.cursor_pos).collect();
                if let Some(next_newline) = text_after.find('\n') {
                    self.cursor_pos += next_newline;
                } else {
                    self.cursor_pos = self.query.len();
                }
                self.input_mode = InputMode::Insert;
            }
            // Insert at start of line
            KeyCode::Char('I') => {
                let text_before: String = self.query.chars().take(self.cursor_pos).collect();
                if let Some(last_newline) = text_before.rfind('\n') {
                    self.cursor_pos = last_newline + 1;
                } else {
                    self.cursor_pos = 0;
                }
                self.input_mode = InputMode::Insert;
            }
            // New line below
            KeyCode::Char('o') => {
                let text_after: String = self.query.chars().skip(self.cursor_pos).collect();
                let line_end = if let Some(next_newline) = text_after.find('\n') {
                    self.cursor_pos + next_newline
                } else {
                    self.query.len()
                };
                self.query.insert(line_end, '\n');
                self.cursor_pos = line_end + 1;
                self.input_mode = InputMode::Insert;
            }
            // New line above
            KeyCode::Char('O') => {
                let text_before: String = self.query.chars().take(self.cursor_pos).collect();
                let line_start = if let Some(last_newline) = text_before.rfind('\n') {
                    last_newline + 1
                } else {
                    0
                };
                self.query.insert(line_start, '\n');
                self.cursor_pos = line_start;
                self.input_mode = InputMode::Insert;
            }
            // Change character
            KeyCode::Char('c') => {
                if self.cursor_pos < self.query.len() {
                    self.query.remove(self.cursor_pos);
                    if self.cursor_pos >= self.query.len() && self.cursor_pos > 0 {
                        self.cursor_pos -= 1;
                    }
                    self.input_mode = InputMode::Insert;
                }
            }
            // Insert mode
            KeyCode::Char('i') => {
                self.input_mode = InputMode::Insert;
            }
            // Visual mode
            KeyCode::Char('v') => {
                self.visual_anchor = self.cursor_pos;
                self.input_mode = InputMode::Visual;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle Visual mode - text selection
    fn handle_visual_mode(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            // Exit visual mode
            KeyCode::Esc | KeyCode::Char('v') => {
                self.input_mode = InputMode::Normal;
            }
            // Movement - expands/contracts selection
            KeyCode::Char('h') | KeyCode::Left => {
                self.cursor_pos = self.cursor_pos.saturating_sub(1);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.cursor_pos = (self.cursor_pos + 1).min(self.query.len().saturating_sub(1));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_cursor_up();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_cursor_down();
            }
            // Line start/end
            KeyCode::Char('0') | KeyCode::Home => {
                let text_before: String = self.query.chars().take(self.cursor_pos).collect();
                if let Some(last_newline) = text_before.rfind('\n') {
                    self.cursor_pos = last_newline + 1;
                } else {
                    self.cursor_pos = 0;
                }
            }
            KeyCode::Char('$') | KeyCode::End => {
                let text_after: String = self.query.chars().skip(self.cursor_pos).collect();
                if let Some(next_newline) = text_after.find('\n') {
                    self.cursor_pos += next_newline;
                } else {
                    self.cursor_pos = self.query.len().saturating_sub(1);
                }
            }
            // Word forward
            KeyCode::Char('w') => {
                let chars: Vec<char> = self.query.chars().collect();
                let mut pos = self.cursor_pos;
                while pos < chars.len() && chars[pos].is_alphanumeric() {
                    pos += 1;
                }
                while pos < chars.len() && chars[pos].is_whitespace() && chars[pos] != '\n' {
                    pos += 1;
                }
                self.cursor_pos = pos.min(chars.len().saturating_sub(1));
            }
            // Word backward
            KeyCode::Char('b') => {
                let chars: Vec<char> = self.query.chars().collect();
                let mut pos = self.cursor_pos.saturating_sub(1);
                while pos > 0 && chars[pos].is_whitespace() {
                    pos -= 1;
                }
                while pos > 0 && chars[pos - 1].is_alphanumeric() {
                    pos -= 1;
                }
                self.cursor_pos = pos;
            }
            // Document start/end
            KeyCode::Char('g') => {
                self.cursor_pos = 0;
            }
            KeyCode::Char('G') => {
                self.cursor_pos = self.query.len().saturating_sub(1);
            }
            // Yank (copy) selection
            KeyCode::Char('y') => {
                if let Some(text) = self.yank_selection() {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(&text);
                        self.message = Some(format!("Yanked {} chars", text.len()));
                    }
                }
            }
            // Delete selection
            KeyCode::Char('d') | KeyCode::Char('x') => {
                let text = self.get_selected_text();
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(&text);
                }
                self.delete_selection();
                self.message = Some(format!("Deleted {} chars", text.len()));
            }
            // Change (delete and enter insert mode)
            KeyCode::Char('c') => {
                let text = self.get_selected_text();
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(&text);
                }
                self.delete_selection();
                self.input_mode = InputMode::Insert;
            }
            // Select all (simulated ggVG)
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.visual_anchor = 0;
                self.cursor_pos = self.query.len().saturating_sub(1);
            }
            _ => {}
        }
        Ok(())
    }
}
