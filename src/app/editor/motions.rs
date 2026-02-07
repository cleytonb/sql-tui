//! Cursor motion functions for vim-like navigation
//!
//! This module contains functions for moving the cursor within text,
//! implementing vim-style motions like w, b, 0, $, g, G, etc.

/// Find the start position of the current line
pub fn line_start(text: &str, cursor_pos: usize) -> usize {
    let text_before: String = text.chars().take(cursor_pos).collect();
    if let Some(last_newline) = text_before.rfind('\n') {
        last_newline + 1
    } else {
        0
    }
}

/// Find the end position of the current line (before newline)
pub fn line_end(text: &str, cursor_pos: usize) -> usize {
    let text_after: String = text.chars().skip(cursor_pos).collect();
    if let Some(next_newline) = text_after.find('\n') {
        cursor_pos + next_newline
    } else {
        text.len()
    }
}

/// Find the first non-whitespace character on the current line
pub fn first_non_whitespace(text: &str, cursor_pos: usize) -> usize {
    let start = line_start(text, cursor_pos);
    let line: String = text.chars().skip(start).collect();
    let offset = line.find(|c: char| !c.is_whitespace() || c == '\n').unwrap_or(0);
    start + offset
}

/// Move forward by one word
pub fn word_forward(text: &str, cursor_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut pos = cursor_pos;
    
    // Skip current word characters
    while pos < chars.len() && chars[pos].is_alphanumeric() {
        pos += 1;
    }
    // Skip whitespace (but not newlines)
    while pos < chars.len() && chars[pos].is_whitespace() && chars[pos] != '\n' {
        pos += 1;
    }
    
    pos.min(chars.len().saturating_sub(1))
}

/// Move backward by one word
pub fn word_backward(text: &str, cursor_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut pos = cursor_pos.saturating_sub(1);
    
    // Skip whitespace
    while pos > 0 && chars[pos].is_whitespace() {
        pos -= 1;
    }
    // Skip word characters
    while pos > 0 && chars[pos - 1].is_alphanumeric() {
        pos -= 1;
    }
    
    pos
}

/// Move to the end of the current word
pub fn word_end(text: &str, cursor_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut pos = cursor_pos;
    
    // If on whitespace, skip it first
    while pos < chars.len() && chars[pos].is_whitespace() {
        pos += 1;
    }
    // Move to end of word
    while pos < chars.len() && chars[pos].is_alphanumeric() {
        pos += 1;
    }
    
    pos.saturating_sub(1).min(chars.len().saturating_sub(1))
}

/// Move to start of document
pub fn document_start() -> usize {
    0
}

/// Move to end of document
pub fn document_end(text: &str) -> usize {
    text.len().saturating_sub(1)
}

/// Calculate cursor position for moving up one line, preserving column
pub fn cursor_up(text: &str, cursor_pos: usize) -> usize {
    let text_before: String = text.chars().take(cursor_pos).collect();
    
    if let Some(last_newline) = text_before.rfind('\n') {
        let col = cursor_pos - last_newline - 1;
        let before_that: String = text_before.chars().take(last_newline).collect();
        
        if let Some(prev_newline) = before_that.rfind('\n') {
            let prev_line_len = last_newline - prev_newline - 1;
            prev_newline + 1 + col.min(prev_line_len)
        } else {
            col.min(last_newline)
        }
    } else {
        cursor_pos // Already on first line
    }
}

/// Calculate cursor position for moving down one line, preserving column
pub fn cursor_down(text: &str, cursor_pos: usize) -> usize {
    let text_before: String = text.chars().take(cursor_pos).collect();
    let text_after: String = text.chars().skip(cursor_pos).collect();

    let col = if let Some(last_newline) = text_before.rfind('\n') {
        cursor_pos - last_newline - 1
    } else {
        cursor_pos
    };

    if let Some(next_newline) = text_after.find('\n') {
        let next_line_start = cursor_pos + next_newline + 1;
        let remaining: String = text.chars().skip(next_line_start).collect();
        let next_line_len = remaining.find('\n').unwrap_or(remaining.len());
        next_line_start + col.min(next_line_len)
    } else {
        cursor_pos // Already on last line
    }
}

/// Find the next occurrence of a character on the current line (f motion)
pub fn find_char_forward(text: &str, cursor_pos: usize, target: char) -> Option<usize> {
    let line_end_pos = line_end(text, cursor_pos);
    let search_range: String = text.chars().skip(cursor_pos + 1).take(line_end_pos - cursor_pos - 1).collect();
    
    search_range.find(target).map(|offset| cursor_pos + 1 + offset)
}

/// Find the previous occurrence of a character on the current line (F motion)
pub fn find_char_backward(text: &str, cursor_pos: usize, target: char) -> Option<usize> {
    let line_start_pos = line_start(text, cursor_pos);
    let search_range: String = text.chars().skip(line_start_pos).take(cursor_pos - line_start_pos).collect();
    
    search_range.rfind(target).map(|offset| line_start_pos + offset)
}

/// Move to just before the next occurrence of a character (t motion)
pub fn till_char_forward(text: &str, cursor_pos: usize, target: char) -> Option<usize> {
    find_char_forward(text, cursor_pos, target).map(|pos| pos.saturating_sub(1))
}

/// Move to just after the previous occurrence of a character (T motion)
pub fn till_char_backward(text: &str, cursor_pos: usize, target: char) -> Option<usize> {
    find_char_backward(text, cursor_pos, target).map(|pos| pos + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_start() {
        let text = "hello\nworld";
        assert_eq!(line_start(text, 0), 0);
        assert_eq!(line_start(text, 3), 0);
        assert_eq!(line_start(text, 6), 6);
        assert_eq!(line_start(text, 8), 6);
    }

    #[test]
    fn test_line_end() {
        let text = "hello\nworld";
        assert_eq!(line_end(text, 0), 5);
        assert_eq!(line_end(text, 6), 11);
    }

    #[test]
    fn test_word_forward() {
        let text = "hello world test";
        assert_eq!(word_forward(text, 0), 6);
        assert_eq!(word_forward(text, 6), 12);
    }

    #[test]
    fn test_word_backward() {
        let text = "hello world test";
        assert_eq!(word_backward(text, 16), 12);
        assert_eq!(word_backward(text, 12), 6);
        assert_eq!(word_backward(text, 6), 0);
    }
}
