use ratatui::style::{Color, Modifier, Style};

pub struct DefaultTheme;

impl DefaultTheme {
    // Primary brand colors
    pub const PRIMARY: Color = Color::Rgb(0, 102, 51);      // Default Dark
    pub const PRIMARY_LIGHT: Color = Color::Rgb(0, 76, 153); // Light Default Blue
    pub const PRIMARY_DARK: Color = Color::Rgb(0, 0, 102);   // Dark Default Blue

    // Secondary colors
    pub const GOLD: Color = Color::Rgb(197, 164, 103);       // Default Gold
    pub const GOLD_LIGHT: Color = Color::Rgb(218, 195, 148);

    // UI colors
    pub const BG_DARK: Color = Color::Rgb(18, 18, 24);       // Dark background
    pub const BG_PANEL: Color = Color::Rgb(28, 28, 36);      // Panel background
    pub const BG_HIGHLIGHT: Color = Color::Rgb(38, 38, 48);  // Highlight background

    // Text colors
    pub const TEXT: Color = Color::Rgb(230, 230, 230);       // Primary text
    pub const TEXT_DIM: Color = Color::Rgb(150, 150, 160);   // Secondary text
    pub const TEXT_MUTED: Color = Color::Rgb(100, 100, 110); // Muted text

    // Status colors
    pub const SUCCESS: Color = Color::Rgb(80, 200, 120);     // Success green
    pub const ERROR: Color = Color::Rgb(255, 100, 100);      // Error red
    pub const WARNING: Color = Color::Rgb(255, 200, 100);    // Warning yellow
    pub const INFO: Color = Color::Rgb(100, 180, 255);       // Info blue

    // SQL Syntax highlighting
    pub const KEYWORD: Color = Color::Rgb(197, 134, 192);    // Purple for keywords
    pub const STRING: Color = Color::Rgb(206, 145, 120);     // Orange for strings
    pub const NUMBER: Color = Color::Rgb(181, 206, 168);     // Green for numbers
    pub const COMMENT: Color = Color::Rgb(106, 153, 85);     // Gray-green for comments
    pub const FUNCTION: Color = Color::Rgb(220, 220, 170);   // Yellow for functions
    pub const OPERATOR: Color = Color::Rgb(212, 212, 212);   // White for operators

    // Styles
    pub fn header() -> Style {
        Style::default()
            .fg(Self::GOLD)
            .bg(Self::PRIMARY_DARK)
            .add_modifier(Modifier::BOLD)
    }

    pub fn title() -> Style {
        Style::default()
            .fg(Self::GOLD)
            .add_modifier(Modifier::BOLD)
    }

    pub fn active_border() -> Style {
        Style::default().fg(Self::PRIMARY_LIGHT)
    }

    pub fn inactive_border() -> Style {
        Style::default().fg(Self::TEXT_MUTED)
    }

    pub fn normal_text() -> Style {
        Style::default().fg(Self::TEXT)
    }

    pub fn dim_text() -> Style {
        Style::default().fg(Self::TEXT_DIM)
    }

    pub fn muted_text() -> Style {
        Style::default().fg(Self::TEXT_MUTED)
    }

    pub fn selected() -> Style {
        Style::default()
            .fg(Self::TEXT)
            .bg(Self::PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    pub fn highlighted() -> Style {
        Style::default()
            .fg(Self::TEXT)
            .bg(Self::BG_HIGHLIGHT)
    }

    pub fn success() -> Style {
        Style::default().fg(Self::SUCCESS)
    }

    pub fn error() -> Style {
        Style::default()
            .fg(Self::ERROR)
            .add_modifier(Modifier::BOLD)
    }

    pub fn warning() -> Style {
        Style::default().fg(Self::WARNING)
    }

    pub fn info() -> Style {
        Style::default().fg(Self::INFO)
    }

    pub fn status_bar() -> Style {
        Style::default()
            .fg(Self::TEXT)
            .bg(Self::PRIMARY_DARK)
    }

    pub fn mode_normal() -> Style {
        Style::default()
            .fg(Color::Black)
            .bg(Self::PRIMARY_LIGHT)
            .add_modifier(Modifier::BOLD)
    }

    pub fn mode_insert() -> Style {
        Style::default()
            .fg(Color::Black)
            .bg(Self::GOLD)
            .add_modifier(Modifier::BOLD)
    }

    pub fn mode_command() -> Style {
        Style::default()
            .fg(Color::Black)
            .bg(Self::INFO)
            .add_modifier(Modifier::BOLD)
    }

    pub fn null_value() -> Style {
        Style::default()
            .fg(Self::TEXT_MUTED)
            .add_modifier(Modifier::ITALIC)
    }

    pub fn primary_key() -> Style {
        Style::default()
            .fg(Self::GOLD)
            .add_modifier(Modifier::BOLD)
    }

    pub fn table_header() -> Style {
        Style::default()
            .fg(Self::GOLD)
            .bg(Self::PRIMARY_DARK)
            .add_modifier(Modifier::BOLD)
    }

    pub fn table_row_alt() -> Style {
        Style::default()
            .fg(Self::TEXT)
            .bg(Self::BG_PANEL)
    }

    pub fn popup() -> Style {
        Style::default()
            .fg(Self::TEXT)
            .bg(Self::BG_PANEL)
    }

    pub fn popup_border() -> Style {
        Style::default().fg(Self::GOLD)
    }

    // Data type colors for column headers
    pub fn type_int() -> Style {
        Style::default().fg(Color::Rgb(100, 180, 255)) // Blue for integers
    }

    pub fn type_float() -> Style {
        Style::default().fg(Color::Rgb(181, 206, 168)) // Green for floats
    }

    pub fn type_string() -> Style {
        Style::default().fg(Color::Rgb(206, 145, 120)) // Orange for strings
    }

    pub fn type_datetime() -> Style {
        Style::default().fg(Color::Rgb(197, 134, 192)) // Purple for dates
    }

    pub fn type_binary() -> Style {
        Style::default().fg(Color::Rgb(150, 150, 150)) // Gray for binary
    }

    pub fn type_bool() -> Style {
        Style::default().fg(Color::Rgb(255, 200, 100)) // Yellow for bool
    }

    // Row number column
    pub fn row_number() -> Style {
        Style::default()
            .fg(Self::TEXT_MUTED)
            .bg(Self::BG_PANEL)
    }

    // Execution stats
    pub fn stats_label() -> Style {
        Style::default().fg(Self::TEXT_DIM)
    }

    pub fn stats_value() -> Style {
        Style::default()
            .fg(Self::SUCCESS)
            .add_modifier(Modifier::BOLD)
    }
}
