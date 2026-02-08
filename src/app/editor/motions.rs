//! Cursor motion functions for vim-like navigation
//!
//! All positions (cursor_pos, return values) are **char indices**, not byte indices.
//! This module contains functions for moving the cursor within text,
//! implementing vim-style motions like w, b, e, 0, $, g, G, etc.

/// Character class for vim word motions.
/// In vim, a "word" is a sequence of word chars (alphanumeric + underscore),
/// a sequence of punctuation (non-word, non-whitespace), or whitespace.
#[derive(PartialEq, Eq)]
enum CharClass {
    Word,
    Punct,
    Whitespace,
}

fn char_class(c: char) -> CharClass {
    if c.is_whitespace() {
        CharClass::Whitespace
    } else if c.is_alphanumeric() || c == '_' {
        CharClass::Word
    } else {
        CharClass::Punct
    }
}

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

/// Move forward by one word (w motion).
/// Jumps to the start of the next word. Words are sequences of word chars
/// (alphanumeric + underscore) or sequences of punctuation.
pub fn word_forward(text: &str, cursor_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return 0;
    }
    let mut pos = cursor_pos;
    let cls = char_class(chars[pos]);

    // Skip current word/punct class
    if cls != CharClass::Whitespace {
        while pos < chars.len() && char_class(chars[pos]) == cls {
            pos += 1;
        }
    }
    // Skip whitespace
    while pos < chars.len() && char_class(chars[pos]) == CharClass::Whitespace {
        pos += 1;
    }

    pos.min(chars.len().saturating_sub(1))
}

/// Move backward by one word (b motion).
pub fn word_backward(text: &str, cursor_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return 0;
    }
    let mut pos = cursor_pos.saturating_sub(1);

    // Skip whitespace
    while pos > 0 && char_class(chars[pos]) == CharClass::Whitespace {
        pos -= 1;
    }
    // Skip current word/punct class
    let cls = char_class(chars[pos]);
    while pos > 0 && char_class(chars[pos - 1]) == cls {
        pos -= 1;
    }

    pos
}

/// Move to the end of the current/next word (e motion).
pub fn word_end(text: &str, cursor_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return 0;
    }
    let mut pos = cursor_pos + 1;
    if pos >= chars.len() {
        return chars.len().saturating_sub(1);
    }

    // Skip whitespace
    while pos < chars.len() && char_class(chars[pos]) == CharClass::Whitespace {
        pos += 1;
    }
    // Skip current word/punct class
    if pos < chars.len() {
        let cls = char_class(chars[pos]);
        while pos + 1 < chars.len() && char_class(chars[pos + 1]) == cls {
            pos += 1;
        }
    }

    pos.min(chars.len().saturating_sub(1))
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
    fn test_word_forward_with_punctuation() {
        // "SELECT * FROM pmt.Contas"
        //  0123456789...
        let text = "SELECT * FROM pmt.Contas";
        // S(0) -> w -> *(7)
        assert_eq!(word_forward(text, 0), 7);
        // *(7) -> w -> F(9)
        assert_eq!(word_forward(text, 7), 9);
        // F(9) -> w -> p(14)
        assert_eq!(word_forward(text, 9), 14);
        // p(14) -> w -> .(17) (punct is its own word)
        assert_eq!(word_forward(text, 14), 17);
        // .(17) -> w -> C(18)
        assert_eq!(word_forward(text, 17), 18);
    }

    #[test]
    fn test_word_end_with_punctuation() {
        let text = "SELECT * FROM pmt.Contas";
        // S(0) -> e -> T(5)
        assert_eq!(word_end(text, 0), 5);
        // T(5) -> e -> *(7)
        assert_eq!(word_end(text, 5), 7);
        // *(7) -> e -> M(12)
        assert_eq!(word_end(text, 7), 12);
        // M(12) -> e -> t(16)
        assert_eq!(word_end(text, 12), 16);
        // t(16) -> e -> .(17)
        assert_eq!(word_end(text, 16), 17);
        // .(17) -> e -> s(23)
        assert_eq!(word_end(text, 17), 23);
    }

    #[test]
    fn test_word_backward_with_punctuation() {
        let text = "SELECT * FROM pmt.Contas";
        // s(23) -> b -> C(18)
        assert_eq!(word_backward(text, 23), 18);
        // C(18) -> b -> .(17)
        assert_eq!(word_backward(text, 18), 17);
        // .(17) -> b -> p(14)
        assert_eq!(word_backward(text, 17), 14);
        // p(14) -> b -> F(9)
        assert_eq!(word_backward(text, 14), 9);
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
