use pst_framebuffer::font::{GLYPH_WIDTH, GLYPH_HEIGHT};
use pst_framebuffer::Color;

const WIDTH: usize = 640;
const HEIGHT: usize = 480;
const COLS: usize = WIDTH / GLYPH_WIDTH;
const ROWS: usize = HEIGHT / GLYPH_HEIGHT;

static mut VGACON: VgaCon = VgaCon {
    fb_vaddr: 0,
    enabled: true,
    cursor_col: 0,
    cursor_row: 0,
    fg: Color::WHITE,
    bg: Color::DARK_BG,
    esc_state: EscState::Normal,
    esc_buf: [0u8; 16],
    esc_len: 0,
    utf8_buf: [0u8; 4],
    utf8_len: 0,
    utf8_expected: 0,
};

#[derive(Clone, Copy, PartialEq)]
enum EscState {
    Normal,
    Esc,       // seen ESC
    Csi,       // seen ESC[
}

pub struct VgaCon {
    fb_vaddr: u64,
    enabled: bool,
    cursor_col: usize,
    cursor_row: usize,
    fg: Color,
    bg: Color,
    esc_state: EscState,
    esc_buf: [u8; 16],
    esc_len: usize,
    utf8_buf: [u8; 4],
    utf8_len: usize,
    utf8_expected: usize,
}

pub fn init(fb_vaddr: u64) {
    unsafe {
        VGACON.fb_vaddr = fb_vaddr;
        VGACON.cursor_col = 0;
        VGACON.cursor_row = 0;
        VGACON.esc_state = EscState::Normal;
        VGACON.utf8_len = 0;
        VGACON.utf8_expected = 0;
    }
    clear();
}

pub fn clear() {
    unsafe {
        if VGACON.fb_vaddr == 0 { return; }
        let vga = VGACON.fb_vaddr as *mut u8;
        let bg = VGACON.bg;
        let total = WIDTH * HEIGHT * 4;
        for off in (0..total).step_by(4) {
            *vga.add(off) = bg.b;
            *vga.add(off + 1) = bg.g;
            *vga.add(off + 2) = bg.r;
            *vga.add(off + 3) = 0xFF;
        }
        VGACON.cursor_col = 0;
        VGACON.cursor_row = 0;
    }
}

pub fn set_enabled(enabled: bool) {
    unsafe { VGACON.enabled = enabled; }
}

pub fn putchar(c: u8) {
    unsafe {
        if VGACON.fb_vaddr == 0 || !VGACON.enabled { return; }

        // UTF-8 multi-byte handling
        if VGACON.utf8_expected > 0 {
            VGACON.utf8_buf[VGACON.utf8_len] = c;
            VGACON.utf8_len += 1;
            if VGACON.utf8_len >= VGACON.utf8_expected {
                let ch = decode_utf8_box(&VGACON.utf8_buf[..VGACON.utf8_len]);
                VGACON.utf8_len = 0;
                VGACON.utf8_expected = 0;
                if ch != 0 {
                    emit_char(ch);
                }
            }
            return;
        }

        // Start of UTF-8 multi-byte sequence
        if c >= 0xC0 && c < 0xFE {
            VGACON.utf8_buf[0] = c;
            VGACON.utf8_len = 1;
            if c < 0xE0 { VGACON.utf8_expected = 2; }
            else if c < 0xF0 { VGACON.utf8_expected = 3; }
            else { VGACON.utf8_expected = 4; }
            return;
        }

        // ANSI escape sequence handling
        match VGACON.esc_state {
            EscState::Esc => {
                if c == b'[' {
                    VGACON.esc_state = EscState::Csi;
                    VGACON.esc_len = 0;
                } else {
                    VGACON.esc_state = EscState::Normal;
                }
                return;
            }
            EscState::Csi => {
                if c.is_ascii_digit() || c == b';' || c == b'?' {
                    if VGACON.esc_len < 16 {
                        VGACON.esc_buf[VGACON.esc_len] = c;
                        VGACON.esc_len += 1;
                    }
                    return;
                }
                // End of CSI sequence
                handle_csi(c);
                VGACON.esc_state = EscState::Normal;
                return;
            }
            EscState::Normal => {}
        }

        if c == 0x1B { VGACON.esc_state = EscState::Esc; return; }
        if c == b'\r' { VGACON.cursor_col = 0; return; }
        if c == b'\n' {
            VGACON.cursor_col = 0;
            VGACON.cursor_row += 1;
            if VGACON.cursor_row >= ROWS { scroll_up(); VGACON.cursor_row = ROWS - 1; }
            return;
        }
        if c == 0x08 {
            if VGACON.cursor_col > 0 { VGACON.cursor_col -= 1; draw_char(VGACON.cursor_col, VGACON.cursor_row, b' '); }
            return;
        }
        if c < 0x20 { return; }

        emit_char(c);
    }
}

unsafe fn emit_char(c: u8) {
    draw_char(VGACON.cursor_col, VGACON.cursor_row, c);
    VGACON.cursor_col += 1;
    if VGACON.cursor_col >= COLS {
        VGACON.cursor_col = 0;
        VGACON.cursor_row += 1;
        if VGACON.cursor_row >= ROWS { scroll_up(); VGACON.cursor_row = ROWS - 1; }
    }
}

unsafe fn handle_csi(cmd: u8) {
    let params = parse_params();

    match cmd {
        b'H' | b'f' => {
            // Cursor position: ESC[row;colH
            let row = if params.len() > 0 && params[0] > 0 { params[0] - 1 } else { 0 };
            let col = if params.len() > 1 && params[1] > 0 { params[1] - 1 } else { 0 };
            VGACON.cursor_row = row.min(ROWS - 1);
            VGACON.cursor_col = col.min(COLS - 1);
        }
        b'J' => {
            // Clear screen
            let mode = if params.len() > 0 { params[0] } else { 0 };
            if mode == 2 { clear(); }
        }
        b'K' => {
            // Clear line from cursor
            for c in VGACON.cursor_col..COLS {
                draw_char(c, VGACON.cursor_row, b' ');
            }
        }
        b'm' => {
            // SGR — color/style (simplified: just reset)
            if params.is_empty() || params[0] == 0 {
                VGACON.fg = Color::WHITE;
            }
        }
        b'l' | b'h' => {} // cursor show/hide — ignore
        _ => {}
    }
}

unsafe fn parse_params() -> [usize; 4] {
    let mut params = [0usize; 4];
    let mut pi = 0;
    let mut val = 0usize;
    let mut has_val = false;

    for i in 0..VGACON.esc_len {
        let c = VGACON.esc_buf[i];
        if c == b'?' { continue; }
        if c == b';' {
            if pi < 4 { params[pi] = val; pi += 1; }
            val = 0;
            has_val = false;
        } else if c.is_ascii_digit() {
            val = val * 10 + (c - b'0') as usize;
            has_val = true;
        }
    }
    if has_val && pi < 4 { params[pi] = val; }
    params
}

fn decode_utf8_box(buf: &[u8]) -> u8 {
    // Map common UTF-8 box drawing to ASCII equivalents
    if buf.len() == 3 && buf[0] == 0xE2 && buf[1] == 0x94 {
        match buf[2] {
            0x80 => return b'-',  // ─
            0x82 => return b'|',  // │
            0x8C => return b'+',  // ┌
            0x90 => return b'+',  // ┐
            0x94 => return b'+',  // └
            0x98 => return b'+',  // ┘
            0xBC => return b'+',  // ┼
            _ => {}
        }
    }
    // Other UTF-8: show as '?'
    if buf.len() >= 2 { return b'?'; }
    buf[0]
}

fn draw_char(col: usize, row: usize, c: u8) {
    unsafe {
        let vga = VGACON.fb_vaddr as *mut u8;
        let glyph = pst_framebuffer::font::glyph(c);
        let px = col * GLYPH_WIDTH;
        let py = row * GLYPH_HEIGHT;
        let fg = VGACON.fg;
        let bg = VGACON.bg;

        for gy in 0..GLYPH_HEIGHT {
            let bits = glyph[gy];
            for gx in 0..GLYPH_WIDTH {
                let color = if bits & (0x80 >> gx) != 0 { fg } else { bg };
                let x = px + gx;
                let y = py + gy;
                if x < WIDTH && y < HEIGHT {
                    let off = (y * WIDTH + x) * 4;
                    *vga.add(off) = color.b;
                    *vga.add(off + 1) = color.g;
                    *vga.add(off + 2) = color.r;
                    *vga.add(off + 3) = 0xFF;
                }
            }
        }
    }
}

fn scroll_up() {
    unsafe {
        let vga = VGACON.fb_vaddr as *mut u8;
        let row_bytes = WIDTH * 4 * GLYPH_HEIGHT;
        let total = WIDTH * 4 * HEIGHT;
        core::ptr::copy(vga.add(row_bytes), vga, total - row_bytes);
        let bg = VGACON.bg;
        let start = total - row_bytes;
        for i in (start..total).step_by(4) {
            *vga.add(i) = bg.b;
            *vga.add(i + 1) = bg.g;
            *vga.add(i + 2) = bg.r;
            *vga.add(i + 3) = 0xFF;
        }
    }
}
