use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use crate::ps2::{self, Ps2, InputEvent};
use crate::serial_print;
use crate::storage::Storage;
use crate::codeview::CodeView;
use crate::editor::{Editor, EditorAction};
use crate::browser;
use crate::convergence;

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

pub fn run(ps2: &mut Ps2, mut store: Option<Storage>, mut net: Option<crate::net::VirtioNet>, fb_vaddr: u64) {
    let mut windows = Vec::new();

    let restored = store.as_mut().and_then(|s| s.load_desktop());
    if let Some(saved) = restored {
        for (title, lines) in saved {
            let mut w = Window::new(&title);
            w.doc = lines;
            windows.push(w);
        }
        serial_print("[desktop] Restored from disk\n");
    }

    if windows.is_empty() {
        windows.push(Window::new("Terminal"));
        windows.push(Window::new("Scratch"));
        windows[0].doc.push(String::from("| Welcome to PST OS"));
    }

    let mut focused: usize = 0;
    let mut codeview: Option<CodeView> = None;
    let mut editor: Option<Editor> = None;

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
                        let ed = Editor::new("untitled.txt");
                        serial_print(&ed.render());
                        editor = Some(ed);
                    } else if x >= 84 && x < 164 {
                        let ed = Editor::new("untitled.md");
                        serial_print(&ed.render());
                        editor = Some(ed);
                    } else if x >= 168 && x < 244 {
                        browser::run_with_ps2(ps2, &mut store, &mut net);
                        render_desktop(&windows, focused, fb_vaddr);
                        print_prompt(&windows[focused]);
                    } else if x >= 248 && x < 304 {
                        let cv = CodeView::new(DEMO_SOURCE, DEMO_OUTPUT);
                        serial_print(&cv.render());
                        codeview = Some(cv);
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
                    }
                    continue;
                }

                // Click on status bar (row 0-2) — switch window
                if row <= 2 {
                    focused = (focused + 1) % windows.len();
                    render_desktop(&windows, focused, fb_vaddr);
                    print_prompt(&windows[focused]);
                }
                continue;
            }
            InputEvent::MouseMove { .. } => continue,
        };

        // Editor mode
        if let Some(ref mut ed) = editor {
            match ed.handle_key(ch) {
                EditorAction::Continue => { serial_print(&ed.render()); }
                EditorAction::Save => {
                    if let Some(ref mut s) = store { save_file(s, &ed.filename, &ed.to_text()); }
                    serial_print("[editor] Saved "); serial_print(&ed.filename); serial_print("\n");
                    editor = None;
                    render_desktop(&windows, focused, fb_vaddr);
                    print_prompt(&windows[focused]);
                }
                EditorAction::Quit => {
                    editor = None;
                    render_desktop(&windows, focused, fb_vaddr);
                    print_prompt(&windows[focused]);
                }
            }
            continue;
        }

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

        // F1=editor  F2=markout  F3=browser  F4=code  F5=convergence  F6=form
        match ch {
            ps2::KEY_F1 => { let ed = Editor::new("untitled.txt"); serial_print(&ed.render()); editor = Some(ed); continue; }
            ps2::KEY_F2 => { let ed = Editor::new("untitled.md"); serial_print(&ed.render()); editor = Some(ed); continue; }
            ps2::KEY_F3 => { browser::run_with_ps2(ps2, &mut store, &mut net); render_desktop(&windows, focused, fb_vaddr); print_prompt(&windows[focused]); continue; }
            ps2::KEY_F4 => { let cv = CodeView::new(DEMO_SOURCE, DEMO_OUTPUT); serial_print(&cv.render()); codeview = Some(cv); continue; }
            ps2::KEY_F5 => { convergence::run_with_ps2(ps2); render_desktop(&windows, focused, fb_vaddr); print_prompt(&windows[focused]); continue; }
            ps2::KEY_F6 => { run_form(ps2, fb_vaddr); render_desktop(&windows, focused, fb_vaddr); print_prompt(&windows[focused]); continue; }
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
    use crate::gui_input::InputField;

    let mut fields = [
        InputField::text(60, 120, "Username"),
        InputField::password(60, 152, "Password"),
        InputField::checkbox(60, 184, "Remember me"),
    ];
    let mut focus_idx: usize = 0;
    fields[0].focused = true;

    loop {
        // Clear form area
        if fb_vaddr != 0 {
            let vga = fb_vaddr as *mut u8;
            fill(vga, 0, 0, 640, 448, pst_framebuffer::Color::rgb(30, 30, 30));

            // Title
            draw_text(vga, 60, 60, "PST OS Login", pst_framebuffer::Color::WHITE);
            draw_text(vga, 60, 80, "____________________", pst_framebuffer::Color::rgb(59, 130, 246));

            // Draw fields
            for f in &fields {
                f.draw(fb_vaddr);
            }

            // Submit button
            fill(vga, 60, 224, 120, 28, pst_framebuffer::Color::rgb(59, 130, 246));
            fill(vga, 60, 224, 120, 1, pst_framebuffer::Color::rgb(99, 170, 255));
            fill(vga, 60, 251, 120, 1, pst_framebuffer::Color::rgb(30, 90, 200));
            draw_text(vga, 88, 232, "Sign In", pst_framebuffer::Color::WHITE);

            // Esc hint
            draw_text(vga, 60, 270, "Esc to close  Tab to switch", pst_framebuffer::Color::rgb(100, 100, 100));
        }

        let event = ps2.read_event();
        match event {
            InputEvent::Key(ch) => {
                if ch == 0x1B { return; } // Esc = close
                if ch == b'\t' {
                    fields[focus_idx].focused = false;
                    focus_idx = (focus_idx + 1) % fields.len();
                    fields[focus_idx].focused = true;
                    continue;
                }
                fields[focus_idx].handle_key(ch);
            }
            InputEvent::Click { x, y } => {
                // Click on a field
                for (i, f) in fields.iter().enumerate() {
                    if f.contains(x, y) {
                        fields[focus_idx].focused = false;
                        focus_idx = i;
                        fields[focus_idx].focused = true;
                        if fields[focus_idx].input_type == crate::gui_input::InputType::Checkbox {
                            fields[focus_idx].checked = !fields[focus_idx].checked;
                        }
                        break;
                    }
                }
                // Click submit
                if x >= 60 && x < 180 && y >= 224 && y < 252 {
                    serial_print("[form] Submit: ");
                    serial_print(&fields[0].value);
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
    let buttons: [(usize, usize, &str, Color); 6] = [
        (8,   72,  "Editor",  Color::rgb(59, 130, 246)),
        (84,  80,  "Markout", Color::rgb(16, 185, 129)),
        (168, 76,  "Browser", Color::rgb(245, 158, 11)),
        (248, 56,  "Code",    Color::rgb(139, 92, 246)),
        (308, 56,  "Form",    Color::rgb(236, 72, 153)),
        (368, 56,  "Save",    Color::rgb(107, 114, 128)),
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
