//! Results table widget

use crate::app::{App, ResultsTab};
use crate::db::CellValue;
use crate::ui::DefaultTheme;
use crate::ui::widgets::helpers::{format_cell_value, format_number, get_type_indicator};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table};
use ratatui::layout::Margin;
use std::collections::HashMap;
use rust_i18n::t;

/// Draw the results table panel with tabs
pub fn draw_results_table(f: &mut Frame, app: &mut App, area: Rect, active: bool) {
    let border_style = if active {
        DefaultTheme::active_border()
    } else {
        DefaultTheme::inactive_border()
    };

    // Draw tabs header
    let tabs_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 2,
    };

    let content_area = Rect {
        x: area.x,
        y: area.y + 2,
        width: area.width,
        height: area.height.saturating_sub(2),
    };

    // Draw tab bar
    draw_results_tabs(f, app, tabs_area, active);

    if app.result.columns.is_empty() {
        let help_text = vec![
            Line::from(""),
            Line::from(Span::styled(t!("no_results").to_string(), DefaultTheme::dim_text())),
            Line::from(""),
            Line::from(vec![
                Span::styled(t!("type_query_hint").to_string(), DefaultTheme::dim_text()),
                Span::styled("Enter", DefaultTheme::info()),
                Span::styled(t!("to_execute").to_string(), DefaultTheme::dim_text()),
            ]),
        ];
        let empty_msg = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .alignment(Alignment::Center);
        f.render_widget(empty_msg, content_area);
        return;
    }

    // Draw content based on selected tab
    match app.results_tab {
        ResultsTab::Data => draw_results_data(f, app, content_area, active),
        ResultsTab::Columns => draw_results_columns(f, app, content_area, active),
        ResultsTab::Stats => draw_results_stats(f, app, content_area, active),
    }
}

/// Draw the tabs bar
fn draw_results_tabs(f: &mut Frame, app: &App, area: Rect, active: bool) {
    let tabs = vec![
        ("1:Dados", ResultsTab::Data),
        ("2:Colunas", ResultsTab::Columns),
        ("3:Estatísticas", ResultsTab::Stats),
    ];

    let mut spans: Vec<Span> = vec![Span::raw(" ")];
    for (label, tab) in tabs {
        let style = if app.results_tab == tab {
            Style::default()
                .fg(DefaultTheme::TEXT)
                .bg(DefaultTheme::PRIMARY)
                .add_modifier(Modifier::BOLD)
        } else if active {
            Style::default().fg(DefaultTheme::TEXT_DIM)
        } else {
            Style::default().fg(DefaultTheme::TEXT_MUTED)
        };
        spans.push(Span::styled(format!(" {} ", label), style));
        spans.push(Span::raw(" "));
    }

    // Add row/col info on the right
    if !app.result.columns.is_empty() {
        let info = format!(
            "│ {} linhas × {} colunas ",
            app.result.row_count,
            app.result.columns.len()
        );
        spans.push(Span::styled(info, DefaultTheme::dim_text()));
    }

    let tabs_line = Line::from(spans);
    let tabs_widget = Paragraph::new(tabs_line);
        // .style(Style::default().bg(DefaultTheme::BG_PANEL));
    f.render_widget(tabs_widget, area);
}

/// Draw the data tab (table rows)
fn draw_results_data(f: &mut Frame, app: &mut App, area: Rect, active: bool) {
    let border_style = if active {
        DefaultTheme::active_border()
    } else {
        DefaultTheme::inactive_border()
    };

    // Build title with stats
    let exec_time_ms = app.result.execution_time.as_secs_f64() * 1000.0;
    let title = format!(
        " Dados │ {} linhas │ {} colunas │ {:.1}ms ",
        app.result.row_count,
        app.result.columns.len(),
        exec_time_ms
    );

    // Calculate available width for columns
    let available_width = area.width.saturating_sub(2) as usize; // minus borders
    let row_num_width = (app.result.rows.len().to_string().len() + 2).max(4) as u16;

    // Calculate which columns to show based on horizontal scroll
    // Each column gets a fixed width for consistent display
    let col_width: u16 = 30; // Fixed column width
    let cols_that_fit = ((available_width as u16).saturating_sub(row_num_width) / col_width).max(1) as usize;

    // Atualiza número de colunas visíveis para uso no handler
    app.results_cols_visible = cols_that_fit;
    
    // Calcula scroll horizontal para manter coluna selecionada visível
    // Se coluna selecionada está antes da área visível, ajusta scroll para esquerda
    if app.results_col_selected < app.results_col_scroll {
        app.results_col_scroll = app.results_col_selected;
    }
    // Se coluna selecionada está depois da área visível, ajusta scroll para direita
    else if app.results_col_selected >= app.results_col_scroll + cols_that_fit {
        app.results_col_scroll = app.results_col_selected.saturating_sub(cols_that_fit - 1);
    }
    
    let col_scroll = app.results_col_scroll;

    // Get visible columns range
    let visible_cols_start = col_scroll;
    let visible_cols_end = (col_scroll + cols_that_fit).min(app.result.columns.len());

    // Build column widths
    let mut widths: Vec<Constraint> = vec![Constraint::Length(row_num_width)];
    for _ in visible_cols_start..visible_cols_end {
        widths.push(Constraint::Length(col_width));
    }

    // Create header row with row number column and type indicators
    let mut header_cells: Vec<Cell> = vec![
        Cell::from(" # ").style(DefaultTheme::table_header())
    ];
    header_cells.extend(
        app.result
            .columns
            .iter()
            .enumerate()
            .skip(visible_cols_start)
            .take(visible_cols_end - visible_cols_start)
            .map(|(i, c)| {
                // Get type indicator
                let type_indicator = get_type_indicator(&c.type_name);
                // Truncate column name to fit
                let name: String = c.name.chars().take(col_width as usize - 4).collect();
                let header_text = format!("{} {}", type_indicator, name);

                let style = if active && i == app.results_col_selected {
                    DefaultTheme::selected()
                } else {
                    DefaultTheme::table_header()
                };
                Cell::from(header_text).style(style)
            })
    );
    let header = Row::new(header_cells).height(1);

    // Create data rows with row numbers
    let visible_height = area.height.saturating_sub(3) as usize;
    let scroll_offset = if app.results_selected >= visible_height {
        app.results_selected.saturating_sub(visible_height - 1)
    } else {
        0
    };

    let rows: Vec<Row> = app
        .result
        .rows
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
        .map(|(row_idx, row)| {
            // Row number cell
            let row_num_style = if active && row_idx == app.results_selected {
                DefaultTheme::selected()
            } else {
                DefaultTheme::row_number()
            };
            let mut cells: Vec<Cell> = vec![
                Cell::from(format!("{:>width$} ", row_idx + 1, width = row_num_width as usize - 1))
                    .style(row_num_style)
            ];

            // Data cells - only visible columns
            cells.extend(
                row.iter()
                    .enumerate()
                    .skip(visible_cols_start)
                    .take(visible_cols_end - visible_cols_start)
                    .map(|(col_idx, cell)| {
                        let (value, is_null) = format_cell_value(cell);
                        // Truncate value to fit column
                        let display_value: String = value.chars().take(col_width as usize - 2).collect();

                        let style = if active && row_idx == app.results_selected && col_idx == app.results_col_selected {
                            DefaultTheme::selected()
                        } else if active && row_idx == app.results_selected {
                            DefaultTheme::highlighted()
                        } else if is_null {
                            DefaultTheme::null_value()
                        // } else if row_idx % 2 == 1 {
                            // DefaultTheme::table_row_alt()
                        } else {
                            DefaultTheme::normal_text()
                        };

                        Cell::from(format!(" {} ", display_value)).style(style)
                    })
            );
            Row::new(cells)
        })
        .collect();

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(Span::styled(title, DefaultTheme::title())),
        )
        .highlight_style(DefaultTheme::highlighted());

    f.render_widget(table, area);

    // Draw scrollbar if needed
    if app.result.rows.len() > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"));

        let mut scrollbar_state = ScrollbarState::new(app.result.rows.len())
            .position(app.results_selected);

        f.render_stateful_widget(
            scrollbar,
            area.inner(&Margin { vertical: 1, horizontal: 0 }),
            &mut scrollbar_state,
        );
    }

    // Draw position indicator at bottom right
    if !app.result.rows.is_empty() {
        let pos_text = format!(
            " Linha {}/{} Coluna {}/{} ",
            app.results_selected + 1,
            app.result.rows.len(),
            app.results_col_selected + 1,
            app.result.columns.len()
        );
        let pos_len = pos_text.len() as u16;
        let pos_x = area.x + area.width.saturating_sub(pos_len + 2);
        let pos_y = area.y + area.height.saturating_sub(1);

        if pos_x > area.x && pos_y < area.y + area.height {
            let pos_span = Span::styled(pos_text, DefaultTheme::dim_text());
            f.render_widget(
                Paragraph::new(pos_span),
                Rect::new(pos_x, pos_y, pos_len, 1),
            );
        }
    }
}

/// Draw the columns tab (column info)
fn draw_results_columns(f: &mut Frame, app: &App, area: Rect, active: bool) {
    let border_style = if active {
        DefaultTheme::active_border()
    } else {
        DefaultTheme::inactive_border()
    };

    let title = format!(" Colunas │ {} total ", app.result.columns.len());

    // Create column info rows - use results_selected for vertical scrolling
    let visible_height = area.height.saturating_sub(3) as usize;
    let scroll_offset = if app.results_selected >= visible_height {
        app.results_selected.saturating_sub(visible_height - 1)
    } else {
        0
    };

    let rows: Vec<Row> = app
        .result
        .columns
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
        .map(|(idx, col)| {
            let type_indicator = get_type_indicator(&col.type_name);
            let row_style = if active && idx == app.results_selected {
                DefaultTheme::selected()
            } else if idx % 2 == 1 {
                DefaultTheme::table_row_alt()
            } else {
                DefaultTheme::normal_text()
            };

            Row::new(vec![
                Cell::from(format!(" {:>3} ", idx + 1)).style(DefaultTheme::row_number()),
                Cell::from(format!(" {} ", type_indicator)),
                Cell::from(format!(" {} ", col.name)).style(row_style),
                Cell::from(format!(" {} ", col.type_name)).style(DefaultTheme::dim_text()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(6),   // #
        Constraint::Length(4),   // Icon
        Constraint::Min(20),     // Name
        Constraint::Length(20),  // Type
    ];

    let header = Row::new(vec![
        Cell::from(" # ").style(DefaultTheme::table_header()),
        Cell::from(" ").style(DefaultTheme::table_header()),
        Cell::from(" Nome da Coluna ").style(DefaultTheme::table_header()),
        Cell::from(" Tipo de Dados ").style(DefaultTheme::table_header()),
    ])
    .height(1);

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(Span::styled(title, DefaultTheme::title())),
        );

    f.render_widget(table, area);

    // Draw scrollbar if needed
    if app.result.columns.len() > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"));

        let mut scrollbar_state = ScrollbarState::new(app.result.columns.len())
            .position(app.results_selected);

        f.render_stateful_widget(
            scrollbar,
            area.inner(&Margin { vertical: 1, horizontal: 0 }),
            &mut scrollbar_state,
        );
    }
}

/// Draw the stats tab (query statistics)
fn draw_results_stats(f: &mut Frame, app: &App, area: Rect, active: bool) {
    let border_style = if active {
        DefaultTheme::active_border()
    } else {
        DefaultTheme::inactive_border()
    };

    let exec_time = app.result.execution_time;
    let exec_ms = exec_time.as_secs_f64() * 1000.0;

    // Count data types
    let mut type_counts: HashMap<String, usize> = HashMap::new();
    for col in &app.result.columns {
        *type_counts.entry(col.type_name.clone()).or_insert(0) += 1;
    }

    // Count NULL values
    let mut null_count = 0;
    let mut total_cells = 0;
    for row in &app.result.rows {
        for cell in row {
            total_cells += 1;
            if matches!(cell, CellValue::Null) {
                null_count += 1;
            }
        }
    }

    let null_percentage = if total_cells > 0 {
        (null_count as f64 / total_cells as f64) * 100.0
    } else {
        0.0
    };

    // Build stats text with aligned labels
    let labels = [
        t!("execution_time").to_string(),
        t!("stats_rows_returned").to_string(),
        t!("columns").to_string(),
        t!("total_cells").to_string(),
        t!("null_values").to_string(),
    ];
    let max_label_len = labels.iter().map(|l| l.trim().len()).max().unwrap_or(0);

    let pad_label = |label: &str| -> String {
        let trimmed = label.trim();
        format!("  {:<width$}  ", trimmed, width = max_label_len)
    };

    let stats_lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(t!("stats_header").to_string(), DefaultTheme::info())),
        Line::from(""),
        Line::from(vec![
            Span::styled(pad_label(&labels[0]), DefaultTheme::dim_text()),
            Span::styled(format!("{:.2} ms", exec_ms), DefaultTheme::success()),
        ]),
        Line::from(vec![
            Span::styled(pad_label(&labels[1]), DefaultTheme::dim_text()),
            Span::styled(format_number(app.result.row_count as i64), DefaultTheme::info()),
        ]),
        Line::from(vec![
            Span::styled(pad_label(&labels[2]), DefaultTheme::dim_text()),
            Span::styled(format!("{}", app.result.columns.len()), DefaultTheme::info()),
        ]),
        Line::from(vec![
            Span::styled(pad_label(&labels[3]), DefaultTheme::dim_text()),
            Span::styled(format_number(total_cells as i64), DefaultTheme::normal_text()),
        ]),
        Line::from(vec![
            Span::styled(pad_label(&labels[4]), DefaultTheme::dim_text()),
            Span::styled(format!("{} ({:.1}%)", format_number(null_count as i64), null_percentage), DefaultTheme::warning()),
        ]),
        // Line::from(""),
        // Line::from(Span::styled("═══ TIPOS DE DADOS ═══", DefaultTheme::info())),
        // Line::from(""),
    ];

    // Add type breakdown
    // let mut type_vec: Vec<(&String, &usize)> = type_counts.iter().collect();
    // type_vec.sort_by(|a, b| b.1.cmp(a.1));

    // for (type_name, count) in type_vec.iter().take(10) {
    //     let indicator = get_type_indicator(type_name);
    //     stats_lines.push(Line::from(vec![
    //         Span::styled(format!("  {} ", indicator), DefaultTheme::normal_text()),
    //         Span::styled(format!("{:<20}", type_name), DefaultTheme::dim_text()),
    //         Span::styled(format!("{:>5} coluna(s)", count), DefaultTheme::normal_text()),
    //     ]));
    // }

    // stats_lines.push(Line::from(""));
    // stats_lines.push(Line::from(Span::styled("═══ ATALHOS ═══", DefaultTheme::info())));
    // stats_lines.push(Line::from(""));
    // stats_lines.push(Line::from(vec![
    //     Span::styled("  Ctrl+E  ", DefaultTheme::info()),
    //     Span::styled("Exportar para CSV", DefaultTheme::dim_text()),
    // ]));
    // stats_lines.push(Line::from(vec![
    //     Span::styled("  Ctrl+S  ", DefaultTheme::info()),
    //     Span::styled("Exportar para JSON", DefaultTheme::dim_text()),
    // ]));
    // stats_lines.push(Line::from(vec![
    //     Span::styled("  Ctrl+I  ", DefaultTheme::info()),
    //     Span::styled("Copiar linha como INSERT", DefaultTheme::dim_text()),
    // ]));
    // stats_lines.push(Line::from(vec![
    //     Span::styled("  Ctrl+Y  ", DefaultTheme::info()),
    //     Span::styled("Copiar valor da célula", DefaultTheme::dim_text()),
    // ]));

    let stats_widget = Paragraph::new(stats_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(Span::styled(format!(" {} ", t!("stats_title")), DefaultTheme::title())),
        );

    f.render_widget(stats_widget, area);
}
