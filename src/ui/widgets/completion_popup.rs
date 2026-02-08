//! Completion popup widget for autocomplete suggestions

use crate::app::App;
use crate::completion::CompletionKind;
use crate::ui::DefaultTheme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};

/// Maximum number of items to show in the popup
const MAX_VISIBLE_ITEMS: usize = 10;

/// Minimum width of the popup
const MIN_POPUP_WIDTH: u16 = 20;

/// Maximum width of the popup
const MAX_POPUP_WIDTH: u16 = 50;

/// Draw the completion popup overlay
pub fn draw_completion_popup(f: &mut Frame, app: &App, editor_area: Rect) {
    if !app.completion.visible || app.completion.items.is_empty() {
        return;
    }

    // Calculate popup position based on cursor
    let (cursor_line, cursor_col) = app.get_cursor_line_col();
    
    // Adjust for scroll offset and line numbers (5 chars for line numbers)
    let line_number_width = 5u16;
    let visible_line = cursor_line.saturating_sub(app.query_scroll_y);
    let visible_col = cursor_col.saturating_sub(app.query_scroll_x);
    
    // Calculate popup dimensions
    let item_count = app.completion.items.len().min(MAX_VISIBLE_ITEMS);
    let popup_height = (item_count as u16) + 2; // +2 for borders
    
    // Find the maximum item width
    let max_label_width = app.completion.items
        .iter()
        .take(MAX_VISIBLE_ITEMS)
        .map(|item| {
            // icon (2) + label + detail spacing
            let detail_len = item.detail.as_ref().map(|d| d.len() + 3).unwrap_or(0);
            2 + item.label.len() + detail_len
        })
        .max()
        .unwrap_or(MIN_POPUP_WIDTH as usize);
    
    let popup_width = (max_label_width as u16 + 4)
        .max(MIN_POPUP_WIDTH)
        .min(MAX_POPUP_WIDTH);
    
    // Calculate popup position
    // Start below the cursor, offset by prefix length
    let prefix_offset = app.completion.prefix.len() as u16;
    let popup_x = (editor_area.x + line_number_width + 1 + visible_col as u16)
        .saturating_sub(prefix_offset);
    let popup_y = editor_area.y + 1 + visible_line as u16 + 1; // +1 for border, +1 below cursor
    
    // Ensure popup fits on screen
    let screen_width = f.size().width;
    let screen_height = f.size().height;
    
    // Adjust X if popup would go off right edge
    let popup_x = if popup_x + popup_width > screen_width {
        screen_width.saturating_sub(popup_width + 1)
    } else {
        popup_x
    };
    
    // If popup would go below screen, show it above cursor instead
    let popup_y = if popup_y + popup_height > screen_height {
        // Show above cursor
        (editor_area.y + 1 + visible_line as u16).saturating_sub(popup_height)
    } else {
        popup_y
    };
    
    // Create popup area
    let popup_area = Rect::new(
        popup_x,
        popup_y,
        popup_width.min(screen_width.saturating_sub(popup_x)),
        popup_height.min(screen_height.saturating_sub(popup_y)),
    );
    
    // Calculate which items to show (handle scrolling within popup)
    let visible_start = if app.completion.selected >= MAX_VISIBLE_ITEMS {
        app.completion.selected - MAX_VISIBLE_ITEMS + 1
    } else {
        0
    };
    
    // Create list items
    let items: Vec<ListItem> = app.completion.items
        .iter()
        .enumerate()
        .skip(visible_start)
        .take(MAX_VISIBLE_ITEMS)
        .map(|(i, item)| {
            let is_selected = i == app.completion.selected;
            
            // Build the display line
            let kind_indicator = match item.kind {
                CompletionKind::Keyword => "K",
                CompletionKind::Schema => "S",
                CompletionKind::Table => "T",
                CompletionKind::View => "V",
                CompletionKind::Procedure => "P",
                CompletionKind::Column => "C",
                CompletionKind::Function => "F",
            };
            
            let kind_color = match item.kind {
                CompletionKind::Keyword => DefaultTheme::KEYWORD,
                CompletionKind::Schema => DefaultTheme::INFO,
                CompletionKind::Table => DefaultTheme::SUCCESS,
                CompletionKind::View => DefaultTheme::PRIMARY_LIGHT,
                CompletionKind::Procedure => DefaultTheme::GOLD,
                CompletionKind::Column => DefaultTheme::TEXT_DIM,
                CompletionKind::Function => DefaultTheme::FUNCTION,
            };
            
            // Create spans for the item
            let mut spans = vec![
                Span::styled(
                    format!("{} ", kind_indicator),
                    Style::default().fg(kind_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &item.label,
                    if is_selected {
                        Style::default().fg(DefaultTheme::TEXT).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(DefaultTheme::TEXT)
                    },
                ),
            ];
            
            // Add detail (schema) if present
            if let Some(ref detail) = item.detail {
                spans.push(Span::styled(
                    format!(" ({})", detail),
                    Style::default().fg(DefaultTheme::TEXT_MUTED),
                ));
            }
            
            let line = Line::from(spans);
            
            if is_selected {
                ListItem::new(line).style(DefaultTheme::selected())
            } else {
                ListItem::new(line)
            }
        })
        .collect();
    
    // Clear the area first
    f.render_widget(Clear, popup_area);
    
    // Build title with count info
    let total = app.completion.items.len();
    let title = if total > MAX_VISIBLE_ITEMS {
        format!(" {}/{} ", app.completion.selected + 1, total)
    } else {
        String::new()
    };
    
    // Create and render the list
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(DefaultTheme::popup_border())
                .style(DefaultTheme::popup())
                .title(title)
        );
    
    f.render_widget(list, popup_area);
}
