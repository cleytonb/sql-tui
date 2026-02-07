//! Text editing operations for vim-like functionality
//!
//! This module contains functions for modifying text,
//! implementing vim-style operations like delete, yank, change, etc.

use super::motions;

/// Delete a line from the text, returning the deleted content
pub fn delete_line(text: &mut String, cursor_pos: usize) -> (String, usize) {
    let line_start = motions::line_start(text, cursor_pos);
    let mut line_end = motions::line_end(text, cursor_pos);
    
    // Include the newline if present
    if line_end < text.len() {
        line_end += 1;
    }
    
    let deleted: String = text.chars().skip(line_start).take(line_end - line_start).collect();
    text.drain(line_start..line_end);
    
    let new_cursor = line_start.min(text.len().saturating_sub(1));
    (deleted, new_cursor)
}

/// Delete a character at the given position
pub fn delete_char(text: &mut String, cursor_pos: usize) -> Option<char> {
    if cursor_pos < text.len() {
        Some(text.remove(cursor_pos))
    } else {
        None
    }
}

/// Delete a range of text, returning the deleted content
pub fn delete_range(text: &mut String, start: usize, end: usize) -> String {
    let end_inclusive = (end + 1).min(text.len());
    let deleted: String = text.chars().skip(start).take(end_inclusive - start).collect();
    text.drain(start..end_inclusive);
    deleted
}

/// Yank (copy) a range of text
pub fn yank_range(text: &str, start: usize, end: usize) -> String {
    let end_inclusive = (end + 1).min(text.len());
    text.chars().skip(start).take(end_inclusive - start).collect()
}

/// Yank the current line
pub fn yank_line(text: &str, cursor_pos: usize) -> String {
    let line_start = motions::line_start(text, cursor_pos);
    let mut line_end = motions::line_end(text, cursor_pos);
    
    // Include the newline if present
    if line_end < text.len() {
        line_end += 1;
    }
    
    text.chars().skip(line_start).take(line_end - line_start).collect()
}

/// Insert text at the given position
pub fn insert_text(text: &mut String, pos: usize, content: &str) -> usize {
    for (i, c) in content.chars().enumerate() {
        text.insert(pos + i, c);
    }
    pos + content.len()
}

/// Replace a character at the given position
pub fn replace_char(text: &mut String, cursor_pos: usize, new_char: char) {
    if cursor_pos < text.len() {
        text.remove(cursor_pos);
        text.insert(cursor_pos, new_char);
    }
}

/// Join the current line with the next line
pub fn join_lines(text: &mut String, cursor_pos: usize) -> usize {
    let line_end = motions::line_end(text, cursor_pos);
    
    if line_end < text.len() {
        // Remove the newline
        text.remove(line_end);
        // Insert a space if needed
        if line_end < text.len() && !text.chars().nth(line_end).unwrap().is_whitespace() {
            text.insert(line_end, ' ');
        }
    }
    
    cursor_pos
}

/// Delete from cursor to end of line (D command)
pub fn delete_to_line_end(text: &mut String, cursor_pos: usize) -> String {
    let line_end = motions::line_end(text, cursor_pos);
    let deleted: String = text.chars().skip(cursor_pos).take(line_end - cursor_pos).collect();
    text.drain(cursor_pos..line_end);
    deleted
}

/// Delete from cursor to start of line
pub fn delete_to_line_start(text: &mut String, cursor_pos: usize) -> String {
    let line_start = motions::line_start(text, cursor_pos);
    let deleted: String = text.chars().skip(line_start).take(cursor_pos - line_start).collect();
    text.drain(line_start..cursor_pos);
    deleted
}

/// Change (delete and return new cursor position for insert mode)
pub fn change_range(text: &mut String, start: usize, end: usize) -> usize {
    let end_inclusive = (end + 1).min(text.len());
    text.drain(start..end_inclusive);
    start.min(text.len())
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
