use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use crate::serial_print;

pub struct CodeView {
    pub lines: Vec<String>,
    pub output: Vec<String>,
    pub current: usize,
}

impl CodeView {
    pub fn new(source: &str, output: &[&str]) -> Self {
        Self {
            lines: source.lines().map(String::from).collect(),
            output: output.iter().map(|s| String::from(*s)).collect(),
            current: 0,
        }
    }

    pub fn step_forward(&mut self) {
        if self.current < self.lines.len().saturating_sub(1) {
            self.current += 1;
        }
    }

    pub fn step_back(&mut self) {
        if self.current > 0 {
            self.current -= 1;
        }
    }

    pub fn render(&self) -> String {
        let mut out = String::new();

        out.push_str("\x1b[2J\x1b[H");
        out.push_str("\x1b[1;33m");
        out.push_str("  Code Stepper — ↑/↓ step, q quit");
        out.push_str("\x1b[0m\r\n\r\n");

        // Two-pane header
        out.push_str("\x1b[1m");
        out.push_str("  Source");
        for _ in 0..30 { out.push(' '); }
        out.push_str("│ Output");
        out.push_str("\x1b[0m\r\n");

        out.push_str("  ");
        for _ in 0..37 { out.push('─'); }
        out.push('┼');
        for _ in 0..38 { out.push('─'); }
        out.push_str("\r\n");

        let max_lines = self.lines.len().max(self.output.len());
        let output_visible = visible_output(self.current, &self.lines, &self.output);

        for i in 0..max_lines {
            // Left pane: source code
            out.push_str("  ");
            if i < self.lines.len() {
                let is_current = i == self.current;

                // Line number
                if is_current {
                    out.push_str("\x1b[7;1m"); // reverse + bold
                } else {
                    out.push_str("\x1b[2m"); // dim
                }
                let num = format!("{:>3} ", i + 1);
                out.push_str(&num);

                if is_current {
                    out.push_str("\x1b[7m"); // reverse (no bold for code)
                } else {
                    out.push_str("\x1b[0m");
                }

                let highlighted = highlight_rust(&self.lines[i]);
                let display = if is_current {
                    // In reverse mode, strip color codes and just show plain
                    let plain = strip_ansi(&self.lines[i]);
                    let padded = pad_to(&plain, 33);
                    format!("\x1b[7m{}\x1b[0m", padded)
                } else {
                    let padded = pad_to_ansi(&highlighted, 33);
                    padded
                };
                out.push_str(&display);
            } else {
                out.push_str(&pad_to("", 37));
            }

            // Separator
            out.push_str(" │ ");

            // Right pane: output
            if i < output_visible.len() {
                out.push_str("\x1b[32m"); // green
                out.push_str(&output_visible[i]);
                out.push_str("\x1b[0m");
            }

            out.push_str("\r\n");
        }

        out
    }
}

fn visible_output(current_line: usize, source: &[String], output: &[String]) -> Vec<String> {
    if output.is_empty() || source.is_empty() { return Vec::new(); }
    // Integer math: proportional mapping of source lines to output lines
    let visible = ((current_line + 1) * output.len() + source.len() - 1) / source.len();
    output[..visible.min(output.len())].to_vec()
}

fn highlight_rust(line: &str) -> String {
    let keywords = [
        "fn", "let", "mut", "if", "else", "for", "while", "loop", "match",
        "return", "use", "pub", "struct", "enum", "impl", "self", "Self",
        "const", "static", "mod", "crate", "super", "trait", "where",
        "async", "await", "move", "ref", "type", "as", "in", "true", "false",
    ];

    let mut out = String::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Comments
        if i + 1 < len && chars[i] == '/' && chars[i + 1] == '/' {
            out.push_str("\x1b[2;3m"); // dim italic
            while i < len { out.push(chars[i]); i += 1; }
            out.push_str("\x1b[0m");
            continue;
        }

        // Strings
        if chars[i] == '"' {
            out.push_str("\x1b[33m"); // yellow
            out.push(chars[i]); i += 1;
            while i < len && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < len { out.push(chars[i]); i += 1; }
                out.push(chars[i]); i += 1;
            }
            if i < len { out.push(chars[i]); i += 1; }
            out.push_str("\x1b[0m");
            continue;
        }

        // Numbers
        if chars[i].is_ascii_digit() && (i == 0 || !chars[i - 1].is_ascii_alphanumeric()) {
            out.push_str("\x1b[36m"); // cyan
            while i < len && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == '_') {
                out.push(chars[i]); i += 1;
            }
            out.push_str("\x1b[0m");
            continue;
        }

        // Identifiers / keywords
        if chars[i].is_ascii_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') { i += 1; }
            let word: String = chars[start..i].iter().collect();
            if keywords.contains(&word.as_str()) {
                out.push_str("\x1b[35m"); // magenta
                out.push_str(&word);
                out.push_str("\x1b[0m");
            } else if word.chars().next().map_or(false, |c| c.is_uppercase()) {
                out.push_str("\x1b[34m"); // blue for types
                out.push_str(&word);
                out.push_str("\x1b[0m");
            } else {
                out.push_str(&word);
            }
            continue;
        }

        // Macros (word!)
        out.push(chars[i]);
        i += 1;
    }

    out
}

fn pad_to(s: &str, width: usize) -> String {
    let len = s.len();
    if len >= width {
        String::from(&s[..width])
    } else {
        let mut out = String::from(s);
        for _ in 0..(width - len) { out.push(' '); }
        out
    }
}

fn pad_to_ansi(s: &str, width: usize) -> String {
    let visible = visible_len(s);
    if visible >= width {
        let mut out = String::from(s);
        out.push_str("\x1b[0m");
        out
    } else {
        let mut out = String::from(s);
        for _ in 0..(width - visible) { out.push(' '); }
        out.push_str("\x1b[0m");
        out
    }
}

fn visible_len(s: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' { in_escape = true; }
        else if in_escape { if c == 'm' { in_escape = false; } }
        else { len += 1; }
    }
    len
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' { in_escape = true; }
        else if in_escape { if c == 'm' { in_escape = false; } }
        else { out.push(c); }
    }
    out
}
