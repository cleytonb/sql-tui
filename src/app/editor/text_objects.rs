//! Text objects for vim-like selection
//!
//! This module provides text object functionality for selecting
//! semantic units of text like words, sentences, paragraphs,
//! and delimited content (quotes, brackets, etc.)
//!
//! Text objects come in two flavors:
//! - "inner" (i) - selects content without delimiters
//! - "around" (a) - selects content including delimiters

/// Text object result containing start and end positions
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextObject {
    pub start: usize,
    pub end: usize,
}

impl TextObject {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
    
    /// Get the text content from the source
    pub fn extract<'a>(&self, text: &'a str) -> String {
        text.chars().skip(self.start).take(self.end - self.start + 1).collect()
    }
}

/// Find the inner word at cursor position (iw)
pub fn inner_word(text: &str, cursor_pos: usize) -> Option<TextObject> {
    let chars: Vec<char> = text.chars().collect();
    if cursor_pos >= chars.len() {
        return None;
    }
    
    // Find word boundaries
    let mut start = cursor_pos;
    let mut end = cursor_pos;
    
    // If on whitespace, return the whitespace
    if chars[cursor_pos].is_whitespace() {
        while start > 0 && chars[start - 1].is_whitespace() && chars[start - 1] != '\n' {
            start -= 1;
        }
        while end < chars.len() - 1 && chars[end + 1].is_whitespace() && chars[end + 1] != '\n' {
            end += 1;
        }
        return Some(TextObject::new(start, end));
    }
    
    // Find word start
    while start > 0 && chars[start - 1].is_alphanumeric() {
        start -= 1;
    }
    
    // Find word end
    while end < chars.len() - 1 && chars[end + 1].is_alphanumeric() {
        end += 1;
    }
    
    Some(TextObject::new(start, end))
}

/// Find the word with surrounding whitespace (aw)
pub fn a_word(text: &str, cursor_pos: usize) -> Option<TextObject> {
    let chars: Vec<char> = text.chars().collect();
    if let Some(mut obj) = inner_word(text, cursor_pos) {
        // Include trailing whitespace
        while obj.end < chars.len() - 1 && chars[obj.end + 1].is_whitespace() && chars[obj.end + 1] != '\n' {
            obj.end += 1;
        }
        Some(obj)
    } else {
        None
    }
}

/// Find content inside quotes (i" or i')
pub fn inner_quoted(text: &str, cursor_pos: usize, quote_char: char) -> Option<TextObject> {
    let chars: Vec<char> = text.chars().collect();
    
    // Find opening quote (search backward first, then forward)
    let mut start = None;
    let mut end = None;
    
    // Search backward for opening quote
    for i in (0..=cursor_pos).rev() {
        if chars[i] == quote_char {
            start = Some(i);
            break;
        }
        if chars[i] == '\n' {
            break; // Don't cross lines
        }
    }
    
    // Search forward for closing quote
    if let Some(s) = start {
        for i in (s + 1)..chars.len() {
            if chars[i] == quote_char {
                end = Some(i);
                break;
            }
            if chars[i] == '\n' {
                break; // Don't cross lines
            }
        }
    }
    
    match (start, end) {
        (Some(s), Some(e)) if e > s + 1 => Some(TextObject::new(s + 1, e - 1)),
        _ => None,
    }
}

/// Find content including quotes (a" or a')
pub fn a_quoted(text: &str, cursor_pos: usize, quote_char: char) -> Option<TextObject> {
    let chars: Vec<char> = text.chars().collect();
    
    let mut start = None;
    let mut end = None;
    
    for i in (0..=cursor_pos).rev() {
        if chars[i] == quote_char {
            start = Some(i);
            break;
        }
        if chars[i] == '\n' {
            break;
        }
    }
    
    if let Some(s) = start {
        for i in (s + 1)..chars.len() {
            if chars[i] == quote_char {
                end = Some(i);
                break;
            }
            if chars[i] == '\n' {
                break;
            }
        }
    }
    
    match (start, end) {
        (Some(s), Some(e)) => Some(TextObject::new(s, e)),
        _ => None,
    }
}

/// Find content inside brackets (i( i[ i{ i<)
pub fn inner_bracket(text: &str, cursor_pos: usize, open: char, close: char) -> Option<TextObject> {
    let chars: Vec<char> = text.chars().collect();
    
    // Find matching brackets using a stack-like approach
    let mut depth = 0i32;
    let mut start = None;
    
    // Search backward for opening bracket
    for i in (0..=cursor_pos).rev() {
        if chars[i] == close {
            depth += 1;
        } else if chars[i] == open {
            if depth == 0 {
                start = Some(i);
                break;
            }
            depth -= 1;
        }
    }
    
    // Search forward for closing bracket
    if let Some(s) = start {
        depth = 0;
        for i in (s + 1)..chars.len() {
            if chars[i] == open {
                depth += 1;
            } else if chars[i] == close {
                if depth == 0 {
                    if i > s + 1 {
                        return Some(TextObject::new(s + 1, i - 1));
                    } else {
                        return None; // Empty brackets
                    }
                }
                depth -= 1;
            }
        }
    }
    
    None
}

/// Find content including brackets (a( a[ a{ a<)
pub fn a_bracket(text: &str, cursor_pos: usize, open: char, close: char) -> Option<TextObject> {
    let chars: Vec<char> = text.chars().collect();
    
    let mut depth = 0i32;
    let mut start = None;
    
    for i in (0..=cursor_pos).rev() {
        if chars[i] == close {
            depth += 1;
        } else if chars[i] == open {
            if depth == 0 {
                start = Some(i);
                break;
            }
            depth -= 1;
        }
    }
    
    if let Some(s) = start {
        depth = 0;
        for i in (s + 1)..chars.len() {
            if chars[i] == open {
                depth += 1;
            } else if chars[i] == close {
                if depth == 0 {
                    return Some(TextObject::new(s, i));
                }
                depth -= 1;
            }
        }
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inner_word() {
        let text = "hello world";
        let obj = inner_word(text, 2).unwrap();
        assert_eq!(obj.start, 0);
        assert_eq!(obj.end, 4);
        assert_eq!(obj.extract(text), "hello");
    }

    #[test]
    fn test_inner_quoted() {
        let text = r#"say "hello world" please"#;
        let obj = inner_quoted(text, 8, '"').unwrap();
        assert_eq!(obj.extract(text), "hello world");
    }

    #[test]
    fn test_inner_bracket() {
        let text = "func(arg1, arg2)";
        let obj = inner_bracket(text, 8, '(', ')').unwrap();
        assert_eq!(obj.extract(text), "arg1, arg2");
    }

    #[test]
    fn test_a_bracket() {
        let text = "func(arg1, arg2)";
        let obj = a_bracket(text, 8, '(', ')').unwrap();
        assert_eq!(obj.extract(text), "(arg1, arg2)");
    }
}
