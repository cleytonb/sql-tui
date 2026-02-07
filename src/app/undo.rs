//! Undo/Redo system for the query editor
//!
//! Implements a vim-like undo system with snapshots of text state.

/// A snapshot of the editor state for undo/redo
#[derive(Clone, Debug)]
pub struct EditorSnapshot {
    /// The text content
    pub text: String,
    /// Cursor position
    pub cursor_pos: usize,
}

/// Undo manager with undo/redo stacks
pub struct UndoManager {
    /// Stack of previous states (for undo)
    undo_stack: Vec<EditorSnapshot>,
    /// Stack of undone states (for redo)
    redo_stack: Vec<EditorSnapshot>,
    /// Maximum number of undo levels
    max_history: usize,
}

impl UndoManager {
    /// Create a new undo manager
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history,
        }
    }

    /// Save current state before making changes
    /// 
    /// Call this BEFORE modifying the text (not after)
    pub fn save_state(&mut self, text: &str, cursor_pos: usize) {
        // Don't save if identical to last state
        if let Some(last) = self.undo_stack.last() {
            if last.text == text && last.cursor_pos == cursor_pos {
                return;
            }
        }

        self.undo_stack.push(EditorSnapshot {
            text: text.to_string(),
            cursor_pos,
        });

        // Clear redo stack when new changes are made
        self.redo_stack.clear();

        // Limit history size
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }

    /// Undo: restore previous state
    /// 
    /// Returns the state to restore, or None if nothing to undo
    pub fn undo(&mut self, current_text: &str, current_cursor: usize) -> Option<EditorSnapshot> {
        if let Some(state) = self.undo_stack.pop() {
            // Save current state to redo stack
            self.redo_stack.push(EditorSnapshot {
                text: current_text.to_string(),
                cursor_pos: current_cursor,
            });
            Some(state)
        } else {
            None
        }
    }

    /// Redo: restore previously undone state
    /// 
    /// Returns the state to restore, or None if nothing to redo
    pub fn redo(&mut self, current_text: &str, current_cursor: usize) -> Option<EditorSnapshot> {
        if let Some(state) = self.redo_stack.pop() {
            // Save current state to undo stack
            self.undo_stack.push(EditorSnapshot {
                text: current_text.to_string(),
                cursor_pos: current_cursor,
            });
            Some(state)
        } else {
            None
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get number of undo levels available
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get number of redo levels available
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_redo() {
        let mut undo = UndoManager::new(100);
        
        // Initial state
        undo.save_state("hello", 5);
        
        // Make changes
        undo.save_state("hello world", 11);
        
        // Undo
        let state = undo.undo("hello world!!!", 14).unwrap();
        assert_eq!(state.text, "hello world");
        
        let state = undo.undo("hello world", 11).unwrap();
        assert_eq!(state.text, "hello");
        
        // Redo
        let state = undo.redo("hello", 5).unwrap();
        assert_eq!(state.text, "hello world");
    }
}
