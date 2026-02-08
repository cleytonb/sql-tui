//! Query editor keyboard handlers

use crate::app::{App, InputMode};
use crate::completion::{extract_context, get_candidates, get_candidates_with_columns};
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
        // Handle completion navigation first if visible
        if self.completion.visible {
            match key.code {
                // Navigate completion up
                KeyCode::Up => {
                    self.completion.select_prev();
                    return Ok(());
                }
                KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.completion.select_prev();
                    return Ok(());
                }
                // Navigate completion down
                KeyCode::Down => {
                    self.completion.select_next();
                    return Ok(());
                }
                KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.completion.select_next();
                    return Ok(());
                }
                // Accept completion with Enter or Tab
                KeyCode::Enter | KeyCode::Tab => {
                    self.accept_completion();
                    return Ok(());
                }
                // Close completion with Escape
                KeyCode::Esc => {
                    self.completion.hide();
                    return Ok(());
                }
                // Continue typing - will update completion
                _ => {}
            }
        }

        match key.code {
            KeyCode::Enter => {
                self.completion.hide();
                self.save_undo_state();
                self.query.insert(self.cursor_pos, '\n');
                self.cursor_pos += 1;
            }
            // Tab = trigger completion OR insert 4 spaces
            KeyCode::Tab => {
                if self.completion.visible {
                    self.accept_completion();
                } else {
                    self.trigger_completion();
                }
            }
            // Ctrl+Space = force trigger completion
            KeyCode::Char(' ') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.trigger_completion();
            }
            // Escape = close completion or go to normal mode
            KeyCode::Esc => {
                if self.completion.visible {
                    self.completion.hide();
                } else {
                    self.input_mode = InputMode::Normal;
                }
            }
            // Typing "." triggers completion automatically
            KeyCode::Char('.') => {
                self.save_undo_state();
                self.query.insert(self.cursor_pos, '.');
                self.cursor_pos += 1;
                // Trigger completion after schema.
                self.trigger_completion();
            }
            // Regular typing
            KeyCode::Char(c) => {
                self.save_undo_state();
                self.query.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
                
                // After typing space, check if we should auto-trigger completion
                if c == ' ' {
                    self.maybe_trigger_after_keyword();
                } else if self.completion.visible {
                    // Update completion if visible
                    self.update_completion();
                }
            }
            // Backspace
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.save_undo_state();
                    self.cursor_pos -= 1;
                    self.query.remove(self.cursor_pos);
                    // Update or hide completion
                    if self.completion.visible {
                        self.update_completion();
                    }
                }
            }
            // Delete
            KeyCode::Delete => {
                if self.cursor_pos < self.query.len() {
                    self.save_undo_state();
                    self.query.remove(self.cursor_pos);
                }
            }
            // Arrow keys for cursor movement
            KeyCode::Left => {
                self.completion.hide();
                self.cursor_pos = self.cursor_pos.saturating_sub(1);
            }
            KeyCode::Right => {
                self.completion.hide();
                self.cursor_pos = (self.cursor_pos + 1).min(self.query.len());
            }
            KeyCode::Up => {
                self.completion.hide();
                self.move_cursor_up();
            }
            KeyCode::Down => {
                self.completion.hide();
                self.move_cursor_down();
            }
            KeyCode::Home => {
                self.completion.hide();
                // Go to start of current line
                let text_before: String = self.query.chars().take(self.cursor_pos).collect();
                if let Some(last_newline) = text_before.rfind('\n') {
                    self.cursor_pos = last_newline + 1;
                } else {
                    self.cursor_pos = 0;
                }
            }
            KeyCode::End => {
                self.completion.hide();
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

    /// Trigger autocomplete at current cursor position
    fn trigger_completion(&mut self) {
        let context = extract_context(&self.query, self.cursor_pos);
        
        // Get prefix for filtering (text after last separator)
        let prefix = self.get_completion_prefix();
        
        // Try to get column cache (non-blocking)
        let candidates = if let Ok(cache) = self.column_cache.try_read() {
            get_candidates_with_columns(&context, &self.schema_tree, &prefix, &cache)
        } else {
            // Cache is locked, use version without columns
            get_candidates(&context, &self.schema_tree, &prefix)
        };
        
        if candidates.is_empty() {
            self.completion.hide();
        } else {
            self.completion.show(candidates, self.cursor_pos, prefix);
        }
    }

    /// Update completion while typing
    fn update_completion(&mut self) {
        let prefix = self.get_completion_prefix();
        
        // Check if we're right after a dot (e.g., "pmt.")
        let after_dot = self.cursor_pos > 0 && 
            self.query.chars().nth(self.cursor_pos - 1) == Some('.');
        
        // If prefix is empty but we're after a dot, re-trigger full completion
        if prefix.is_empty() {
            if after_dot {
                // Re-trigger completion for schema. context
                self.trigger_completion();
            } else {
                self.completion.hide();
            }
            return;
        }
        
        // Always re-calculate context and get fresh candidates
        // This ensures we keep the right context (e.g., AfterWhere with tables)
        let context = extract_context(&self.query, self.cursor_pos);
        
        // Try to get column cache (non-blocking)
        let candidates = if let Ok(cache) = self.column_cache.try_read() {
            get_candidates_with_columns(&context, &self.schema_tree, &prefix, &cache)
        } else {
            get_candidates(&context, &self.schema_tree, &prefix)
        };
        
        if candidates.is_empty() {
            self.completion.hide();
        } else {
            self.completion.show(candidates, self.cursor_pos - prefix.len(), prefix);
        }
    }

    /// Check if we should auto-trigger completion after typing a space
    /// (e.g., after WHERE, AND, OR, SELECT, FROM)
    fn maybe_trigger_after_keyword(&mut self) {
        let before_cursor = &self.query[..self.cursor_pos];
        let upper = before_cursor.to_uppercase();
        
        // Keywords that should trigger completion when followed by space
        let trigger_keywords = [
            "WHERE ", "AND ", "OR ", "SELECT ", "FROM ", "JOIN ",
            "INNER JOIN ", "LEFT JOIN ", "RIGHT JOIN ", "FULL JOIN ",
            "ORDER BY ", "GROUP BY ", "HAVING ", "ON ", "SET ",
        ];
        
        for keyword in trigger_keywords {
            if upper.ends_with(keyword) {
                self.trigger_completion();
                return;
            }
        }
    }

    /// Get the prefix being typed for completion (word at cursor)
    fn get_completion_prefix(&self) -> String {
        let before_cursor = &self.query[..self.cursor_pos];
        let chars: Vec<char> = before_cursor.chars().collect();
        
        if chars.is_empty() {
            return String::new();
        }
        
        let mut start = chars.len();
        
        // Walk backwards to find word start
        for i in (0..chars.len()).rev() {
            let c = chars[i];
            if c.is_alphanumeric() || c == '_' {
                start = i;
            } else {
                break;
            }
        }
        
        chars[start..].iter().collect()
    }

    /// Accept the currently selected completion
    fn accept_completion(&mut self) {
        if let Some(item) = self.completion.get_selected().cloned() {
            self.save_undo_state();
            
            // Remove the prefix that was already typed
            let prefix_len = self.completion.prefix.len();
            for _ in 0..prefix_len {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.query.remove(self.cursor_pos);
                }
            }
            
            // Insert the completion text
            for c in item.insert_text.chars() {
                self.query.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
            }
            
            self.completion.hide();
        }
    }

    /// Handle Normal mode - vim commands
    fn handle_normal_mode(&mut self, key: KeyEvent) -> Result<()> {
        // Handle pending character search (f/F/t/T waiting for char)
        if let Some(pending) = self.pending_char_search {
            self.pending_char_search = None;
            if let KeyCode::Char(ch) = key.code {
                match pending {
                    'f' => { self.find_char_forward(ch, false); }
                    'F' => { self.find_char_backward(ch, false); }
                    't' => { self.find_char_forward(ch, true); }
                    'T' => { self.find_char_backward(ch, true); }
                    _ => {}
                }
            }
            return Ok(());
        }

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
                        self.save_undo_state();
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
            // Find character forward (f)
            KeyCode::Char('f') => {
                self.pending_char_search = Some('f');
            }
            // Find character backward (F)
            KeyCode::Char('F') => {
                self.pending_char_search = Some('F');
            }
            // Till character forward (t)
            KeyCode::Char('t') => {
                self.pending_char_search = Some('t');
            }
            // Till character backward (T)
            KeyCode::Char('T') => {
                self.pending_char_search = Some('T');
            }
            // Repeat last f/F/t/T search (;)
            KeyCode::Char(';') => {
                self.repeat_char_search();
            }
            // Repeat last f/F/t/T search in opposite direction (,)
            KeyCode::Char(',') => {
                self.repeat_char_search_opposite();
            }
            // Document start/end
            KeyCode::Char('g') => {
                self.cursor_pos = 0;
            }
            KeyCode::Char('G') => {
                self.cursor_pos = self.query.len().saturating_sub(1);
            }
            // Undo
            KeyCode::Char('u') => {
                self.undo();
            }
            // Redo (Ctrl+R)
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.redo();
            }
            // Delete character
            KeyCode::Char('x') => {
                if self.cursor_pos < self.query.len() {
                    self.save_undo_state();
                    self.query.remove(self.cursor_pos);
                    if self.cursor_pos >= self.query.len() && self.cursor_pos > 0 {
                        self.cursor_pos -= 1;
                    }
                }
            }
            // Delete line
            KeyCode::Char('d') => {
                self.save_undo_state();
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
                self.save_undo_state();
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
                self.save_undo_state();
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
                    self.save_undo_state();
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
        // Handle pending character search (f/F/t/T waiting for char)
        if let Some(pending) = self.pending_char_search {
            self.pending_char_search = None;
            if let KeyCode::Char(ch) = key.code {
                match pending {
                    'f' => { self.find_char_forward(ch, false); }
                    'F' => { self.find_char_backward(ch, false); }
                    't' => { self.find_char_forward(ch, true); }
                    'T' => { self.find_char_backward(ch, true); }
                    _ => {}
                }
            }
            return Ok(());
        }

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
            // Find character forward (f)
            KeyCode::Char('f') => {
                self.pending_char_search = Some('f');
            }
            // Find character backward (F)
            KeyCode::Char('F') => {
                self.pending_char_search = Some('F');
            }
            // Till character forward (t)
            KeyCode::Char('t') => {
                self.pending_char_search = Some('t');
            }
            // Till character backward (T)
            KeyCode::Char('T') => {
                self.pending_char_search = Some('T');
            }
            // Repeat last f/F/t/T search (;)
            KeyCode::Char(';') => {
                self.repeat_char_search();
            }
            // Repeat last f/F/t/T search in opposite direction (,)
            KeyCode::Char(',') => {
                self.repeat_char_search_opposite();
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
                self.save_undo_state();
                let text = self.get_selected_text();
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(&text);
                }
                self.delete_selection();
                self.message = Some(format!("Deleted {} chars", text.len()));
            }
            // Change (delete and enter insert mode)
            KeyCode::Char('c') => {
                self.save_undo_state();
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
