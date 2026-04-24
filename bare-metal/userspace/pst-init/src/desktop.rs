use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use crate::keyboard::Keyboard;
use crate::serial_print;

struct Window {
    title: String,
    doc: Vec<String>,
    line: String,
}

impl Window {
    fn new(title: &str) -> Self {
        Self { title: String::from(title), doc: Vec::new(), line: String::new() }
    }

    fn markout(&self) -> String {
        if self.doc.is_empty() && self.line.is_empty() {
            return format!("| (empty)");
        }
        self.doc.join("\n")
    }
}

pub fn run(kb: &Keyboard) {
    let mut windows = Vec::new();
    windows.push(Window::new("Terminal"));
    windows.push(Window::new("Scratch"));
    let mut focused: usize = 0;

    // Seed the terminal window with a welcome message
    windows[0].doc.push(String::from("| Welcome to PST OS"));
    windows[0].doc.push(String::from("| Type Markout. Tab switches windows."));

    render_desktop(&windows, focused);
    print_prompt(&windows[focused]);

    loop {
        let ch = kb.read_key();

        if ch == b'\t' {
            focused = (focused + 1) % windows.len();
            render_desktop(&windows, focused);
            print_prompt(&windows[focused]);
            continue;
        }

        let win = &mut windows[focused];

        if ch == b'\n' {
            serial_print("\n");

            if win.line.is_empty() {
                if !win.doc.is_empty() {
                    win.doc.clear();
                }
                render_desktop(&windows, focused);
                print_prompt(&windows[focused]);
                continue;
            }

            win.doc.push(win.line.clone());
            win.line.clear();

            render_desktop(&windows, focused);
            print_prompt(&windows[focused]);
        } else if ch == 0x08 {
            if !win.line.is_empty() {
                win.line.pop();
                serial_print("\x08 \x08");
            }
        } else {
            win.line.push(ch as char);
            unsafe { crate::debug_putchar(ch) };
        }
    }
}

fn render_desktop(windows: &[Window], focused: usize) {
    let mut doc = String::new();

    // Status bar
    doc.push_str("@card\n");
    doc.push_str("| ");
    for (i, w) in windows.iter().enumerate() {
        if i == focused {
            doc.push_str(&format!("[*{}*]", w.title));
        } else {
            doc.push_str(&format!(" {} ", w.title));
        }
        if i < windows.len() - 1 { doc.push_str(" | "); }
    }
    doc.push('\n');
    doc.push_str("@end card\n");

    // Windows
    for (i, w) in windows.iter().enumerate() {
        let border = if i == focused { "=" } else { "-" };
        doc.push_str("@card\n");
        doc.push_str(&format!("| {} {}\n", w.title, border.repeat(40 - w.title.len())));
        let content = w.markout();
        for line in content.lines() {
            doc.push_str(line);
            doc.push('\n');
        }
        doc.push_str("@end card\n");
    }

    let output = pst_terminal::render(&doc, 80, 24);
    serial_print("\x1b[2J\x1b[H");
    serial_print(&output);
}

fn print_prompt(win: &Window) {
    if win.doc.is_empty() {
        serial_print(&format!("{}> ", win.title));
    } else {
        serial_print("  ..> ");
    }
}
