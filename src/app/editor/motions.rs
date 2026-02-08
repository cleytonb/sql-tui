//! Cursor motion functions for vim-like navigation
//!
//! All positions (cursor_pos, return values) are **char indices**, not byte indices.
//! This module contains functions for moving the cursor within text,
//! implementing vim-style motions like w, b, 0, $, g, G, etc.

/// Find the start position (char index) of the current line
pub fn line_start(text: &str, cursor_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    // Walk backwards from cursor_pos to find '\n'
    for i in (0..cursor_pos).rev() {
        if chars[i] == '\n' {
            return i + 1;
        }
    }
    0
}

/// Find the end position (char index) of the current line (before newline)
pub fn line_end(text: &str, cursor_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    for i in cursor_pos..chars.len() {
        if chars[i] == '\n' {
            return i;
        }
    }
    chars.len()
}

/// Find the first non-whitespace character on the current line
pub fn first_non_whitespace(text: &str, cursor_pos: usize) -> usize {
    let start = line_start(text, cursor_pos);
    let chars: Vec<char> = text.chars().collect();
    for i in start..chars.len() {
        if !chars[i].is_whitespace() || chars[i] == '\n' {
            return i;
        }
    }
    start
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
    text.chars().count().saturating_sub(1)
}

/// Calculate cursor position for moving up one line, preserving column
pub fn cursor_up(text: &str, cursor_pos: usize) -> usize {
    let line_start_pos = line_start(text, cursor_pos);

    if line_start_pos == 0 {
        return cursor_pos; // Already on first line
    }

    let col = cursor_pos - line_start_pos;
    // Previous line ends at line_start_pos - 1 (the '\n')
    let prev_line_end = line_start_pos - 1;
    let prev_line_start = line_start(text, prev_line_end);
    let prev_line_len = prev_line_end - prev_line_start;

    prev_line_start + col.min(prev_line_len)
}

/// Calculate cursor position for moving down one line, preserving column
pub fn cursor_down(text: &str, cursor_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let line_start_pos = line_start(text, cursor_pos);
    let col = cursor_pos - line_start_pos;
    let line_end_pos = line_end(text, cursor_pos);

    if line_end_pos >= chars.len() {
        return cursor_pos; // Already on last line
    }

    let next_line_start = line_end_pos + 1;
    let next_line_end = line_end(text, next_line_start);
    let next_line_len = next_line_end - next_line_start;

    next_line_start + col.min(next_line_len)
}

/// Find the next occurrence of a character on the current line (f motion)
pub fn find_char_forward(text: &str, cursor_pos: usize, target: char) -> Option<usize> {
    let chars: Vec<char> = text.chars().collect();
    let end = line_end(text, cursor_pos);
    for i in (cursor_pos + 1)..end {
        if chars[i] == target {
            return Some(i);
        }
    }
    None
}

/// Find the previous occurrence of a character on the current line (F motion)
pub fn find_char_backward(text: &str, cursor_pos: usize, target: char) -> Option<usize> {
    let chars: Vec<char> = text.chars().collect();
    let start = line_start(text, cursor_pos);
    for i in (start..cursor_pos).rev() {
        if chars[i] == target {
            return Some(i);
        }
    }
    None
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

    #[test]
    fn test_line_start_with_utf8() {
        let text = "café\nwörld";
        // "café" = 4 chars, then \n at index 4, "wörld" starts at 5
        assert_eq!(line_start(text, 0), 0);
        assert_eq!(line_start(text, 3), 0);
        assert_eq!(line_start(text, 5), 5);
        assert_eq!(line_start(text, 7), 5);
    }

    #[test]
    fn test_line_end_with_utf8() {
        let text = "café\nwörld";
        assert_eq!(line_end(text, 0), 4);
        assert_eq!(line_end(text, 5), 10);
    }
}
