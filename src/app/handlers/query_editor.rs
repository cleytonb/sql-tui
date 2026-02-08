//! Query editor keyboard handlers

use crate::app::{App, InputMode};
use crate::completion::{extract_context, get_candidates, get_candidates_with_columns};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rust_i18n::t;

impl App {
    /// Get the char index of the start of the current line
    fn current_line_start_char(&self) -> usize {
        let chars: Vec<char> = self.query.chars().collect();
        for i in (0..self.cursor_pos).rev() {
            if chars[i] == '\n' {
                return i + 1;
            }
        }
        0
    }

    /// Get the indentation (leading whitespace) of the current line
    fn get_current_line_indent(&self) -> String {
        let line_start = self.current_line_start_char();

        // Extract leading whitespace from current line
        let mut indent = String::new();
        for c in self.query.chars().skip(line_start) {
            if c == ' ' || c == '\t' {
                indent.push(c);
            } else {
                break;
            }
        }
        indent
    }

    /// Get the current line text (before cursor, from line start)
    fn get_current_line_text(&self) -> String {
        let line_start = self.current_line_start_char();
        self.query.chars().skip(line_start).take(self.cursor_pos - line_start).collect()
    }

    /// Insert newline with auto-indentation and BEGIN/END autoclose
    fn insert_newline_with_autoclose(&mut self) {
        let indent = self.get_current_line_indent();
        let current_line = self.get_current_line_text();
        let trimmed_line = current_line.trim_end();

        // Check if line ends with BEGIN (case-insensitive)
        let ends_with_begin = trimmed_line
            .to_uppercase()
            .ends_with("BEGIN");

        if ends_with_begin {
            // Insert: \n<indent>    <cursor>\n<indent>END
            let inner_indent = format!("{}    ", indent);

            // Two newlines + inner indent (cursor line)
            self.query.insert(self.query_byte_pos(), '\n');
            self.cursor_pos += 1;
            self.query.insert(self.query_byte_pos(), '\n');
            self.cursor_pos += 1;
            for c in inner_indent.chars() {
                self.query.insert(self.query_byte_pos(), c);
                self.cursor_pos += 1;
            }

            // Save cursor position (this is where the user will type)
            let cursor_final = self.cursor_pos;

            // Two newlines + original indent + END
            self.query.insert(self.query_byte_pos(), '\n');
            self.cursor_pos += 1;
            self.query.insert(self.query_byte_pos(), '\n');
            self.cursor_pos += 1;
            for c in indent.chars() {
                self.query.insert(self.query_byte_pos(), c);
                self.cursor_pos += 1;
            }
            for c in "END".chars() {
                self.query.insert(self.query_byte_pos(), c);
                self.cursor_pos += 1;
            }

            // Move cursor back to the inner line
            self.cursor_pos = cursor_final;
        } else {
            // Normal newline with auto-indent
            self.query.insert(self.query_byte_pos(), '\n');
            self.cursor_pos += 1;
            for c in indent.chars() {
                self.query.insert(self.query_byte_pos(), c);
                self.cursor_pos += 1;
            }
        }
    }

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
            // Enter or Ctrl+J (Shift+Enter in iTerm2 sends Ctrl+J)
            KeyCode::Enter => {
                self.completion.hide();
                self.save_undo_state();
                self.insert_newline_with_autoclose();
            }
            // Ctrl+J = Line Feed (Shift+Enter in some terminals like iTerm2)
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.completion.hide();
                self.save_undo_state();
                self.insert_newline_with_autoclose();
            }
            // Tab = accept completion OR insert 4 spaces
            KeyCode::Tab => {
                if self.completion.visible {
                    self.accept_completion();
                } else {
                    // Insert 4 spaces for indentation
                    self.save_undo_state();
                    for _ in 0..4 {
                        self.query.insert(self.query_byte_pos(), ' ');
                        self.cursor_pos += 1;
                    }
                }
            }
            // BackTab = remove 4 spaces
            KeyCode::BackTab => {
                if !self.completion.visible {
                    for _ in 0..4 {
                        if self.cursor_pos > 0 {
                            self.cursor_pos -= 1;
                            self.query.remove(self.query_byte_pos());
                        }
                    }
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
                self.query.insert(self.query_byte_pos(), '.');
                self.cursor_pos += 1;
                self.trigger_completion();
            }
            // Typing "@" triggers variable completion
            KeyCode::Char('@') => {
                self.save_undo_state();
                self.query.insert(self.query_byte_pos(), '@');
                self.cursor_pos += 1;
                self.trigger_completion();
            }
            // Regular typing
            KeyCode::Char(c) => {
                self.save_undo_state();

                // Autoclose: single quotes
                if c == '\'' {
                    // If next char is already a closing quote, just skip over it
                    let next_char = self.query.chars().nth(self.cursor_pos);
                    if next_char == Some('\'') {
                        self.cursor_pos += 1;
                    } else {
                        let byte_pos = self.query_byte_pos();
                        self.query.insert(byte_pos, '\'');
                        self.query.insert(byte_pos + 1, '\'');
                        self.cursor_pos += 1;
                    }
                } else {
                    self.query.insert(self.query_byte_pos(), c);
                    self.cursor_pos += 1;
                }

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
                    self.query.remove(self.query_byte_pos());
                    // Update or hide completion
                    if self.completion.visible {
                        self.update_completion();
                    }
                }
            }
            // Delete
            KeyCode::Delete => {
                if self.cursor_pos < self.query.chars().count() {
                    self.save_undo_state();
                    self.query.remove(self.query_byte_pos());
                }
            }
            // Arrow keys for cursor movement
            KeyCode::Left => {
                self.completion.hide();
                self.cursor_pos = self.cursor_pos.saturating_sub(1);
            }
            KeyCode::Right => {
                self.completion.hide();
                self.cursor_pos = (self.cursor_pos + 1).min(self.query.chars().count());
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
                self.cursor_pos = self.current_line_start_char();
            }
            KeyCode::End => {
                self.completion.hide();
                // Go to end of current line
                let chars: Vec<char> = self.query.chars().collect();
                let mut end = self.cursor_pos;
                while end < chars.len() && chars[end] != '\n' {
                    end += 1;
                }
                self.cursor_pos = end;
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
        
        // Check if we're right after a dot (e.g., "pmt.") or @ (e.g., "@")
        let last_char = if self.cursor_pos > 0 {
            self.query.chars().nth(self.cursor_pos - 1)
        } else {
            None
        };
        let after_dot = last_char == Some('.');
        let after_at = last_char == Some('@');

        // If prefix is empty but we're after a dot or @, re-trigger full completion
        if prefix.is_empty() {
            if after_dot || after_at {
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
        let byte_pos = self.query_byte_pos();
        let before_cursor = &self.query[..byte_pos];
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
        let byte_pos = self.query_byte_pos();
        let before_cursor = &self.query[..byte_pos];
        let chars: Vec<char> = before_cursor.chars().collect();
        
        if chars.is_empty() {
            return String::new();
        }
        
        let mut start = chars.len();
        
        // Walk backwards to find word start (include @ for variable names)
        for i in (0..chars.len()).rev() {
            let c = chars[i];
            if c.is_alphanumeric() || c == '_' || c == '@' {
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
            
            // Remove the prefix that was already typed (prefix.len() is char count since it was built from chars)
            let prefix_char_len = self.completion.prefix.chars().count();
            for _ in 0..prefix_char_len {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.query.remove(self.query_byte_pos());
                }
            }

            // Insert the completion text
            for c in item.insert_text.chars() {
                self.query.insert(self.query_byte_pos(), c);
                self.cursor_pos += 1;
            }
            
            self.completion.hide();
        }
    }

    /// Handle g prefix motions (gg, g_, ge, etc.)
    fn handle_g_motion(&mut self, ch: char) {
        match ch {
            // gg = go to start of document
            'g' => {
                self.cursor_pos = 0;
            }
            // g_ = go to last non-whitespace character of current line
            '_' => {
                let text_after: String = self.query.chars().skip(self.cursor_pos).collect();
                let line_end = if let Some(next_newline) = text_after.find('\n') {
                    self.cursor_pos + text_after[..next_newline].chars().count()
                } else {
                    self.query.chars().count()
                };
                // Walk backwards from line end to find last non-whitespace
                let chars: Vec<char> = self.query.chars().collect();
                let mut target = line_end.saturating_sub(1);
                while target > 0 && (chars.get(target) == Some(&' ') || chars.get(target) == Some(&'\t') || chars.get(target) == Some(&'\n')) {
                    target = target.saturating_sub(1);
                }
                self.cursor_pos = target;
            }
            // ge = go to end of previous word
            'e' => {
                if self.cursor_pos > 0 {
                    let chars: Vec<char> = self.query.chars().collect();
                    let mut pos = self.cursor_pos.saturating_sub(1);
                    // Skip whitespace backwards
                    while pos > 0 && chars[pos].is_whitespace() {
                        pos -= 1;
                    }
                    // Skip word chars backwards
                    while pos > 0 && !chars[pos - 1].is_whitespace() {
                        pos -= 1;
                    }
                    self.cursor_pos = pos;
                }
            }
            _ => {}
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

        // Handle pending g prefix (gg, g_, ge, etc.)
        if self.pending_g {
            self.pending_g = false;
            if let KeyCode::Char(ch) = key.code {
                self.handle_g_motion(ch);
            }
            return Ok(());
        }

        match key.code {
            // Movement
            KeyCode::Char('h') | KeyCode::Left => {
                self.cursor_pos = self.cursor_pos.saturating_sub(1);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.cursor_pos = (self.cursor_pos + 1).min(self.query.chars().count().saturating_sub(1));
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
                            self.query.insert(self.query_byte_pos(), c);
                            self.cursor_pos += 1;
                        }
                    }
                }
            }
            // Line start/end
            KeyCode::Char('0') | KeyCode::Home => {
                self.cursor_pos = self.current_line_start_char();
            }
            KeyCode::Char('$') | KeyCode::End => {
                let chars: Vec<char> = self.query.chars().collect();
                let mut end = self.cursor_pos;
                while end < chars.len() && chars[end] != '\n' {
                    end += 1;
                }
                self.cursor_pos = end;
            }
            // First non-whitespace character
            KeyCode::Char('^') => {
                let line_start = self.current_line_start_char();
                let chars: Vec<char> = self.query.chars().collect();
                let mut pos = line_start;
                while pos < chars.len() && chars[pos] != '\n' && (chars[pos] == ' ' || chars[pos] == '\t') {
                    pos += 1;
                }
                self.cursor_pos = pos;
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
            // Word end forward (e)
            KeyCode::Char('e') => {
                let chars: Vec<char> = self.query.chars().collect();
                let mut pos = self.cursor_pos + 1;
                // Skip whitespace
                while pos < chars.len() && chars[pos].is_whitespace() {
                    pos += 1;
                }
                // Move to end of word
                while pos < chars.len() && chars[pos].is_alphanumeric() {
                    pos += 1;
                }
                self.cursor_pos = pos.saturating_sub(1).min(chars.len().saturating_sub(1));
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
            // g prefix (gg, g_, ge, etc.)
            KeyCode::Char('g') => {
                self.pending_g = true;
            }
            // G = go to end of document
            KeyCode::Char('G') => {
                self.cursor_pos = self.query.chars().count().saturating_sub(1);
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
                let char_count = self.query.chars().count();
                if self.cursor_pos < char_count {
                    self.save_undo_state();
                    self.query.remove(self.query_byte_pos());
                    let new_char_count = self.query.chars().count();
                    if self.cursor_pos >= new_char_count && self.cursor_pos > 0 {
                        self.cursor_pos -= 1;
                    }
                }
            }
            // Delete line
            KeyCode::Char('d') => {
                self.save_undo_state();
                let text_before: String = self.query.chars().take(self.cursor_pos).collect();
                let line_start_char = if let Some(last_newline) = text_before.rfind('\n') {
                    // last_newline is a byte index in text_before; convert to char index
                    text_before[..last_newline].chars().count() + 1
                } else {
                    0
                };
                let text_after: String = self.query.chars().skip(self.cursor_pos).collect();
                let line_end_char = if let Some(next_newline) = text_after.find('\n') {
                    // next_newline is byte index in text_after; convert to char count
                    self.cursor_pos + text_after[..next_newline].chars().count() + 1
                } else {
                    self.query.chars().count()
                };
                let byte_start = Self::char_to_byte_index(&self.query, line_start_char);
                let byte_end = Self::char_to_byte_index(&self.query, line_end_char);
                self.query.drain(byte_start..byte_end);
                self.cursor_pos = line_start_char.min(self.query.chars().count().saturating_sub(1));
            }
            // Append (insert after cursor)
            KeyCode::Char('a') => {
                if self.cursor_pos < self.query.chars().count() {
                    self.cursor_pos += 1;
                }
                self.input_mode = InputMode::Insert;
            }
            // Append at end of line
            KeyCode::Char('A') => {
                let chars: Vec<char> = self.query.chars().collect();
                let mut end = self.cursor_pos;
                while end < chars.len() && chars[end] != '\n' {
                    end += 1;
                }
                self.cursor_pos = end;
                self.input_mode = InputMode::Insert;
            }
            // Insert at start of line
            KeyCode::Char('I') => {
                self.cursor_pos = self.current_line_start_char();
                self.input_mode = InputMode::Insert;
            }
            // New line below
            KeyCode::Char('o') => {
                self.save_undo_state();
                let indent = self.get_current_line_indent();
                let text_after: String = self.query.chars().skip(self.cursor_pos).collect();
                let line_end_char = if let Some(next_newline) = text_after.find('\n') {
                    self.cursor_pos + text_after[..next_newline].chars().count()
                } else {
                    self.query.chars().count()
                };
                let byte_pos = Self::char_to_byte_index(&self.query, line_end_char);
                self.query.insert(byte_pos, '\n');
                self.cursor_pos = line_end_char + 1;
                // Insert the same indentation on the new line
                for c in indent.chars() {
                    self.query.insert(self.query_byte_pos(), c);
                    self.cursor_pos += 1;
                }
                self.input_mode = InputMode::Insert;
            }
            // New line above
            KeyCode::Char('O') => {
                self.save_undo_state();
                let indent = self.get_current_line_indent();
                let text_before: String = self.query.chars().take(self.cursor_pos).collect();
                let line_start_char = if let Some(last_newline) = text_before.rfind('\n') {
                    text_before[..last_newline].chars().count() + 1
                } else {
                    0
                };
                // Build the string to insert: indent + newline
                let mut insert_str = indent.clone();
                insert_str.push('\n');
                let byte_start = Self::char_to_byte_index(&self.query, line_start_char);
                self.query.insert_str(byte_start, &insert_str);
                self.cursor_pos = line_start_char + indent.chars().count();
                self.input_mode = InputMode::Insert;
            }
            // Change character
            KeyCode::Char('c') => {
                let char_count = self.query.chars().count();
                if self.cursor_pos < char_count {
                    self.save_undo_state();
                    self.query.remove(self.query_byte_pos());
                    let new_char_count = self.query.chars().count();
                    if self.cursor_pos >= new_char_count && self.cursor_pos > 0 {
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

        // Handle pending g prefix
        if self.pending_g {
            self.pending_g = false;
            if let KeyCode::Char(ch) = key.code {
                self.handle_g_motion(ch);
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
                self.cursor_pos = (self.cursor_pos + 1).min(self.query.chars().count().saturating_sub(1));
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
                    self.cursor_pos = text_before[..last_newline].chars().count() + 1;
                } else {
                    self.cursor_pos = 0;
                }
            }
            KeyCode::Char('$') | KeyCode::End => {
                let text_after: String = self.query.chars().skip(self.cursor_pos).collect();
                if let Some(next_newline) = text_after.find('\n') {
                    self.cursor_pos += text_after[..next_newline].chars().count();
                } else {
                    self.cursor_pos = self.query.chars().count().saturating_sub(1);
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
            // Word end forward (e)
            KeyCode::Char('e') => {
                let chars: Vec<char> = self.query.chars().collect();
                let mut pos = self.cursor_pos + 1;
                while pos < chars.len() && chars[pos].is_whitespace() {
                    pos += 1;
                }
                while pos < chars.len() && chars[pos].is_alphanumeric() {
                    pos += 1;
                }
                self.cursor_pos = pos.saturating_sub(1).min(chars.len().saturating_sub(1));
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
            // g prefix (gg, g_, ge, etc.)
            KeyCode::Char('g') => {
                self.pending_g = true;
            }
            // G = go to end of document
            KeyCode::Char('G') => {
                self.cursor_pos = self.query.chars().count().saturating_sub(1);
            }
            // Yank (copy) selection
            KeyCode::Char('y') => {
                if let Some(text) = self.yank_selection() {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(&text);
                        self.message = Some(t!("yanked_chars", count = text.len()).to_string());
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
                self.message = Some(t!("deleted_chars", count = text.len()).to_string());
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
                self.cursor_pos = self.query.chars().count().saturating_sub(1);
            }
            _ => {}
        }
        Ok(())
    }
}
