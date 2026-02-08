//! Text editing operations for vim-like functionality
//!
//! This module contains functions for modifying text,
//! implementing vim-style operations like delete, yank, change, etc.

use super::motions;
use crate::app::App;

/// Helper to convert char index to byte index
fn c2b(s: &str, char_idx: usize) -> usize {
    App::char_to_byte_index(s, char_idx)
}

/// Delete a line from the text, returning the deleted content
/// cursor_pos is a char index
pub fn delete_line(text: &mut String, cursor_pos: usize) -> (String, usize) {
    let line_start = motions::line_start(text, cursor_pos);
    let mut line_end = motions::line_end(text, cursor_pos);

    // Include the newline if present
    let char_count = text.chars().count();
    if line_end < char_count {
        line_end += 1;
    }

    let deleted: String = text.chars().skip(line_start).take(line_end - line_start).collect();
    let byte_start = c2b(text, line_start);
    let byte_end = c2b(text, line_end);
    text.drain(byte_start..byte_end);

    let new_cursor = line_start.min(text.chars().count().saturating_sub(1));
    (deleted, new_cursor)
}

/// Delete a character at the given position (char index)
pub fn delete_char(text: &mut String, cursor_pos: usize) -> Option<char> {
    if cursor_pos < text.chars().count() {
        Some(text.remove(c2b(text, cursor_pos)))
    } else {
        None
    }
}

/// Delete a range of text (char indices), returning the deleted content
pub fn delete_range(text: &mut String, start: usize, end: usize) -> String {
    let char_count = text.chars().count();
    let end_inclusive = (end + 1).min(char_count);
    let deleted: String = text.chars().skip(start).take(end_inclusive - start).collect();
    let byte_start = c2b(text, start);
    let byte_end = c2b(text, end_inclusive);
    text.drain(byte_start..byte_end);
    deleted
}

/// Yank (copy) a range of text (char indices)
pub fn yank_range(text: &str, start: usize, end: usize) -> String {
    let char_count = text.chars().count();
    let end_inclusive = (end + 1).min(char_count);
    text.chars().skip(start).take(end_inclusive - start).collect()
}

/// Yank the current line (cursor_pos is char index)
pub fn yank_line(text: &str, cursor_pos: usize) -> String {
    let line_start = motions::line_start(text, cursor_pos);
    let mut line_end = motions::line_end(text, cursor_pos);

    // Include the newline if present
    let char_count = text.chars().count();
    if line_end < char_count {
        line_end += 1;
    }

    text.chars().skip(line_start).take(line_end - line_start).collect()
}

/// Insert text at the given position (char index)
pub fn insert_text(text: &mut String, pos: usize, content: &str) -> usize {
    let byte_pos = c2b(text, pos);
    text.insert_str(byte_pos, content);
    pos + content.chars().count()
}

/// Replace a character at the given position (char index)
pub fn replace_char(text: &mut String, cursor_pos: usize, new_char: char) {
    if cursor_pos < text.chars().count() {
        let byte_pos = c2b(text, cursor_pos);
        text.remove(byte_pos);
        text.insert(byte_pos, new_char);
    }
}

/// Join the current line with the next line (cursor_pos is char index)
pub fn join_lines(text: &mut String, cursor_pos: usize) -> usize {
    let line_end = motions::line_end(text, cursor_pos);

    let char_count = text.chars().count();
    if line_end < char_count {
        // Remove the newline
        let byte_pos = c2b(text, line_end);
        text.remove(byte_pos);
        // Insert a space if needed
        if line_end < text.chars().count() && !text.chars().nth(line_end).unwrap().is_whitespace() {
            text.insert(c2b(text, line_end), ' ');
        }
    }

    cursor_pos
}

/// Delete from cursor to end of line (D command) (char indices)
pub fn delete_to_line_end(text: &mut String, cursor_pos: usize) -> String {
    let line_end = motions::line_end(text, cursor_pos);
    let deleted: String = text.chars().skip(cursor_pos).take(line_end - cursor_pos).collect();
    let byte_start = c2b(text, cursor_pos);
    let byte_end = c2b(text, line_end);
    text.drain(byte_start..byte_end);
    deleted
}

/// Delete from cursor to start of line (char indices)
pub fn delete_to_line_start(text: &mut String, cursor_pos: usize) -> String {
    let line_start = motions::line_start(text, cursor_pos);
    let deleted: String = text.chars().skip(line_start).take(cursor_pos - line_start).collect();
    let byte_start = c2b(text, line_start);
    let byte_end = c2b(text, cursor_pos);
    text.drain(byte_start..byte_end);
    deleted
}

/// Change (delete and return new cursor position for insert mode) (char indices)
pub fn change_range(text: &mut String, start: usize, end: usize) -> usize {
    let char_count = text.chars().count();
    let end_inclusive = (end + 1).min(char_count);
    let byte_start = c2b(text, start);
    let byte_end = c2b(text, end_inclusive);
    text.drain(byte_start..byte_end);
    start.min(text.chars().count())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delete_char() {
        let mut text = String::from("hello");
        let deleted = delete_char(&mut text, 0);
        assert_eq!(deleted, Some('h'));
        assert_eq!(text, "ello");
    }

    #[test]
    fn test_delete_line() {
        let mut text = String::from("hello\nworld\ntest");
        let (deleted, new_cursor) = delete_line(&mut text, 7);
        assert_eq!(deleted, "world\n");
        assert_eq!(text, "hello\ntest");
        assert_eq!(new_cursor, 6);
    }

    #[test]
    fn test_yank_range() {
        let text = "hello world";
        let yanked = yank_range(text, 0, 4);
        assert_eq!(yanked, "hello");
    }

    #[test]
    fn test_insert_text() {
        let mut text = String::from("hello");
        let new_pos = insert_text(&mut text, 5, " world");
        assert_eq!(text, "hello world");
        assert_eq!(new_pos, 11);
    }
}
