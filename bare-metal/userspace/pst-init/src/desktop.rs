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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DesktopItem {
    None,
    PostIt(usize),   // index into postit_texts
    File(usize),     // index into file_names
    Folder(usize),   // index into folder_names
}

struct DesktopGrid {
    grid_x: i32,
    grid_y: i32,
    cells: Vec<(i32, i32, Vec<DesktopItem>)>,
    postit_texts: Vec<String>,
    file_names: Vec<String>,
    folder_names: Vec<String>,
}

impl DesktopGrid {
    fn new() -> Self {
        Self {
            grid_x: 0,
            grid_y: 0,
            cells: Vec::new(),
            postit_texts: Vec::new(),
            file_names: Vec::new(),
            folder_names: Vec::new(),
        }
    }

    fn current_items(&self) -> &[DesktopItem] {
        for (cx, cy, items) in &self.cells {
            if *cx == self.grid_x && *cy == self.grid_y {
                return items;
            }
        }
        &[]
    }

    fn current_items_mut(&mut self) -> &mut Vec<DesktopItem> {
        let gx = self.grid_x;
        let gy = self.grid_y;
        if let Some(pos) = self.cells.iter().position(|(cx, cy, _)| *cx == gx && *cy == gy) {
            return &mut self.cells[pos].2;
        }
        self.cells.push((gx, gy, Vec::new()));
        let len = self.cells.len();
        &mut self.cells[len - 1].2
    }

    fn add_postit(&mut self) {
        let idx = self.postit_texts.len();
        self.postit_texts.push(String::new());
        self.current_items_mut().push(DesktopItem::PostIt(idx));
    }

    fn add_file(&mut self, name: &str) {
        let idx = self.file_names.len();
        self.file_names.push(String::from(name));
        self.current_items_mut().push(DesktopItem::File(idx));
    }

    fn add_folder(&mut self, name: &str) {
        let idx = self.folder_names.len();
        self.folder_names.push(String::from(name));
        self.current_items_mut().push(DesktopItem::Folder(idx));
    }

    fn is_home(&self) -> bool {
        self.grid_x == 0 && self.grid_y == 0
    }

    fn has_neighbors(&self) -> (bool, bool, bool, bool) {
        // (up, down, left, right) — true if that cell has items or is home
        let check = |dx: i32, dy: i32| -> bool {
            let tx = self.grid_x + dx;
            let ty = self.grid_y + dy;
            if tx == 0 && ty == 0 { return true; }
            self.cells.iter().any(|(cx, cy, items)| *cx == tx && *cy == ty && !items.is_empty())
        };
        (check(0, -1), check(0, 1), check(-1, 0), check(1, 0))
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
Home";

const DEFAULT_WELCOME: &str = "\
| {label:title \"PST OS\" primary lg}
| {label:sub \"Parallel String Theory\" muted}
| {divider:d1}
| {spacer:s0}
| {label:desc \"Everything is a flat parallel string.\"}
| {label:desc2 \"Processes, files, UI, network, scheduling\"}
| {label:desc3 \"all unified in one primitive.\"}
| {spacer:s1}
| {badge:kernel \"seL4\" success}  {badge:lang \"Rust\" primary}  {badge:arch \"x86_64\" warning}  {badge:mode \"no_std\"}
| {spacer:s2}
| {divider:d2}
| {spacer:s3}
| {pill:edit \"F1 Editor\" primary}  {pill:md \"F2 Markout\" success}  {pill:web \"F3 Browser\" warning}
| {pill:code \"F4 Code\" primary}  {pill:story \"F7 Storybook\" danger}
| {spacer:s4}
| {progress:boot}
| {label:status \"System ready\" success}
";

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
    let mut start_open = false;
    let mut add_menu_open = false;
    let mut grid = DesktopGrid::new();
    let mut focused_postit: Option<usize> = None;
    let mut selected_file: Option<usize> = None; // file idx selected for moving
    let mut hovered_item: Option<usize> = None;  // index into current_items

    crate::vgacon::set_enabled(false);
    render_full_desktop(&grid, &windows, fb_vaddr);
    set_desktop_hover(ps2, &grid, false);

    loop {
        let event = ps2.read_event();

        let ch = match event {
            InputEvent::Key(k) => k,
            InputEvent::Click { x, y } => {
                let row = y / pst_framebuffer::font::GLYPH_HEIGHT;
                let col = x / pst_framebuffer::font::GLYPH_WIDTH;

                // FAB button (bottom-left corner, 40x40)
                let fab_x = 8usize;
                let fab_y = 436usize;
                let fab_w = 72usize;
                let fab_h = 36usize;

                if x >= fab_x && x < fab_x + fab_w && y >= fab_y && y < fab_y + fab_h {
                    start_open = !start_open;
                    if start_open {
                        draw_start_tray(fb_vaddr);
                        draw_fab(fb_vaddr, true);
                        set_fab_hover(ps2, true);
                    } else {
                        render_full_desktop(&grid, &windows, fb_vaddr);
                        draw_fab(fb_vaddr, false);
                        set_fab_hover(ps2, false);
                        print_prompt(&windows[focused]);
                    }
                    continue;
                }

                // Start menu tray clicks
                if start_open {
                    let tray_x = 8usize;
                    let tray_w = 160usize;
                    let item_h = 28usize;
                    let tray_bottom = fab_y - 4;

                    let menu_items = start_menu_items();
                    let tray_h = menu_items.len() * item_h + 8;
                    let tray_top = tray_bottom - tray_h;

                    if x >= tray_x && x < tray_x + tray_w && y >= tray_top && y < tray_bottom {
                        let idx = (y - tray_top - 4) / item_h;
                        start_open = false;

                        match idx {
                            0 => { // Editor
                                if let Some(text) = editor::run_editor(ps2, fb_vaddr, "untitled.txt", None) {
                                    if let Some(ref mut s) = store { save_file(s, "untitled.txt", &text); }
                                }
                            }
                            1 => { // Markout
                                if let Some(text) = editor::run_editor(ps2, fb_vaddr, "untitled.md", None) {
                                    if let Some(ref mut s) = store { save_file(s, "untitled.md", &text); }
                                }
                            }
                            2 => { // Browser
                                browser::run_with_ps2(ps2, &mut store, &mut net);
                            }
                            3 => { // Code
                                let cv = CodeView::new(DEMO_SOURCE, DEMO_OUTPUT);
                                serial_print(&cv.render());
                                codeview = Some(cv);
                            }
                            4 => { // Storybook
                                storybook::run(ps2, fb_vaddr);
                            }
                            5 => { // Form
                                run_form(ps2, fb_vaddr);
                            }
                            6 => { // Save
                                if let Some(ref mut s) = store {
                                    let snapshot: Vec<(String, Vec<String>)> = windows.iter()
                                        .map(|w| (w.title.clone(), w.doc.clone())).collect();
                                    s.save_desktop(&snapshot);
                                }
                            }
                            _ => {}
                        }

                        render_full_desktop(&grid, &windows, fb_vaddr);
                        draw_fab(fb_vaddr, false);
                        set_fab_hover(ps2, false);
                        ps2.invalidate_cursor();
                        print_prompt(&windows[focused]);
                        continue;
                    }

                    // Click outside tray — dismiss
                    start_open = false;
                    render_full_desktop(&grid, &windows, fb_vaddr);
                    draw_fab(fb_vaddr, false);
                    set_fab_hover(ps2, false);
                    print_prompt(&windows[focused]);
                    continue;
                }

                // D-pad (bottom-right corner)
                let dpad_cx: usize = 590;
                let dpad_cy: usize = 450;
                let dpad_btn: usize = 20;

                // Up
                if x >= dpad_cx - 10 && x < dpad_cx + 10 && y >= dpad_cy - 30 && y < dpad_cy - 10 {
                    grid.grid_y -= 1;
                    add_menu_open = false;
                    render_full_desktop(&grid, &windows, fb_vaddr);
                    set_desktop_hover(ps2, &grid, false);
                    ps2.invalidate_cursor();
                    continue;
                }
                // Down
                if x >= dpad_cx - 10 && x < dpad_cx + 10 && y >= dpad_cy + 10 && y < dpad_cy + 30 {
                    grid.grid_y += 1;
                    add_menu_open = false;
                    render_full_desktop(&grid, &windows, fb_vaddr);
                    set_desktop_hover(ps2, &grid, false);
                    ps2.invalidate_cursor();
                    continue;
                }
                // Left
                if x >= dpad_cx - 30 && x < dpad_cx - 10 && y >= dpad_cy - 10 && y < dpad_cy + 10 {
                    grid.grid_x -= 1;
                    add_menu_open = false;
                    render_full_desktop(&grid, &windows, fb_vaddr);
                    set_desktop_hover(ps2, &grid, false);
                    ps2.invalidate_cursor();
                    continue;
                }
                // Right
                if x >= dpad_cx + 10 && x < dpad_cx + 30 && y >= dpad_cy - 10 && y < dpad_cy + 10 {
                    grid.grid_x += 1;
                    add_menu_open = false;
                    render_full_desktop(&grid, &windows, fb_vaddr);
                    set_desktop_hover(ps2, &grid, false);
                    ps2.invalidate_cursor();
                    continue;
                }

                // + button (above d-pad)
                if x >= 570 && x < 610 && y >= 400 && y < 420 {
                    add_menu_open = !add_menu_open;
                    if add_menu_open {
                        draw_add_menu(fb_vaddr);
                    } else {
                        render_full_desktop(&grid, &windows, fb_vaddr);
                        set_desktop_hover(ps2, &grid, false);
                    }
                    ps2.invalidate_cursor();
                    continue;
                }

                // Add menu clicks
                if add_menu_open {
                    if x >= 510 && x < 620 {
                        let menu_top: usize = 330;
                        let item_h: usize = 24;
                        if y >= menu_top && y < menu_top + item_h {
                            grid.add_postit();
                            // Auto-focus the new note
                            let new_idx = grid.postit_texts.len() - 1;
                            focused_postit = Some(new_idx);
                        } else if y >= menu_top + item_h && y < menu_top + 2 * item_h {
                            let n = grid.file_names.len() + 1;
                            let label = format!("file_{}.txt", n);
                            grid.add_file(&label);
                        } else if y >= menu_top + 2 * item_h && y < menu_top + 3 * item_h {
                            let n = grid.folder_names.len() + 1;
                            let label = format!("folder_{}", n);
                            grid.add_folder(&label);
                        }
                    }
                    add_menu_open = false;
                    render_full_desktop_with_focus(&grid, &windows, fb_vaddr, focused_postit);
                    set_desktop_hover(ps2, &grid, false);
                    ps2.invalidate_cursor();
                    continue;
                }

                // Click on grid items
                if !grid.is_home() && y >= 50 && y < 430 {
                    if let Some(item_idx) = hit_test_item(&grid, x, y) {
                        let (hit_ix, hit_iy) = item_rect(item_idx);
                        // Copy the item to avoid borrow conflict
                        let item_copy = grid.current_items().get(item_idx).copied();
                        if let Some(item) = item_copy {
                            match item {
                                DesktopItem::PostIt(idx) => {
                                    focused_postit = Some(idx);
                                    selected_file = None;
                                }
                                DesktopItem::File(idx) => {
                                    if x >= hit_ix + 94 && y < hit_iy + 20 && selected_file.is_none() {
                                        let name = grid.file_names[idx].clone();
                                        let content = store.as_mut()
                                            .and_then(|s| s.load_file(&name));
                                        let text = content.as_deref();
                                        if let Some(saved) = editor::run_editor(ps2, fb_vaddr, &name, text) {
                                            if let Some(ref mut s) = store {
                                                save_file(s, &name, &saved);
                                            }
                                        }
                                        selected_file = None;
                                    } else {
                                        selected_file = Some(idx);
                                    }
                                    focused_postit = None;
                                }
                                DesktopItem::Folder(fidx) => {
                                    if selected_file.is_some() && x >= hit_ix + 84 && y < hit_iy + 20 {
                                        if let Some(file_idx) = selected_file {
                                            let folder_name = grid.folder_names.get(fidx)
                                                .cloned().unwrap_or_else(|| String::from("folder"));
                                            if let Some(fname) = grid.file_names.get_mut(file_idx) {
                                                let new_name = format!("{}/{}", folder_name, fname);
                                                *fname = new_name;
                                            }
                                        }
                                        selected_file = None;
                                    }
                                    focused_postit = None;
                                }
                                _ => {
                                    focused_postit = None;
                                    selected_file = None;
                                }
                            }
                        }
                    } else {
                        focused_postit = None;
                        selected_file = None;
                    }
                    render_full_desktop_with_state(&grid, &windows, fb_vaddr, focused_postit, hovered_item, selected_file);
                    set_desktop_hover(ps2, &grid, false);
                    ps2.invalidate_cursor();
                }

                continue;
            }
            InputEvent::MouseDown { .. }
            | InputEvent::MouseUp { .. }
            | InputEvent::MouseDrag { .. } => continue,
            InputEvent::MouseMove { .. } => {
                // Track hover over items
                if !grid.is_home() {
                    let mx = ps2.mouse_x() as usize;
                    let my = ps2.mouse_y() as usize;
                    let new_hover = hit_test_item(&grid, mx, my);
                    if new_hover != hovered_item {
                        hovered_item = new_hover;
                        render_full_desktop_with_state(&grid, &windows, fb_vaddr, focused_postit, hovered_item, selected_file);
                        ps2.invalidate_cursor();
                    }
                }
                continue;
            }
        };

        // Post-it typing mode
        if let Some(pidx) = focused_postit {
            match ch {
                0x1B | b'\n' => {
                    focused_postit = None;
                    render_full_desktop_with_focus(&grid, &windows, fb_vaddr, focused_postit);
                    set_desktop_hover(ps2, &grid, false);
                    ps2.invalidate_cursor();
                }
                0x08 => {
                    if let Some(text) = grid.postit_texts.get_mut(pidx) {
                        text.pop();
                    }
                    render_full_desktop_with_focus(&grid, &windows, fb_vaddr, focused_postit);
                    set_desktop_hover(ps2, &grid, false);
                    ps2.invalidate_cursor();
                }
                c if c >= 0x20 && c < 0x80 => {
                    if let Some(text) = grid.postit_texts.get_mut(pidx) {
                        if text.len() < 60 { text.push(c as char); }
                    }
                    render_full_desktop_with_focus(&grid, &windows, fb_vaddr, focused_postit);
                    set_desktop_hover(ps2, &grid, false);
                    ps2.invalidate_cursor();
                }
                _ => {}
            }
            continue;
        }

        // Code viewer mode
        if let Some(ref mut cv) = codeview {
            match ch {
                b'q' => {
                    codeview = None;
                    render_full_desktop(&grid, &windows, fb_vaddr);
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
                render_full_desktop(&grid, &windows, fb_vaddr); ps2.invalidate_cursor(); print_prompt(&windows[focused]); continue;
            }
            ps2::KEY_F2 => {
                if let Some(text) = editor::run_editor(ps2, fb_vaddr, "untitled.md", None) {
                    if let Some(ref mut s) = store { save_file(s, "untitled.md", &text); }
                }
                render_full_desktop(&grid, &windows, fb_vaddr); ps2.invalidate_cursor(); print_prompt(&windows[focused]); continue;
            }
            ps2::KEY_F3 => { browser::run_with_ps2(ps2, &mut store, &mut net); render_full_desktop(&grid, &windows, fb_vaddr); ps2.invalidate_cursor(); print_prompt(&windows[focused]); continue; }
            ps2::KEY_F4 => { let cv = CodeView::new(DEMO_SOURCE, DEMO_OUTPUT); serial_print(&cv.render()); codeview = Some(cv); ps2.invalidate_cursor(); continue; }
            ps2::KEY_F5 => { convergence::run_with_ps2(ps2); render_full_desktop(&grid, &windows, fb_vaddr); ps2.invalidate_cursor(); print_prompt(&windows[focused]); continue; }
            ps2::KEY_F6 => { run_form(ps2, fb_vaddr); render_full_desktop(&grid, &windows, fb_vaddr); ps2.invalidate_cursor(); print_prompt(&windows[focused]); continue; }
            ps2::KEY_F7 => { storybook::run(ps2, fb_vaddr); render_full_desktop(&grid, &windows, fb_vaddr); ps2.invalidate_cursor(); print_prompt(&windows[focused]); continue; }
            _ => {}
        }

        if ch == b'\t' {
            focused = (focused + 1) % windows.len();
            render_full_desktop(&grid, &windows, fb_vaddr);
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
            render_full_desktop(&grid, &windows, fb_vaddr);
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

fn render_full_desktop(grid: &DesktopGrid, windows: &[Window], fb_vaddr: u64) {
    render_full_desktop_inner(grid, windows, fb_vaddr, None, None, None);
}

fn render_full_desktop_with_focus(grid: &DesktopGrid, windows: &[Window], fb_vaddr: u64, focused_postit: Option<usize>) {
    render_full_desktop_inner(grid, windows, fb_vaddr, focused_postit, None, None);
}

fn render_full_desktop_with_state(grid: &DesktopGrid, windows: &[Window], fb_vaddr: u64, focused_postit: Option<usize>, hovered_item: Option<usize>, selected_file: Option<usize>) {
    render_full_desktop_inner(grid, windows, fb_vaddr, focused_postit, hovered_item, selected_file);
}

fn render_full_desktop_inner(grid: &DesktopGrid, windows: &[Window], fb_vaddr: u64, focused_postit: Option<usize>, hovered_item: Option<usize>, selected_file: Option<usize>) {
    if fb_vaddr == 0 { return; }
    use pst_framebuffer::{Framebuffer, Color};

    let mut fb = Framebuffer::new(640, 480);
    let bg = Color::rgb(15, 15, 22);
    fb.clear(bg);

    // Accent stripe at top
    fb.fill_rect(0, 0, 640, 3, Color::rgb(59, 130, 246));

    let cx: usize = 320;

    if grid.is_home() {
    // Hero section — centered

    // Title: "PST OS" at 3x scale
    let title = "PST OS";
    let title_w = title.len() * 8 * 3;
    let title_x = cx - title_w / 2;
    let title_y: usize = 80;
    for (ci, ch) in title.bytes().enumerate() {
        let glyph = pst_framebuffer::font::glyph(ch);
        let gw = pst_framebuffer::font::GLYPH_WIDTH;
        let gh = pst_framebuffer::font::GLYPH_HEIGHT;
        for gy in 0..gh {
            for gx in 0..gw {
                if glyph[gy] & (0x80 >> gx) != 0 {
                    let px = title_x + ci * 8 * 3 + gx * 3;
                    let py = title_y + gy * 3;
                    fb.fill_rect(px, py, 3, 3, Color::WHITE);
                    fb.fill_rect(px + 1, py, 3, 3, Color::WHITE); // faux bold
                }
            }
        }
    }

    // Subtitle
    let sub = "Parallel String Theory";
    let sub_x = cx - sub.len() * 8 / 2;
    let sub_y = title_y + 56;
    fb.draw_text(sub_x, sub_y, sub, Color::rgb(120, 120, 140), bg);

    // Tagline
    let tag1 = "One primitive. Every subsystem.";
    let tag1_x = cx - tag1.len() * 8 / 2;
    fb.draw_text(tag1_x, sub_y + 30, tag1, Color::rgb(180, 180, 195), bg);

    let tag2 = "Processes, files, UI, network, scheduling";
    let tag2_x = cx - tag2.len() * 8 / 2;
    fb.draw_text(tag2_x, sub_y + 46, tag2, Color::rgb(100, 100, 115), bg);

    // Divider line
    fb.fill_rect(cx - 60, sub_y + 70, 120, 1, Color::rgb(50, 50, 60));

    // Badges row
    let badges: [(&str, Color); 4] = [
        ("seL4", Color::rgb(16, 185, 129)),
        ("Rust", Color::rgb(59, 130, 246)),
        ("x86_64", Color::rgb(245, 158, 11)),
        ("no_std", Color::rgb(139, 92, 246)),
    ];
    let badge_y = sub_y + 90;
    let total_badge_w: usize = badges.iter().map(|(l, _)| l.len() * 8 + 16).sum::<usize>()
        + (badges.len() - 1) * 8;
    let mut bx = cx - total_badge_w / 2;
    for (label, color) in &badges {
        let bw = label.len() * 8 + 16;
        fb.fill_rect(bx, badge_y, bw, 22, *color);
        fb.draw_hline(bx, badge_y, bw, lighten_c(*color));
        fb.draw_hline(bx, badge_y + 21, bw, darken_c(*color));
        fb.draw_text(bx + 8, badge_y + 5, label, Color::WHITE, *color);
        bx += bw + 8;
    }

    // App shortcut pills
    let pills: [(&str, Color); 5] = [
        ("F1 Editor", Color::rgb(59, 130, 246)),
        ("F2 Markout", Color::rgb(16, 185, 129)),
        ("F3 Browser", Color::rgb(245, 158, 11)),
        ("F4 Code", Color::rgb(139, 92, 246)),
        ("F7 Storybook", Color::rgb(234, 88, 12)),
    ];
    let pill_y = badge_y + 44;
    let total_pill_w: usize = pills.iter().map(|(l, _)| l.len() * 8 + 12).sum::<usize>()
        + (pills.len() - 1) * 6;
    let mut px = cx - total_pill_w / 2;
    for (label, color) in &pills {
        let pw = label.len() * 8 + 12;
        let pill_bg = Color::rgb(30, 30, 38);
        fb.fill_rect(px, pill_y, pw, 20, pill_bg);
        fb.fill_rect(px, pill_y, 3, 20, *color);
        fb.draw_text(px + 8, pill_y + 4, label, Color::rgb(180, 180, 190), pill_bg);
        px += pw + 6;
    }

    } else {
        // Non-home view — show grid items
        fb.draw_text(20, 16, &format!("Desktop ({}, {})", grid.grid_x, grid.grid_y),
            Color::rgb(100, 100, 120), bg);
        fb.draw_hline(0, 36, 640, Color::rgb(40, 40, 48));

        let items = grid.current_items();
        if items.is_empty() {
            fb.draw_text(240, 200, "Empty space", Color::rgb(60, 60, 75), bg);
            fb.draw_text(220, 220, "Click + to add items", Color::rgb(50, 50, 65), bg);
        } else {
            let mut ix: usize = 20;
            let mut iy: usize = 50;
            for (item_idx, item) in items.iter().enumerate() {
                let is_hovered = hovered_item == Some(item_idx);
                let is_selected_file_item = if let (Some(sel), DesktopItem::File(fidx)) = (selected_file, item) {
                    sel == *fidx
                } else { false };

                match item {
                    DesktopItem::PostIt(idx) => {
                        let text = grid.postit_texts.get(*idx).map(|s| s.as_str()).unwrap_or("");
                        let is_focused = focused_postit == Some(*idx);
                        let note_bg = if is_focused {
                            Color::rgb(255, 240, 120)
                        } else {
                            Color::rgb(250, 230, 100)
                        };
                        let text_fg = Color::rgb(0, 0, 0);
                        fb.fill_rect(ix, iy, 120, 80, note_bg);
                        fb.fill_rect(ix, iy, 120, 3, Color::rgb(220, 200, 70));
                        if is_focused {
                            fb.draw_hline(ix, iy + 79, 120, Color::rgb(59, 130, 246));
                            fb.draw_hline(ix, iy, 120, Color::rgb(59, 130, 246));
                        }
                        // Render text with word wrap (12 chars per line, 4 lines max)
                        let chars_per_line: usize = 12;
                        let max_lines: usize = 4;
                        let mut ty = iy + 10;
                        let mut pos: usize = 0;
                        for _line in 0..max_lines {
                            if pos >= text.len() { break; }
                            let end = (pos + chars_per_line).min(text.len());
                            let slice = &text[pos..end];
                            fb.draw_text(ix + 8, ty, slice, text_fg, note_bg);
                            ty += 16;
                            pos = end;
                        }
                        // Cursor
                        if is_focused {
                            let cursor_line = text.len() / chars_per_line;
                            let cursor_col = text.len() % chars_per_line;
                            let cur_x = ix + 8 + cursor_col * 8;
                            let cur_y = iy + 10 + cursor_line * 16;
                            if cur_y + 14 < iy + 78 {
                                fb.fill_rect(cur_x, cur_y, 2, 14, Color::rgb(59, 130, 246));
                            }
                        }
                        if text.is_empty() && !is_focused {
                            fb.draw_text(ix + 8, iy + 10, "Click to", Color::rgb(180, 170, 80), note_bg);
                            fb.draw_text(ix + 8, iy + 26, "type...", Color::rgb(180, 170, 80), note_bg);
                        }
                    }
                    DesktopItem::File(idx) => {
                        let name = grid.file_names.get(*idx).map(|s| s.as_str()).unwrap_or("file");
                        let card_bg = if is_selected_file_item {
                            Color::rgb(50, 55, 70)
                        } else { Color::rgb(40, 40, 50) };
                        fb.fill_rect(ix, iy, 120, 80, card_bg);
                        fb.draw_hline(ix, iy, 120, Color::rgb(60, 60, 70));
                        if is_selected_file_item {
                            fb.draw_hline(ix, iy, 120, Color::rgb(59, 130, 246));
                            fb.draw_hline(ix, iy + 79, 120, Color::rgb(59, 130, 246));
                        }
                        fb.fill_rect(ix + 45, iy + 10, 30, 36, Color::rgb(80, 80, 95));
                        fb.fill_rect(ix + 45, iy + 10, 30, 3, Color::rgb(59, 130, 246));
                        let max_chars = 13;
                        let display = if name.len() > max_chars { &name[..max_chars] } else { name };
                        fb.draw_text(ix + 10, iy + 56, display, Color::rgb(180, 180, 190), card_bg);
                        // Hover: show edit icon
                        if is_hovered && selected_file.is_none() {
                            let eb = Color::rgb(59, 130, 246);
                            fb.fill_rect(ix + 94, iy + 4, 22, 16, eb);
                            fb.draw_text(ix + 97, iy + 4, "Ed", Color::WHITE, eb);
                        }
                    }
                    DesktopItem::Folder(idx) => {
                        let name = grid.folder_names.get(*idx).map(|s| s.as_str()).unwrap_or("folder");
                        let card_bg = Color::rgb(40, 40, 50);
                        fb.fill_rect(ix, iy, 120, 80, card_bg);
                        fb.draw_hline(ix, iy, 120, Color::rgb(60, 60, 70));
                        fb.fill_rect(ix + 40, iy + 15, 40, 6, Color::rgb(245, 158, 11));
                        fb.fill_rect(ix + 35, iy + 21, 50, 25, Color::rgb(245, 158, 11));
                        let max_chars = 13;
                        let display = if name.len() > max_chars { &name[..max_chars] } else { name };
                        fb.draw_text(ix + 10, iy + 56, display, Color::rgb(180, 180, 190), card_bg);
                        // If a file is selected, show "move here" target
                        if selected_file.is_some() {
                            let mb = Color::rgb(16, 185, 129);
                            fb.fill_rect(ix + 84, iy + 4, 32, 16, mb);
                            fb.draw_text(ix + 86, iy + 4, "->", Color::WHITE, mb);
                        }
                    }
                    _ => {}
                }
                ix += 140;
                if ix + 120 > 560 {
                    ix = 20;
                    iy += 100;
                }
            }
        }
    }

    // === D-pad (always visible) ===
    let dpad_cx: usize = 590;
    let dpad_cy: usize = 450;
    let btn_color = Color::rgb(50, 50, 60);
    let arrow_color = Color::rgb(160, 160, 175);
    let (has_up, has_down, has_left, has_right) = grid.has_neighbors();

    // Center dot — shows position
    fb.fill_rect(dpad_cx - 8, dpad_cy - 8, 16, 16, Color::rgb(40, 40, 48));
    let pos_str = format!("{},{}", grid.grid_x, grid.grid_y);
    let pos_x = dpad_cx - pos_str.len() * 4;
    fb.draw_text(pos_x, dpad_cy - 4, &pos_str, Color::rgb(100, 100, 115), Color::rgb(40, 40, 48));

    // Up
    fb.fill_rect(dpad_cx - 8, dpad_cy - 28, 16, 16, btn_color);
    fb.draw_text(dpad_cx - 4, dpad_cy - 26, "^", if has_up { Color::rgb(59, 130, 246) } else { arrow_color }, btn_color);
    // Down
    fb.fill_rect(dpad_cx - 8, dpad_cy + 12, 16, 16, btn_color);
    fb.draw_text(dpad_cx - 4, dpad_cy + 14, "v", if has_down { Color::rgb(59, 130, 246) } else { arrow_color }, btn_color);
    // Left
    fb.fill_rect(dpad_cx - 28, dpad_cy - 8, 16, 16, btn_color);
    fb.draw_text(dpad_cx - 26, dpad_cy - 6, "<", if has_left { Color::rgb(59, 130, 246) } else { arrow_color }, btn_color);
    // Right
    fb.fill_rect(dpad_cx + 12, dpad_cy - 8, 16, 16, btn_color);
    fb.draw_text(dpad_cx + 14, dpad_cy - 6, ">", if has_right { Color::rgb(59, 130, 246) } else { arrow_color }, btn_color);

    // + button (non-home only)
    if !grid.is_home() {
        fb.fill_rect(570, 400, 40, 20, Color::rgb(59, 130, 246));
        fb.draw_hline(570, 400, 40, lighten_c(Color::rgb(59, 130, 246)));
        fb.draw_text(584, 404, "+", Color::WHITE, Color::rgb(59, 130, 246));
    }

    // Blit
    let vga = fb_vaddr as *mut u8;
    unsafe { core::ptr::copy_nonoverlapping(fb.pixels.as_ptr(), vga, 640 * 4 * 480); }

    draw_fab(fb_vaddr, false);
}

fn draw_add_menu(fb_vaddr: u64) {
    if fb_vaddr == 0 { return; }
    use pst_framebuffer::Color;
    let vga = fb_vaddr as *mut u8;

    let menu_x: usize = 510;
    let menu_y: usize = 326;
    let menu_w: usize = 110;
    let item_h: usize = 24;
    let menu_h: usize = 3 * item_h + 8;

    fill(vga, menu_x, menu_y, menu_w, menu_h, Color::rgb(35, 35, 42));
    fill(vga, menu_x, menu_y, menu_w, 1, Color::rgb(60, 60, 70));
    fill(vga, menu_x, menu_y, 1, menu_h, Color::rgb(60, 60, 70));
    fill(vga, menu_x + menu_w - 1, menu_y, 1, menu_h, Color::rgb(25, 25, 30));
    fill(vga, menu_x, menu_y + menu_h - 1, menu_w, 1, Color::rgb(25, 25, 30));

    let items: [(&str, pst_framebuffer::Color); 3] = [
        ("Post-it", Color::rgb(250, 230, 100)),
        ("File", Color::rgb(59, 130, 246)),
        ("Folder", Color::rgb(245, 158, 11)),
    ];

    for (i, (label, color)) in items.iter().enumerate() {
        let iy = menu_y + 4 + i * item_h;
        fill(vga, menu_x + 4, iy + 4, 4, item_h - 8, *color);
        draw_text(vga, menu_x + 14, iy + 6, label, Color::rgb(200, 200, 210));
        if i < 2 {
            fill(vga, menu_x + 8, iy + item_h - 1, menu_w - 16, 1, Color::rgb(50, 50, 55));
        }
    }
}

fn hit_test_item(grid: &DesktopGrid, x: usize, y: usize) -> Option<usize> {
    if grid.is_home() || y < 50 || y >= 430 { return None; }
    let items = grid.current_items();
    let mut ix: usize = 20;
    let mut iy: usize = 50;
    for (i, _) in items.iter().enumerate() {
        if x >= ix && x < ix + 120 && y >= iy && y < iy + 80 {
            return Some(i);
        }
        ix += 140;
        if ix + 120 > 560 { ix = 20; iy += 100; }
    }
    None
}

fn item_rect(index: usize) -> (usize, usize) {
    let mut ix: usize = 20;
    let mut iy: usize = 50;
    for _ in 0..index {
        ix += 140;
        if ix + 120 > 560 { ix = 20; iy += 100; }
    }
    (ix, iy)
}

fn set_desktop_hover(ps2: &mut Ps2, grid: &DesktopGrid, tray_open: bool) {
    // FAB
    ps2.hover_rects[0] = (8, 436, 80, 472);
    ps2.hover_rect_count = 1;

    // D-pad buttons
    let dcx: usize = 590;
    let dcy: usize = 450;
    ps2.hover_rects[1] = (dcx - 10, dcy - 30, dcx + 10, dcy - 10); // up
    ps2.hover_rects[2] = (dcx - 10, dcy + 10, dcx + 10, dcy + 30); // down
    ps2.hover_rects[3] = (dcx - 30, dcy - 10, dcx - 10, dcy + 10); // left
    ps2.hover_rects[4] = (dcx + 10, dcy - 10, dcx + 30, dcy + 10); // right
    ps2.hover_rects[5] = (570, 400, 610, 420); // + button
    ps2.hover_rect_count = 6;

    if tray_open {
        set_fab_hover(ps2, true);
    }
}

fn lighten_c(c: pst_framebuffer::Color) -> pst_framebuffer::Color {
    pst_framebuffer::Color::rgb(c.r.saturating_add(40), c.g.saturating_add(40), c.b.saturating_add(40))
}

fn darken_c(c: pst_framebuffer::Color) -> pst_framebuffer::Color {
    pst_framebuffer::Color::rgb(c.r.saturating_sub(30), c.g.saturating_sub(30), c.b.saturating_sub(30))
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
        ps2.redraw_cursor();

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

fn set_fab_hover(ps2: &mut Ps2, tray_open: bool) {
    // FAB always hoverable
    ps2.hover_rects[0] = (8, 436, 80, 472);
    ps2.hover_rect_count = 1;

    if tray_open {
        let items = start_menu_items();
        let item_h: usize = 28;
        let tray_bottom: usize = 432;
        let tray_h = items.len() * item_h + 8;
        let tray_top = tray_bottom - tray_h;
        // Add each menu item as a hover rect
        for i in 0..items.len() {
            if ps2.hover_rect_count >= 8 { break; }
            let iy = tray_top + 4 + i * item_h;
            ps2.hover_rects[ps2.hover_rect_count] = (8, iy, 168, iy + item_h);
            ps2.hover_rect_count += 1;
        }
    }
}

fn start_menu_items() -> [(& 'static str, pst_framebuffer::Color); 7] {
    use pst_framebuffer::Color;
    [
        ("Editor",    Color::rgb(59, 130, 246)),
        ("Markout",   Color::rgb(16, 185, 129)),
        ("Browser",   Color::rgb(245, 158, 11)),
        ("Code",      Color::rgb(139, 92, 246)),
        ("Storybook", Color::rgb(234, 88, 12)),
        ("Form",      Color::rgb(236, 72, 153)),
        ("Save",      Color::rgb(107, 114, 128)),
    ]
}

fn draw_fab(fb_vaddr: u64, open: bool) {
    if fb_vaddr == 0 { return; }
    use pst_framebuffer::Color;
    let vga = fb_vaddr as *mut u8;

    let fx: usize = 8;
    let fy: usize = 436;
    let fw: usize = 72;
    let fh: usize = 36;
    let color = Color::rgb(59, 130, 246);

    fill(vga, fx, fy, fw, fh, color);
    fill(vga, fx, fy, fw, 2, lighten(color));
    fill(vga, fx, fy + fh - 2, fw, 2, darken(color));
    fill(vga, fx, fy, 2, fh, lighten(color));
    fill(vga, fx + fw - 2, fy, 2, fh, darken(color));

    draw_text(vga, fx + 6, fy + 12, "Start", Color::WHITE);

    // Arrow on right
    let ax = fx + fw - 14;
    let ay = fy + fh / 2;
    if open {
        for i in 0..5usize {
            fill(vga, ax - 4 + i, ay - 2 + i, 2, 1, Color::WHITE);
            fill(vga, ax + 4 - i, ay - 2 + i, 2, 1, Color::WHITE);
        }
    } else {
        for i in 0..5usize {
            fill(vga, ax - 4 + i, ay + 2 - i, 2, 1, Color::WHITE);
            fill(vga, ax + 4 - i, ay + 2 - i, 2, 1, Color::WHITE);
        }
    }
}

fn draw_start_tray(fb_vaddr: u64) {
    if fb_vaddr == 0 { return; }
    use pst_framebuffer::Color;
    let vga = fb_vaddr as *mut u8;

    let items = start_menu_items();
    let tray_x: usize = 8;
    let tray_w: usize = 160;
    let item_h: usize = 28;
    let tray_h = items.len() * item_h + 8;
    let tray_bottom: usize = 432; // above FAB
    let tray_top = tray_bottom - tray_h;

    // Tray background with border
    fill(vga, tray_x, tray_top, tray_w, tray_h, Color::rgb(35, 35, 40));
    fill(vga, tray_x, tray_top, tray_w, 1, Color::rgb(60, 60, 70));
    fill(vga, tray_x, tray_bottom - 1, tray_w, 1, Color::rgb(25, 25, 30));
    fill(vga, tray_x, tray_top, 1, tray_h, Color::rgb(60, 60, 70));
    fill(vga, tray_x + tray_w - 1, tray_top, 1, tray_h, Color::rgb(25, 25, 30));

    // Menu items
    for (i, (label, color)) in items.iter().enumerate() {
        let iy = tray_top + 4 + i * item_h;

        // Color accent bar on left
        fill(vga, tray_x + 4, iy + 4, 4, item_h - 8, *color);

        // Label
        draw_text(vga, tray_x + 14, iy + 8, label, Color::rgb(220, 220, 220));

        // Subtle separator
        if i < items.len() - 1 {
            fill(vga, tray_x + 10, iy + item_h - 1, tray_w - 20, 1, Color::rgb(50, 50, 55));
        }
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
