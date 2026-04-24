use alloc::format;
use alloc::string::String;

pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const REVERSE: &str = "\x1b[7m";

pub fn fg(r: u8, g: u8, b: u8) -> String { format!("\x1b[38;2;{};{};{}m", r, g, b) }
pub fn bg(r: u8, g: u8, b: u8) -> String { format!("\x1b[48;2;{};{};{}m", r, g, b) }

pub fn cursor_to(row: usize, col: usize) -> String { format!("\x1b[{};{}H", row + 1, col + 1) }
pub fn clear_screen() -> String { String::from("\x1b[2J") }
pub fn hide_cursor() -> String { String::from("\x1b[?25l") }
pub fn show_cursor() -> String { String::from("\x1b[?25h") }
