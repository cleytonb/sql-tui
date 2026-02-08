//! Alrajhi Bank SQL Server Terminal UI - Library
//! High-performance database client for enterprise use

#[macro_use]
extern crate rust_i18n;

// Initialize i18n with locales from the "locales" directory
// Fallback to English if translation not found
i18n!("locales", fallback = "en");

pub mod app;
pub mod completion;
pub mod config;
pub mod db;
pub mod sql;
pub mod ui;
pub mod utils;

/// Available locales in the application
const AVAILABLE_LOCALES: &[&str] = &["en", "pt-BR"];

/// Initialize the locale based on config or system settings
pub fn init_locale(config_locale: Option<&str>) {
    let locale = if let Some(loc) = config_locale {
        loc.to_string()
    } else {
        // Detect system locale
        sys_locale::get_locale().unwrap_or_else(|| "en".to_string())
    };
    
    // Normalize locale:
    // - Replace underscore with dash (pt_BR -> pt-BR)
    // - Remove encoding suffix (.UTF-8, .utf8, etc)
    let normalized = locale
        .replace('_', "-")
        .split('.')
        .next()
        .unwrap_or("en")
        .to_string();
    
    // Handle special cases like "C" or "POSIX" which mean default/English
    let normalized = if normalized == "C" || normalized == "POSIX" {
        "en".to_string()
    } else {
        normalized
    };
    
    // Try to find exact match or fallback to base language
    let final_locale = if AVAILABLE_LOCALES.contains(&normalized.as_str()) {
        normalized
    } else if let Some(base) = normalized.split('-').next() {
        // Try base language (e.g., "pt" from "pt-BR")
        if AVAILABLE_LOCALES.contains(&base) {
            base.to_string()
        } else {
            // Check if any available locale starts with the base
            AVAILABLE_LOCALES
                .iter()
                .find(|l| l.starts_with(base))
                .map(|s| s.to_string())
                .unwrap_or_else(|| "en".to_string())
        }
    } else {
        "en".to_string()
    };
    
    // Set the locale
    rust_i18n::set_locale(&final_locale);
}
