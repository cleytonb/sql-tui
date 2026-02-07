//! Vim-like editor functionality
//! 
//! This module provides vim-style text editing capabilities including:
//! - Cursor motions (word, line, document)
//! - Text operations (delete, yank, change)
//! - Text objects (future: iw, aw, i", etc.)

pub mod motions;
pub mod operations;
pub mod text_objects;

pub use motions::*;
pub use operations::*;
