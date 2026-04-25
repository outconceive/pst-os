use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use crate::keyboard::{self, Keyboard};
use crate::serial_print;
use crate::storage::Storage;

const DEFAULT_INDEX: &str = "\
@card
| PST OS — Page Index
| ====================
|
| dt://pst/welcome    Welcome page
| dt://pst/about      About PST OS
@end card";

const WELCOME_PAGE: &str = "\
@card
| Welcome to PST OS
| ==================
|
@parametric
| {label:title \"Parallel String Theory OS\"}
| {label:ver \"v0.1 — seL4 x86_64\" center-x:title gap-y:8}
@end parametric
|
| One primitive. One loop. One language. Every surface.
|
| Navigation:
|   Enter a dt:// URL to open a page
|   b = back   q = quit to desktop
@end card";

const ABOUT_PAGE: &str = "\
@card
| About PST OS
| =============
|
| PST OS is built on the parallel strings principle:
| position is identity, append-only mutation,
| constraint-solved ordering.
|
| The same model that describes a UI component
| describes a process, a file, a network message,
| and a point in time.
|
| Built on seL4 — formally verified, capability-isolated.
| No Wayland. No X11. No display server.
| Markout all the way down.
@end card";

pub fn run(kb: &Keyboard, store: &mut Option<Storage>) -> BrowserAction {
    // Seed default pages if storage exists and no index yet
    if let Some(ref mut s) = store {
        if s.load_file("/pst/index.md").is_none() {
            s.save_file("/pst/index.md", DEFAULT_INDEX);
            s.save_file("/pst/welcome.md", WELCOME_PAGE);
            s.save_file("/pst/about.md", ABOUT_PAGE);
            serial_print("[browser] Seeded /pst/ pages\n");
        }
    }

    let mut history: Vec<String> = Vec::new();
    let mut url = String::from("dt://pst/welcome");
    let mut url_input = String::new();

    navigate(store, &url, &mut history);

    loop {
        let ch = kb.read_key();

        match ch {
            b'q' => return BrowserAction::Quit,

            b'b' => {
                if history.len() > 1 {
                    history.pop();
                    if let Some(prev) = history.last() {
                        url = prev.clone();
                        render_page(store, &url);
                    }
                }
            }

            // 'g' = go to URL (enter URL mode)
            b'g' => {
                serial_print("\r\n\x1b[7m URL: \x1b[0m ");
                url_input.clear();
                loop {
                    let c = kb.read_key();
                    if c == b'\n' {
                        serial_print("\n");
                        break;
                    } else if c == 0x08 {
                        if !url_input.is_empty() {
                            url_input.pop();
                            serial_print("\x08 \x08");
                        }
                    } else if c < 0x80 {
                        url_input.push(c as char);
                        unsafe { crate::debug_putchar(c) };
                    }
                }
                if !url_input.is_empty() {
                    url = url_input.clone();
                    navigate(store, &url, &mut history);
                }
            }

            // 'i' = show index
            b'i' => {
                url = String::from("dt://pst/index.md");
                navigate(store, &url, &mut history);
            }

            // 'l' = list files
            b'l' => {
                serial_print("\x1b[2J\x1b[H");
                serial_print("\x1b[1m  Files on disk:\x1b[0m\r\n\r\n");
                if let Some(ref mut s) = store {
                    let files = s.list_files();
                    if files.is_empty() {
                        serial_print("  (none)\r\n");
                    }
                    for f in &files {
                        serial_print("  dt://");
                        serial_print(f);
                        serial_print("\r\n");
                    }
                } else {
                    serial_print("  No storage device\r\n");
                }
                serial_print("\r\n\x1b[2m  g=go  b=back  q=quit  i=index\x1b[0m\r\n");
            }

            _ => {}
        }
    }
}

fn navigate(store: &mut Option<Storage>, url: &str, history: &mut Vec<String>) {
    history.push(String::from(url));
    render_page(store, url);
}

fn render_page(store: &mut Option<Storage>, url: &str) {
    let path = resolve_url(url);

    let content = if let Some(ref mut s) = store {
        s.load_file(&path)
    } else {
        None
    };

    serial_print("\x1b[2J\x1b[H");

    // URL bar
    serial_print("\x1b[7m ");
    serial_print(url);
    let pad = 79usize.saturating_sub(url.len() + 1);
    for _ in 0..pad { serial_print(" "); }
    serial_print("\x1b[0m\r\n");

    match content {
        Some(text) => {
            if path.ends_with(".md") {
                let rendered = pst_terminal::render(&text, 80, 22);
                serial_print(&rendered);
            } else {
                serial_print(&text);
                serial_print("\r\n");
            }
        }
        None => {
            serial_print("\r\n  \x1b[31mPage not found:\x1b[0m ");
            serial_print(&path);
            serial_print("\r\n\r\n  Available commands:\r\n");
            serial_print("    g = go to URL\r\n");
            serial_print("    l = list files\r\n");
            serial_print("    i = index\r\n");
        }
    }

    serial_print("\r\n\x1b[2m  g=go  b=back  q=quit  l=list  i=index\x1b[0m\r\n");
}

fn resolve_url(url: &str) -> String {
    if let Some(path) = url.strip_prefix("dt://") {
        // dt://pst/welcome → /pst/welcome.md (add .md if no extension)
        let full = format!("/{}", path);
        if full.contains('.') {
            full
        } else {
            format!("{}.md", full)
        }
    } else {
        String::from(url)
    }
}

pub enum BrowserAction {
    Quit,
}
