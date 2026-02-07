//! Query editor widget with syntax highlighting

use crate::app::{App, InputMode};
use crate::ui::DefaultTheme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

/// Line number gutter width (4 chars + 1 separator)
const LINE_NUMBER_WIDTH: u16 = 5;

/// Draw the query editor panel with line numbers and scrolling
pub fn draw_query_editor(f: &mut Frame, app: &mut App, area: Rect, active: bool) {
    let border_style = if active {
        DefaultTheme::active_border()
    } else {
        DefaultTheme::inactive_border()
    };

    // Title with active and input mode indicator
    let mode_indicator = match app.input_mode {
        InputMode::Insert => "[INSERT]",
        InputMode::Visual => "[VISUAL]",
        InputMode::Normal => "",
        InputMode::Command => "[COMMAND]",
    };
    let title = format!(" Query [<Cmd>q] {} {} ", if active { "▪" } else { "" }, mode_indicator);

    // Create outer block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(title, DefaultTheme::title()));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Split inner area: line numbers | code
    if inner_area.width > LINE_NUMBER_WIDTH + 2 {
        let line_num_area = Rect {
            x: inner_area.x,
            y: inner_area.y,
            width: LINE_NUMBER_WIDTH,
            height: inner_area.height,
        };

        let code_area = Rect {
            x: inner_area.x + LINE_NUMBER_WIDTH,
            y: inner_area.y,
            width: inner_area.width - LINE_NUMBER_WIDTH,
            height: inner_area.height,
        };

        // Update scroll position to keep cursor visible
        let visible_width = code_area.width as usize;
        let visible_height = code_area.height as usize;
        app.update_scroll(visible_width, visible_height);

        // Get lines from query
        let query_lines: Vec<&str> = if app.query.is_empty() {
            vec![""]
        } else {
            app.query.split('\n').collect()
        };

        // Draw line numbers (with vertical scroll)
        let line_numbers: Vec<Line> = query_lines
            .iter()
            .enumerate()
            .skip(app.query_scroll_y)
            .take(visible_height)
            .map(|(n, _)| {
                Line::from(Span::styled(
                    format!("{:>3} │", n + 1),
                    Style::default().fg(DefaultTheme::COMMENT),
                ))
            })
            .collect();

        let line_num_widget = Paragraph::new(line_numbers);
        f.render_widget(line_num_widget, line_num_area);

        // Get visual selection if in visual mode
        let visual_selection = if app.input_mode == InputMode::Visual {
            Some(app.get_visual_selection())
        } else {
            None
        };

        // Draw syntax-highlighted code with scrolling
        let highlighted_lines = highlight_sql_with_scroll(
            &app.query,
            app.query_scroll_x,
            app.query_scroll_y,
            visible_width,
            visible_height,
            visual_selection,
        );
        let code_widget = Paragraph::new(highlighted_lines);
        f.render_widget(code_widget, code_area);

        // Show cursor when query editor is active
        if active {
            let (cursor_x, cursor_y) = calculate_cursor_position_with_scroll(
                app,
                code_area,
            );
            f.set_cursor(cursor_x, cursor_y);
        }
    }
}

/// Calculate cursor position with scroll offset
fn calculate_cursor_position_with_scroll(app: &App, code_area: Rect) -> (u16, u16) {
    let (line, col) = app.get_cursor_line_col();

    // Adjust for scroll offset
    let visible_line = line.saturating_sub(app.query_scroll_y);
    let visible_col = col.saturating_sub(app.query_scroll_x);

    let x = (code_area.x + visible_col as u16).min(code_area.x + code_area.width.saturating_sub(1));
    let y = (code_area.y + visible_line as u16).min(code_area.y + code_area.height.saturating_sub(1));

    (x, y)
}

/// SQL syntax highlighting with scroll support and visual selection
fn highlight_sql_with_scroll(
    sql: &str,
    scroll_x: usize,
    scroll_y: usize,
    visible_width: usize,
    visible_height: usize,
    visual_selection: Option<(usize, usize)>, // (start, end) char positions
) -> Vec<Line<'static>> {
    let keywords = [
        "SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "IN", "LIKE", "BETWEEN",
        "ORDER", "BY", "ASC", "DESC", "GROUP", "HAVING", "JOIN", "INNER", "LEFT",
        "RIGHT", "OUTER", "FULL", "CROSS", "ON", "AS", "DISTINCT", "TOP", "WITH",
        "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "TABLE",
        "ALTER", "DROP", "INDEX", "VIEW", "PROCEDURE", "FUNCTION", "TRIGGER",
        "BEGIN", "END", "IF", "ELSE", "WHILE", "RETURN", "DECLARE", "EXEC", "EXECUTE",
        "NULL", "IS", "CASE", "WHEN", "THEN", "UNION", "ALL", "EXISTS", "COUNT",
        "SUM", "AVG", "MIN", "MAX", "CAST", "CONVERT", "COALESCE", "ISNULL",
    ];

    // Visual selection style (inverted colors)
    let visual_style = Style::default()
        .fg(DefaultTheme::BG_DARK)
        .bg(DefaultTheme::PRIMARY);

    let source_lines: Vec<&str> = sql.split('\n').collect();
    let mut lines: Vec<Line> = Vec::new();

    // Calculate absolute character position at the start of each line
    let mut line_starts: Vec<usize> = vec![0];
    let mut pos = 0;
    for line in &source_lines {
        pos += line.len() + 1; // +1 for newline
        line_starts.push(pos);
    }

    for (line_idx, line_content) in source_lines.iter().enumerate().skip(scroll_y).take(visible_height) {
        let line_start_pos = line_starts[line_idx];
        
        // Apply horizontal scroll
        let display_content: String = line_content
            .chars()
            .skip(scroll_x)
            .take(visible_width)
            .collect();

        let mut spans: Vec<Span> = Vec::new();
        let mut current_word = String::new();
        let mut in_string = false;
        let mut string_char = ' ';

        let chars: Vec<char> = display_content.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];
            // Calculate absolute position in the original string
            let abs_pos = line_start_pos + scroll_x + i;
            
            // Check if this character is in visual selection
            let in_visual = visual_selection.map_or(false, |(start, end)| {
                abs_pos >= start && abs_pos <= end
            });

            // Check for line comment
            if !in_string && i + 1 < chars.len() && chars[i] == '-' && chars[i + 1] == '-' {
                if !current_word.is_empty() {
                    spans.push(colorize_word(&current_word, &keywords));
                    current_word.clear();
                }
                // Rest of line is comment - check if any part is in selection
                let comment: String = chars[i..].iter().collect();
                if in_visual {
                    // Handle comment with visual selection
                    for (j, ch) in comment.chars().enumerate() {
                        let ch_abs_pos = line_start_pos + scroll_x + i + j;
                        let ch_in_visual = visual_selection.map_or(false, |(start, end)| {
                            ch_abs_pos >= start && ch_abs_pos <= end
                        });
                        if ch_in_visual {
                            spans.push(Span::styled(ch.to_string(), visual_style));
                        } else {
                            spans.push(Span::styled(ch.to_string(), Style::default().fg(DefaultTheme::COMMENT)));
                        }
                    }
                } else {
                    spans.push(Span::styled(comment, Style::default().fg(DefaultTheme::COMMENT)));
                }
                break;
            }

            // If in visual selection, use visual style
            if in_visual {
                if !current_word.is_empty() {
                    spans.push(colorize_word(&current_word, &keywords));
                    current_word.clear();
                }
                spans.push(Span::styled(c.to_string(), visual_style));
                i += 1;
                continue;
            }

            // Handle strings
            if (c == '\'' || c == '"') && !in_string {
                if !current_word.is_empty() {
                    spans.push(colorize_word(&current_word, &keywords));
                    current_word.clear();
                }
                in_string = true;
                string_char = c;
                current_word.push(c);
            } else if in_string && c == string_char {
                current_word.push(c);
                spans.push(Span::styled(
                    current_word.clone(),
                    Style::default().fg(DefaultTheme::STRING),
                ));
                current_word.clear();
                in_string = false;
            } else if in_string {
                current_word.push(c);
            } else if c.is_whitespace() || "(),;.=<>+-*/[]".contains(c) {
                if !current_word.is_empty() {
                    spans.push(colorize_word(&current_word, &keywords));
                    current_word.clear();
                }
                spans.push(Span::styled(
                    c.to_string(),
                    Style::default().fg(DefaultTheme::OPERATOR),
                ));
            } else {
                current_word.push(c);
            }

            i += 1;
        }

        if !current_word.is_empty() {
            if in_string {
                spans.push(Span::styled(current_word, Style::default().fg(DefaultTheme::STRING)));
            } else {
                spans.push(colorize_word(&current_word, &keywords));
            }
        }

        lines.push(Line::from(spans));
    }

    // Pad with empty lines if needed
    while lines.len() < visible_height {
        lines.push(Line::from(""));
    }

    lines
}

fn colorize_word(word: &str, keywords: &[&str]) -> Span<'static> {
    let upper = word.to_uppercase();

    if keywords.contains(&upper.as_str()) {
        Span::styled(
            word.to_string(),
            Style::default()
                .fg(DefaultTheme::KEYWORD)
                .add_modifier(Modifier::BOLD),
        )
    } else if word.chars().all(|c| c.is_ascii_digit() || c == '.') {
        Span::styled(
            word.to_string(),
            Style::default().fg(DefaultTheme::NUMBER),
        )
    } else if word.starts_with('@') || word.starts_with("@@") {
        Span::styled(
            word.to_string(),
            Style::default().fg(DefaultTheme::FUNCTION),
        )
    } else {
        Span::styled(word.to_string(), DefaultTheme::normal_text())
    }
}
