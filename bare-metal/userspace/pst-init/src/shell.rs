use alloc::string::String;
use alloc::vec::Vec;

use crate::keyboard::Keyboard;
use crate::serial_print;

pub fn run(kb: &Keyboard) {
    serial_print("[shell] Markout shell ready. Type Markout, Enter to render, empty line to clear.\n\n");

    let mut doc: Vec<String> = Vec::new();
    let mut line = String::new();

    print_prompt(doc.len());

    loop {
        let ch = kb.read_key();

        if ch == b'\n' {
            serial_print("\n");

            if line.is_empty() {
                if !doc.is_empty() {
                    doc.clear();
                    serial_print("[shell] cleared\n");
                }
                print_prompt(doc.len());
                continue;
            }

            doc.push(line.clone());
            line.clear();

            let markout = doc.join("\n");
            let output = pst_terminal::render(&markout, 80, 24);
            serial_print("\x1b[2J\x1b[H"); // clear screen, cursor home
            serial_print(&output);
            serial_print("\n");

            print_prompt(doc.len());
        } else if ch == 0x08 {
            if !line.is_empty() {
                line.pop();
                serial_print("\x08 \x08");
            }
        } else {
            line.push(ch as char);
            unsafe { crate::debug_putchar(ch) };
        }
    }
}

fn print_prompt(lines: usize) {
    if lines == 0 {
        serial_print("markout> ");
    } else {
        serial_print("    ..> ");
    }
}
