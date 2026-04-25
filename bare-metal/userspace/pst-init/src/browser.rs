use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use crate::keyboard::Keyboard;
use crate::serial_print;
use crate::storage::Storage;
use crate::net::VirtioNet;

const PROXY_IP: [u8; 4] = [10, 0, 2, 2];
const PROXY_PORT: u16 = 8080;

const DEFAULT_INDEX: &str = "\
@card
| PST OS — Page Index
| ====================
|
| dt://pst/welcome    Welcome page
| dt://pst/about      About PST OS
| gh://outconceive/pst-os/main/README.md
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
|   g = enter URL (dt:// or gh://)
|   b = back   q = quit to desktop
|   l = list files   i = index
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

pub fn run_with_ps2(ps2: &mut crate::ps2::Ps2, store: &mut Option<Storage>, net: &mut Option<VirtioNet>) -> BrowserAction {
    // Adapter: read keys from ps2
    run_inner(|| { match ps2.read_event() { crate::ps2::InputEvent::Key(k) => k, _ => 0 } }, store, net)
}

fn run_inner(mut read_key: impl FnMut() -> u8, store: &mut Option<Storage>, net: &mut Option<VirtioNet>) -> BrowserAction {
    if let Some(ref mut s) = store {
        if s.load_file("/pst/index.md").is_none() {
            s.save_file("/pst/index.md", DEFAULT_INDEX);
            s.save_file("/pst/welcome.md", WELCOME_PAGE);
            s.save_file("/pst/about.md", ABOUT_PAGE);
            serial_print("[browser] Seeded /pst/ pages\n");
        }
    }

    let mut history: Vec<String> = Vec::new();
    let mut url_input = String::new();

    let url = String::from("dt://pst/welcome");
    navigate(store, net, &url, &mut history);

    loop {
        let ch = read_key();

        match ch {
            b'q' => return BrowserAction::Quit,

            b'b' => {
                if history.len() > 1 {
                    history.pop();
                    if let Some(prev) = history.last().cloned() {
                        render_page(store, net, &prev);
                    }
                }
            }

            b'g' => {
                serial_print("\r\n\x1b[7m URL: \x1b[0m ");
                url_input.clear();
                loop {
                    let c = read_key();
                    if c == b'\n' { serial_print("\n"); break; }
                    else if c == 0x08 {
                        if !url_input.is_empty() { url_input.pop(); serial_print("\x08 \x08"); }
                    } else if c < 0x80 {
                        url_input.push(c as char);
                        unsafe { crate::debug_putchar(c) };
                        crate::vgacon::putchar(c);
                    }
                }
                if !url_input.is_empty() {
                    let u = url_input.clone();
                    navigate(store, net, &u, &mut history);
                }
            }

            b'i' => {
                let u = String::from("dt://pst/index.md");
                navigate(store, net, &u, &mut history);
            }

            b'l' => {
                serial_print("\x1b[2J\x1b[H");
                serial_print("\x1b[1m  Files on disk:\x1b[0m\r\n\r\n");
                if let Some(ref mut s) = store {
                    for f in &s.list_files() {
                        serial_print("  dt://");
                        serial_print(f);
                        serial_print("\r\n");
                    }
                }
                if net.is_some() {
                    serial_print("\r\n  \x1b[32mNetwork available\x1b[0m — gh:// URLs work\r\n");
                }
                serial_print("\r\n\x1b[2m  g=go  b=back  q=quit  i=index\x1b[0m\r\n");
            }

            _ => {}
        }
    }
}

fn navigate(store: &mut Option<Storage>, net: &mut Option<VirtioNet>, url: &str, history: &mut Vec<String>) {
    history.push(String::from(url));
    render_page(store, net, url);
}

fn render_page(store: &mut Option<Storage>, net: &mut Option<VirtioNet>, url: &str) {
    serial_print("\x1b[2J\x1b[H");

    // URL bar
    serial_print("\x1b[7m ");
    serial_print(url);
    let pad = 79usize.saturating_sub(url.len() + 1);
    for _ in 0..pad { serial_print(" "); }
    serial_print("\x1b[0m\r\n");

    let content = fetch(store, net, url);

    match content {
        Some(text) => {
            if url.ends_with(".md") || url.starts_with("gh://") || url.starts_with("dt://") {
                let rendered = pst_terminal::render(&text, 80, 22);
                serial_print(&rendered);
            } else {
                serial_print(&text);
                serial_print("\r\n");
            }
        }
        None => {
            serial_print("\r\n  \x1b[31mPage not found:\x1b[0m ");
            serial_print(url);
            serial_print("\r\n\r\n");
            if url.starts_with("gh://") && net.is_none() {
                serial_print("  Network not available. Run gh-proxy.py on host.\r\n");
            }
        }
    }

    serial_print("\r\n\x1b[2m  g=go  b=back  q=quit  l=list  i=index\x1b[0m\r\n");
}

fn fetch(store: &mut Option<Storage>, net: &mut Option<VirtioNet>, url: &str) -> Option<String> {
    if let Some(path) = url.strip_prefix("dt://") {
        let full = format!("/{}", path);
        let full = if full.contains('.') { full } else { format!("{}.md", full) };
        store.as_mut()?.load_file(&full)
    } else if let Some(path) = url.strip_prefix("gh://") {
        // gh://user/repo/branch/file → GET /user/repo/branch/file from proxy
        let http_path = format!("/{}", path);
        serial_print("  \x1b[33mFetching from GitHub...\x1b[0m\r\n");
        let result = crate::net::http_get(net.as_mut()?, PROXY_IP, PROXY_PORT, &http_path);
        result
    } else {
        None
    }
}

pub enum BrowserAction {
    Quit,
}
