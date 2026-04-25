use pst_framebuffer::font::{GLYPH_WIDTH, GLYPH_HEIGHT};
use pst_framebuffer::Color;

const WIDTH: usize = 640;
const HEIGHT: usize = 480;
const COLS: usize = WIDTH / GLYPH_WIDTH;
const ROWS: usize = HEIGHT / GLYPH_HEIGHT;

static mut VGACON: VgaCon = VgaCon {
    fb_vaddr: 0,
    cursor_col: 0,
    cursor_row: 0,
    fg: Color::WHITE,
    bg: Color::DARK_BG,
};

pub struct VgaCon {
    fb_vaddr: u64,
    cursor_col: usize,
    cursor_row: usize,
    fg: Color,
    bg: Color,
}

pub fn init(fb_vaddr: u64) {
    unsafe {
        VGACON.fb_vaddr = fb_vaddr;
        VGACON.cursor_col = 0;
        VGACON.cursor_row = 0;
    }
    clear();
}

pub fn clear() {
    unsafe {
        if VGACON.fb_vaddr == 0 { return; }
        let vga = VGACON.fb_vaddr as *mut u8;
        let bg = VGACON.bg;
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let off = (y * WIDTH + x) * 4;
                *vga.add(off) = bg.b;
                *vga.add(off + 1) = bg.g;
                *vga.add(off + 2) = bg.r;
                *vga.add(off + 3) = 0xFF;
            }
        }
        VGACON.cursor_col = 0;
        VGACON.cursor_row = 0;
    }
}

pub fn putchar(c: u8) {
    unsafe {
        if VGACON.fb_vaddr == 0 { return; }

        // Handle ANSI escape sequences — skip them
        // (they control serial terminal, VGA just shows plain text)
        static mut IN_ESCAPE: bool = false;
        if IN_ESCAPE {
            if c.is_ascii_alphabetic() || c == b'H' || c == b'J' || c == b'm' {
                IN_ESCAPE = false;
                if c == b'J' {
                    // Clear screen
                    clear();
                } else if c == b'H' {
                    // Cursor home
                    VGACON.cursor_col = 0;
                    VGACON.cursor_row = 0;
                }
            }
            return;
        }
        if c == 0x1B { IN_ESCAPE = true; return; }

        if c == b'\r' {
            VGACON.cursor_col = 0;
            return;
        }

        if c == b'\n' {
            VGACON.cursor_col = 0;
            VGACON.cursor_row += 1;
            if VGACON.cursor_row >= ROWS {
                scroll_up();
                VGACON.cursor_row = ROWS - 1;
            }
            return;
        }

        if c == 0x08 { // backspace
            if VGACON.cursor_col > 0 {
                VGACON.cursor_col -= 1;
                draw_char(VGACON.cursor_col, VGACON.cursor_row, b' ');
            }
            return;
        }

        if c < 0x20 { return; } // skip other control chars

        draw_char(VGACON.cursor_col, VGACON.cursor_row, c);
        VGACON.cursor_col += 1;
        if VGACON.cursor_col >= COLS {
            VGACON.cursor_col = 0;
            VGACON.cursor_row += 1;
            if VGACON.cursor_row >= ROWS {
                scroll_up();
                VGACON.cursor_row = ROWS - 1;
            }
        }
    }
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

        // Move everything up by one text row
        core::ptr::copy(vga.add(row_bytes), vga, total - row_bytes);

        // Clear the last row
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
