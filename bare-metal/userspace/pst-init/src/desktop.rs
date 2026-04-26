use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use crate::ps2::{self, Ps2, InputEvent};
use crate::serial_print;
use crate::storage::Storage;
use crate::codeview::CodeView;
use crate::editor;
use crate::browser;
use crate::convergence;
use crate::storybook;

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

const DEMO_SOURCE: &str = r#"fn main() {
    let table = ParallelTable::new();

    table.append("cryptod", "new", "system");
    table.append("vfs",     "new", "system");
    table.append("netd",    "new", "system");

    let order = solve_constraints(&table);

    for name in &order {
        println!("Boot: {}", name);
    }

    println!("PST OS ready.");
}"#;

const DEMO_OUTPUT: &[&str] = &[
    "Creating parallel table...",
    "",
    "Appending: cryptod (system)",
    "Appending: vfs (system)",
    "Appending: netd (system)",
    "",
    "Solving constraints...",
    "",
    "Boot: cryptod",
    "Boot: vfs",
    "Boot: netd",
    "",
    "PST OS ready.",
];

const DEFAULT_DESKTOP: &str = "\
Terminal
Scratch";

const DEFAULT_WELCOME: &str = "\
@card
| Welcome to PST OS
| ==================
|
| {label:sub \"Parallel String Theory\" primary}
| {spacer:s1}
| {label:hint \"F1=edit F2=md F3=web F4=code F6=form\"}
|
| {link:docs \"Documentation\" href:/pst/docs}
| {spacer:s2}
| {link:about \"About\" href:/pst/about}
@end card";

const DEFAULT_THEME: &str = "\
bg:30,30,30
fg:255,255,255
accent:59,130,246
danger:239,68,68
success:16,185,129
warning:245,158,11";

pub fn run(ps2: &mut Ps2, mut store: Option<Storage>, mut net: Option<crate::net::VirtioNet>, fb_vaddr: u64) {
    // Seed default config files if storage exists
    if let Some(ref mut s) = store {
        if s.load_file("/pst/desktop.md").is_none() {
            s.save_file("/pst/desktop.md", DEFAULT_DESKTOP);
            s.save_file("/pst/welcome.md", DEFAULT_WELCOME);
            s.save_file("/pst/theme.md", DEFAULT_THEME);
            serial_print("[desktop] Seeded /pst/ config files\n");
        }
    }

    // Load window layout from /pst/desktop.md
    let mut windows = Vec::new();

    let desktop_config = store.as_mut().and_then(|s| s.load_file("/pst/desktop.md"));
    if let Some(config) = desktop_config {
        for name in config.lines() {
            let name = name.trim();
            if !name.is_empty() {
                windows.push(Window::new(name));
            }
        }
        serial_print("[desktop] Layout loaded from /pst/desktop.md\n");
    }

    // Fall back to saved desktop state
    if windows.is_empty() {
        let restored = store.as_mut().and_then(|s| s.load_desktop());
        if let Some(saved) = restored {
            for (title, lines) in saved {
                let mut w = Window::new(&title);
                w.doc = lines;
                windows.push(w);
            }
            serial_print("[desktop] Restored from saved state\n");
        }
    }

    // Final fallback
    if windows.is_empty() {
        windows.push(Window::new("Terminal"));
        windows.push(Window::new("Scratch"));
    }

    // Load welcome content from /pst/welcome.md into first window if empty
    if !windows.is_empty() && windows[0].doc.is_empty() {
        if let Some(welcome) = store.as_mut().and_then(|s| s.load_file("/pst/welcome.md")) {
            for line in welcome.lines() {
                windows[0].doc.push(String::from(line));
            }
        } else {
            windows[0].doc.push(String::from("| Welcome to PST OS"));
        }
    }

    let mut focused: usize = 0;
    let mut codeview: Option<CodeView> = None;

    render_desktop(&windows, focused, fb_vaddr);
    print_prompt(&windows[focused]);

    loop {
        let event = ps2.read_event();

        let ch = match event {
            InputEvent::Key(k) => k,
            InputEvent::Click { x, y } => {
                let row = y / pst_framebuffer::font::GLYPH_HEIGHT;
                let col = x / pst_framebuffer::font::GLYPH_WIDTH;

                // Button bar at y >= 448
                if y >= 448 {
                    // Buttons: Editor(8-88), Markout(96-176), Browser(184-264), Code(272-336), Save(344-408)
                    if x >= 8 && x < 80 {
                        if let Some(text) = editor::run_editor(ps2, fb_vaddr, "untitled.txt", None) {
                            if let Some(ref mut s) = store { save_file(s, "untitled.txt", &text); }
                        }
                        render_desktop(&windows, focused, fb_vaddr);
                        ps2.invalidate_cursor();
                        print_prompt(&windows[focused]);
                    } else if x >= 84 && x < 164 {
                        if let Some(text) = editor::run_editor(ps2, fb_vaddr, "untitled.md", None) {
                            if let Some(ref mut s) = store { save_file(s, "untitled.md", &text); }
                        }
                        render_desktop(&windows, focused, fb_vaddr);
                        ps2.invalidate_cursor();
                        print_prompt(&windows[focused]);
                    } else if x >= 168 && x < 244 {
                        browser::run_with_ps2(ps2, &mut store, &mut net);
                        render_desktop(&windows, focused, fb_vaddr);
                        ps2.invalidate_cursor();
                        print_prompt(&windows[focused]);
                    } else if x >= 248 && x < 304 {
                        let cv = CodeView::new(DEMO_SOURCE, DEMO_OUTPUT);
                        serial_print(&cv.render());
                        codeview = Some(cv);
                        ps2.invalidate_cursor();
                    } else if x >= 308 && x < 364 {
                        // Form
                        run_form(ps2, fb_vaddr);
                        render_desktop(&windows, focused, fb_vaddr);
                        print_prompt(&windows[focused]);
                    } else if x >= 368 && x < 424 {
                        if let Some(ref mut s) = store {
                            let snapshot: Vec<(String, Vec<String>)> = windows.iter()
                                .map(|w| (w.title.clone(), w.doc.clone())).collect();
                            s.save_desktop(&snapshot);
                        }
                    } else if x >= 428 && x < 524 {
                        storybook::run(ps2, fb_vaddr);
                        render_desktop(&windows, focused, fb_vaddr);
                        ps2.invalidate_cursor();
                        print_prompt(&windows[focused]);
                    }
                    continue;
                }

                // Click on status bar (row 0-2) — switch window
                if row <= 2 {
                    focused = (focused + 1) % windows.len();
                    render_desktop(&windows, focused, fb_vaddr);
                    ps2.invalidate_cursor();
                    print_prompt(&windows[focused]);
                }
                continue;
            }
            InputEvent::MouseMove { .. }
            | InputEvent::MouseDown { .. }
            | InputEvent::MouseUp { .. }
            | InputEvent::MouseDrag { .. } => continue,
        };

        // Editor mode removed — run_editor handles its own event loop

        // Code viewer mode
        if let Some(ref mut cv) = codeview {
            match ch {
                b'q' => {
                    codeview = None;
                    render_desktop(&windows, focused, fb_vaddr);
                    print_prompt(&windows[focused]);
                }
                ps2::KEY_DOWN | b'j' => { cv.step_forward(); serial_print(&cv.render()); }
                ps2::KEY_UP | b'k' => { cv.step_back(); serial_print(&cv.render()); }
                _ => {}
            }
            continue;
        }

        // F1=editor  F2=markout  F3=browser  F4=code  F5=convergence  F6=form  F7=storybook
        match ch {
            ps2::KEY_F1 => {
                if let Some(text) = editor::run_editor(ps2, fb_vaddr, "untitled.txt", None) {
                    if let Some(ref mut s) = store { save_file(s, "untitled.txt", &text); }
                }
                render_desktop(&windows, focused, fb_vaddr); ps2.invalidate_cursor(); print_prompt(&windows[focused]); continue;
            }
            ps2::KEY_F2 => {
                if let Some(text) = editor::run_editor(ps2, fb_vaddr, "untitled.md", None) {
                    if let Some(ref mut s) = store { save_file(s, "untitled.md", &text); }
                }
                render_desktop(&windows, focused, fb_vaddr); ps2.invalidate_cursor(); print_prompt(&windows[focused]); continue;
            }
            ps2::KEY_F3 => { browser::run_with_ps2(ps2, &mut store, &mut net); render_desktop(&windows, focused, fb_vaddr); ps2.invalidate_cursor(); print_prompt(&windows[focused]); continue; }
            ps2::KEY_F4 => { let cv = CodeView::new(DEMO_SOURCE, DEMO_OUTPUT); serial_print(&cv.render()); codeview = Some(cv); ps2.invalidate_cursor(); continue; }
            ps2::KEY_F5 => { convergence::run_with_ps2(ps2); render_desktop(&windows, focused, fb_vaddr); ps2.invalidate_cursor(); print_prompt(&windows[focused]); continue; }
            ps2::KEY_F6 => { run_form(ps2, fb_vaddr); render_desktop(&windows, focused, fb_vaddr); ps2.invalidate_cursor(); print_prompt(&windows[focused]); continue; }
            ps2::KEY_F7 => { storybook::run(ps2, fb_vaddr); render_desktop(&windows, focused, fb_vaddr); ps2.invalidate_cursor(); print_prompt(&windows[focused]); continue; }
            _ => {}
        }

        if ch == b'\t' {
            focused = (focused + 1) % windows.len();
            render_desktop(&windows, focused, fb_vaddr);
            print_prompt(&windows[focused]);
            continue;
        }

        if ch == 0x1B {
            if let Some(ref mut s) = store {
                let snapshot: Vec<(String, Vec<String>)> = windows.iter()
                    .map(|w| (w.title.clone(), w.doc.clone())).collect();
                s.save_desktop(&snapshot);
            }
            continue;
        }

        let win = &mut windows[focused];

        if ch == b'\n' {
            serial_print("\n");
            if win.line.is_empty() {
                if !win.doc.is_empty() { win.doc.clear(); }
            } else {
                win.doc.push(win.line.clone());
                win.line.clear();
            }
            render_desktop(&windows, focused, fb_vaddr);
            print_prompt(&windows[focused]);
        } else if ch == 0x08 {
            if !win.line.is_empty() { win.line.pop(); serial_print("\x08 \x08"); }
        } else if ch < 0x80 {
            win.line.push(ch as char);
            unsafe { crate::debug_putchar(ch) };
            crate::vgacon::putchar(ch);
        }
    }
}

fn render_desktop(windows: &[Window], focused: usize, _fb_vaddr: u64) {
    let mut doc = String::new();

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

    for (i, w) in windows.iter().enumerate() {
        let border = if i == focused { "=" } else { "-" };
        doc.push_str("@card\n");
        doc.push_str(&format!("| {} {}\n", w.title, border.repeat(40 - w.title.len().min(39))));
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

    // Draw GUI button bar directly to framebuffer
    draw_button_bar(_fb_vaddr);
}

fn run_form(ps2: &mut Ps2, fb_vaddr: u64) {
    if fb_vaddr == 0 { return; }
    use pst_framebuffer::{Framebuffer, Color};

    // State: field values and focus
    let field_names = ["username", "pass", "remember"];
    let field_types = [0u8, 1, 2]; // 0=text, 1=password, 2=checkbox
    let mut values = [String::new(), String::new(), String::new()];
    let mut checked = false;
    let mut focus: usize = 0;

    // Field positions (y coords, computed from card layout)
    let field_x = 32;
    let field_ys = [100, 130, 160];
    let field_w = 200;
    let field_h = 22;
    let btn_y = 200;

    loop {
        // Build Markout with current state injected as text
        let mut fb = Framebuffer::new(640, 480);
        fb.clear(Color::DARK_BG);

        // Title
        fb.draw_text_transparent(field_x, 50, "PST OS Login", Color::WHITE);
        fb.draw_hline(field_x, 66, 160, Color::rgb(59, 130, 246));

        // Render each field
        for i in 0..3 {
            let y = field_ys[i];
            let tab_w = 5;
            let focused = i == focus;

            let (tab_color, tab_hi) = match field_types[i] {
                0 => (Color::rgb(59, 130, 246), Color::rgb(99, 170, 255)),
                1 => (Color::rgb(239, 68, 68), Color::rgb(255, 108, 108)),
                _ => (Color::rgb(16, 185, 129), Color::rgb(56, 225, 169)),
            };

            let bg = if focused { Color::rgb(60, 60, 65) } else { Color::rgb(50, 50, 55) };
            let border = if focused { Color::rgb(90, 90, 100) } else { Color::rgb(70, 70, 75) };

            if field_types[i] == 2 {
                // Checkbox
                fb.fill_rect(field_x, y, 22, field_h, bg);
                fb.fill_rect(field_x, y, tab_w, field_h, tab_color);
                fb.fill_rect(field_x, y, tab_w, 1, tab_hi);
                fb.draw_hline(field_x, y, 22, border);
                fb.draw_hline(field_x, y + field_h - 1, 22, Color::rgb(40, 40, 45));
                fb.fill_rect(field_x + tab_w + 3, y + 3, 14, 14, Color::rgb(40, 40, 45));
                fb.draw_hline(field_x + tab_w + 3, y + 3, 14, Color::rgb(70, 70, 75));
                if checked {
                    let cc = Color::rgb(50, 255, 120);
                    for j in 0..4usize { fb.fill_rect(field_x + tab_w + 6 + j, y + 8 + j, 2, 2, cc); }
                    for j in 0..7usize { fb.fill_rect(field_x + tab_w + 9 + j, y + 11 - j, 2, 2, cc); }
                }
                fb.draw_text_transparent(field_x + 30, y + 5, "Remember me", Color::rgb(200, 200, 200));
            } else {
                // Text/password field
                fb.fill_rect(field_x, y, field_w, field_h, bg);
                fb.fill_rect(field_x, y, tab_w, field_h, tab_color);
                fb.fill_rect(field_x, y, tab_w, 1, tab_hi);
                fb.draw_hline(field_x, y, field_w, border);
                fb.draw_hline(field_x, y + field_h - 1, field_w, Color::rgb(40, 40, 45));

                let text_x = field_x + tab_w + 6;
                let display: String = if field_types[i] == 1 {
                    (0..values[i].len()).map(|_| '*').collect()
                } else {
                    values[i].clone()
                };
                fb.draw_text_transparent(text_x, y + 5, &display, Color::WHITE);

                // Cursor
                if focused {
                    let cx = text_x + display.len() * 8;
                    fb.fill_rect(cx, y + 4, 2, field_h - 8, Color::WHITE);
                }

                // Label
                let label = if field_types[i] == 0 { "Username" } else { "Password" };
                fb.draw_text_transparent(field_x + field_w + 8, y + 5, label, Color::rgb(150, 150, 150));
            }
        }

        // Submit button
        let btn_w = 120;
        let btn_h = 28;
        fb.fill_rect(field_x, btn_y, btn_w, btn_h, Color::rgb(59, 130, 246));
        fb.draw_hline(field_x, btn_y, btn_w, Color::rgb(99, 170, 255));
        fb.draw_hline(field_x, btn_y + btn_h - 1, btn_w, Color::rgb(30, 90, 200));
        fb.draw_text(field_x + 28, btn_y + 8, "Sign In", Color::WHITE, Color::rgb(59, 130, 246));

        fb.draw_text_transparent(field_x, 250, "Esc=close  Tab=next field", Color::rgb(80, 80, 80));

        let vga = fb_vaddr as *mut u8;
        unsafe { core::ptr::copy_nonoverlapping(fb.pixels.as_ptr(), vga, 640 * 4 * 480); }

        // Handle input
        match ps2.read_event() {
            InputEvent::Key(0x1B) => return,
            InputEvent::Key(b'\t') => { focus = (focus + 1) % 3; }
            InputEvent::Key(ch) => {
                if field_types[focus] == 2 {
                    if ch == b' ' || ch == b'\n' { checked = !checked; }
                } else {
                    if ch == 0x08 { values[focus].pop(); }
                    else if ch >= 0x20 && ch < 0x80 { values[focus].push(ch as char); }
                }
            }
            InputEvent::Click { x, y } => {
                for i in 0..3 {
                    if x >= field_x && x < field_x + field_w && y >= field_ys[i] && y < field_ys[i] + field_h {
                        focus = i;
                        if field_types[i] == 2 { checked = !checked; }
                        break;
                    }
                }
                if x >= field_x && x < field_x + btn_w && y >= btn_y && y < btn_y + btn_h {
                    serial_print("[form] Login: ");
                    serial_print(&values[0]);
                    serial_print("\n");
                    return;
                }
            }
            _ => {}
        }
    }
}

fn draw_button_bar(fb_vaddr: u64) {
    if fb_vaddr == 0 { return; }
    use pst_framebuffer::Color;
    let vga = fb_vaddr as *mut u8;
    let w = 640usize;
    let bar_y = 448; // near bottom
    let bar_h = 28;

    // Bar background
    fill(vga, 0, bar_y, w, bar_h, Color::rgb(45, 45, 45));

    // Buttons: x, width, label, color
    let buttons: [(usize, usize, &str, Color); 7] = [
        (8,   72,  "Editor",    Color::rgb(59, 130, 246)),
        (84,  80,  "Markout",   Color::rgb(16, 185, 129)),
        (168, 76,  "Browser",   Color::rgb(245, 158, 11)),
        (248, 56,  "Code",      Color::rgb(139, 92, 246)),
        (308, 56,  "Form",      Color::rgb(236, 72, 153)),
        (368, 56,  "Save",      Color::rgb(107, 114, 128)),
        (428, 96,  "Storybook", Color::rgb(234, 88, 12)),
    ];

    for (bx, bw, label, color) in &buttons {
        // Button body
        fill(vga, *bx, bar_y + 4, *bw, 20, *color);
        // Highlight edge (top)
        fill(vga, *bx, bar_y + 4, *bw, 1, lighten(*color));
        // Shadow edge (bottom)
        fill(vga, *bx, bar_y + 23, *bw, 1, darken(*color));
        // Text centered
        let tx = bx + (bw - label.len() * 8) / 2;
        draw_text(vga, tx, bar_y + 10, label, Color::WHITE);
    }
}

fn fill(vga: *mut u8, x: usize, y: usize, w: usize, h: usize, c: pst_framebuffer::Color) {
    for dy in 0..h {
        for dx in 0..w {
            let px = x + dx;
            let py = y + dy;
            if px < 640 && py < 480 {
                let off = (py * 640 + px) * 4;
                unsafe {
                    *vga.add(off) = c.b;
                    *vga.add(off + 1) = c.g;
                    *vga.add(off + 2) = c.r;
                    *vga.add(off + 3) = 0xFF;
                }
            }
        }
    }
}

fn draw_text(vga: *mut u8, x: usize, y: usize, s: &str, fg: pst_framebuffer::Color) {
    let mut cx = x;
    for ch in s.bytes() {
        let glyph = pst_framebuffer::font::glyph(ch);
        for gy in 0..pst_framebuffer::font::GLYPH_HEIGHT {
            let bits = glyph[gy];
            for gx in 0..pst_framebuffer::font::GLYPH_WIDTH {
                if bits & (0x80 >> gx) != 0 {
                    let px = cx + gx;
                    let py = y + gy;
                    if px < 640 && py < 480 {
                        let off = (py * 640 + px) * 4;
                        unsafe {
                            *vga.add(off) = fg.b;
                            *vga.add(off + 1) = fg.g;
                            *vga.add(off + 2) = fg.r;
                            *vga.add(off + 3) = 0xFF;
                        }
                    }
                }
            }
        }
        cx += pst_framebuffer::font::GLYPH_WIDTH;
    }
}

fn lighten(c: pst_framebuffer::Color) -> pst_framebuffer::Color {
    pst_framebuffer::Color::rgb(c.r.saturating_add(40), c.g.saturating_add(40), c.b.saturating_add(40))
}

fn darken(c: pst_framebuffer::Color) -> pst_framebuffer::Color {
    pst_framebuffer::Color::rgb(c.r.saturating_sub(30), c.g.saturating_sub(30), c.b.saturating_sub(30))
}

fn print_prompt(win: &Window) {
    if win.doc.is_empty() {
        serial_print(&format!("{}> ", win.title));
    } else {
        serial_print("  ..> ");
    }
}

fn save_file(store: &mut Storage, filename: &str, content: &str) {
    use pst_blk::block::BLOCK_SIZE;
    let bytes = content.as_bytes();
    // File header at block 16+ (blocks 0-15 reserved for desktop)
    let mut block = [0u8; BLOCK_SIZE];
    block[0..4].copy_from_slice(b"PSTF");
    let name_bytes = filename.as_bytes();
    let nlen = name_bytes.len().min(59);
    block[4] = nlen as u8;
    block[5..5 + nlen].copy_from_slice(&name_bytes[..nlen]);
    let total = bytes.len();
    block[64] = (total & 0xFF) as u8;
    block[65] = ((total >> 8) & 0xFF) as u8;
    block[66] = ((total >> 16) & 0xFF) as u8;
    store.write_block(16, &block);

    // Content in subsequent blocks
    let mut lba = 17u64;
    let mut offset = 0usize;
    while offset < total {
        block = [0u8; BLOCK_SIZE];
        let chunk = (total - offset).min(BLOCK_SIZE);
        block[..chunk].copy_from_slice(&bytes[offset..offset + chunk]);
        store.write_block(lba, &block);
        offset += chunk;
        lba += 1;
    }
}
