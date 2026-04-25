use alloc::format;
use alloc::string::String;

pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const ITALIC: &str = "\x1b[3m";
pub const REVERSE: &str = "\x1b[7m";

pub fn fg(r: u8, g: u8, b: u8) -> String { format!("\x1b[38;2;{};{};{}m", r, g, b) }
pub fn bg(r: u8, g: u8, b: u8) -> String { format!("\x1b[48;2;{};{};{}m", r, g, b) }

pub fn cursor_to(row: usize, col: usize) -> String { format!("\x1b[{};{}H", row + 1, col + 1) }
pub fn clear_screen() -> String { String::from("\x1b[2J") }
pub fn hide_cursor() -> String { String::from("\x1b[?25l") }
pub fn show_cursor() -> String { String::from("\x1b[?25h") }

pub fn theme_style(class: &str) -> &'static str {
    if class.contains("mc-primary") { "\x1b[38;2;59;130;246m" }     // blue
    else if class.contains("mc-secondary") { "\x1b[38;2;107;114;128m" } // gray
    else if class.contains("mc-danger") { "\x1b[38;2;239;68;68m" }   // red
    else if class.contains("mc-warning") { "\x1b[38;2;245;158;11m" }  // amber
    else if class.contains("mc-info") { "\x1b[38;2;6;182;212m" }     // cyan
    else if class.contains("mc-ghost") { "\x1b[2m" }                 // dim
    else { "" }
}
