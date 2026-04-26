use sel4_sys::*;
use crate::{serial_print, serial_print_num};
use crate::sel4_shims;
use libprivos::mem::UntypedAllocator;

const PS2_DATA: u16 = 0x60;
const PS2_STATUS: u16 = 0x64;
const KB_IRQ_PIN: u64 = 1;
const KB_VECTOR: u64 = 33;
const MOUSE_IRQ_PIN: u64 = 12;
const MOUSE_VECTOR: u64 = 44;

const SCREEN_W: i32 = 640;
const SCREEN_H: i32 = 480;

static SCANCODE_TO_ASCII: [u8; 128] = {
    let mut t = [0u8; 128];
    t[0x01] = 0x1B;
    t[0x02] = b'1'; t[0x03] = b'2'; t[0x04] = b'3'; t[0x05] = b'4';
    t[0x06] = b'5'; t[0x07] = b'6'; t[0x08] = b'7'; t[0x09] = b'8';
    t[0x0A] = b'9'; t[0x0B] = b'0'; t[0x0C] = b'-'; t[0x0D] = b'=';
    t[0x0E] = 0x08;
    t[0x0F] = b'\t';
    t[0x10] = b'q'; t[0x11] = b'w'; t[0x12] = b'e'; t[0x13] = b'r';
    t[0x14] = b't'; t[0x15] = b'y'; t[0x16] = b'u'; t[0x17] = b'i';
    t[0x18] = b'o'; t[0x19] = b'p'; t[0x1A] = b'['; t[0x1B] = b']';
    t[0x1C] = b'\n';
    t[0x1E] = b'a'; t[0x1F] = b's'; t[0x20] = b'd'; t[0x21] = b'f';
    t[0x22] = b'g'; t[0x23] = b'h'; t[0x24] = b'j'; t[0x25] = b'k';
    t[0x26] = b'l'; t[0x27] = b';'; t[0x28] = b'\'';
    t[0x29] = b'`';
    t[0x2B] = b'\\';
    t[0x2C] = b'z'; t[0x2D] = b'x'; t[0x2E] = b'c'; t[0x2F] = b'v';
    t[0x30] = b'b'; t[0x31] = b'n'; t[0x32] = b'm'; t[0x33] = b',';
    t[0x34] = b'.'; t[0x35] = b'/';
    t[0x39] = b' ';
    t
};

static SCANCODE_TO_ASCII_SHIFT: [u8; 128] = {
    let mut t = [0u8; 128];
    t[0x01] = 0x1B;
    t[0x02] = b'!'; t[0x03] = b'@'; t[0x04] = b'#'; t[0x05] = b'$';
    t[0x06] = b'%'; t[0x07] = b'^'; t[0x08] = b'&'; t[0x09] = b'*';
    t[0x0A] = b'('; t[0x0B] = b')'; t[0x0C] = b'_'; t[0x0D] = b'+';
    t[0x0E] = 0x08;
    t[0x0F] = b'\t';
    t[0x10] = b'Q'; t[0x11] = b'W'; t[0x12] = b'E'; t[0x13] = b'R';
    t[0x14] = b'T'; t[0x15] = b'Y'; t[0x16] = b'U'; t[0x17] = b'I';
    t[0x18] = b'O'; t[0x19] = b'P'; t[0x1A] = b'{'; t[0x1B] = b'}';
    t[0x1C] = b'\n';
    t[0x1E] = b'A'; t[0x1F] = b'S'; t[0x20] = b'D'; t[0x21] = b'F';
    t[0x22] = b'G'; t[0x23] = b'H'; t[0x24] = b'J'; t[0x25] = b'K';
    t[0x26] = b'L'; t[0x27] = b':'; t[0x28] = b'"';
    t[0x29] = b'~';
    t[0x2B] = b'|';
    t[0x2C] = b'Z'; t[0x2D] = b'X'; t[0x2E] = b'C'; t[0x2F] = b'V';
    t[0x30] = b'B'; t[0x31] = b'N'; t[0x32] = b'M'; t[0x33] = b'<';
    t[0x34] = b'>'; t[0x35] = b'?';
    t[0x39] = b' ';
    t
};

pub const KEY_UP: u8 = 0x80;
pub const KEY_DOWN: u8 = 0x81;
pub const KEY_LEFT: u8 = 0x82;
pub const KEY_RIGHT: u8 = 0x83;
pub const KEY_F1: u8 = 0xF1;
pub const KEY_F2: u8 = 0xF2;
pub const KEY_F3: u8 = 0xF3;
pub const KEY_F4: u8 = 0xF4;
pub const KEY_F5: u8 = 0xF5;
pub const KEY_F6: u8 = 0xF6;
pub const KEY_F7: u8 = 0xF7;

pub enum InputEvent {
    Key(u8),
    Click { x: usize, y: usize },
    MouseDown { x: usize, y: usize },
    MouseUp { x: usize, y: usize },
    MouseDrag { x: usize, y: usize },
    MouseMove { x: usize, y: usize },
}

pub struct Ps2 {
    port_cap: u64,
    notif: seL4_CPtr,       // single notification for both IRQs
    kb_handler: seL4_CPtr,
    mouse_handler: seL4_CPtr,
    has_mouse: bool,
    kb_extended: bool,
    shift_held: bool,
    mouse_packet: [u8; 3],
    mouse_idx: u8,
    mouse_x: i32,
    mouse_y: i32,
    fb_vaddr: u64,
    saved_bg: [u8; 16 * 16 * 4],
    cursor_x: i32,
    cursor_y: i32,
    cursor_drawn: bool,
    mouse_btn_down: bool,
    drag_anchor_x: i32,
    drag_anchor_y: i32,
    drag_rect_drawn: bool,
    drag_last_x: i32,
    drag_last_y: i32,
}

pub fn setup(bootinfo: *const seL4_BootInfo, mut next_slot: u64, fb_vaddr: u64) -> Option<Ps2> {
    let bi = unsafe { &*bootinfo };

    let port_cap = next_slot;
    next_slot += 1;
    let err = unsafe {
        seL4_X86_IOPortControl_Issue(
            seL4_CapIOPortControl, PS2_DATA, PS2_STATUS,
            seL4_CapInitThreadCNode, port_cap, 64,
        )
    };
    if err != seL4_NoError {
        serial_print("[ps2] Port cap failed: ");
        serial_print_num(err as usize);
        serial_print("\n");
        return None;
    }

    let mut alloc = unsafe { UntypedAllocator::new(bi) };
    let skip = (next_slot - bi.empty.start) as usize;
    for _ in 0..skip { alloc.next_slot(); }

    // ONE notification shared by both IRQs
    let notif = match alloc.create_notification() {
        Ok(c) => c, Err(_) => { serial_print("[ps2] Notif alloc failed\n"); return None; }
    };

    // Keyboard IRQ
    let kb_handler = alloc.next_slot();
    let err = unsafe {
        sel4_shims::seL4_IRQControl_GetIOAPIC(
            seL4_CapIRQControl, seL4_CapInitThreadCNode, kb_handler, 64,
            0, KB_IRQ_PIN, 0, 0, KB_VECTOR,
        )
    };
    if err != seL4_NoError {
        serial_print("[ps2] KB IRQ failed: "); serial_print_num(err as usize); serial_print("\n");
        return None;
    }
    let err = unsafe { seL4_IRQHandler_SetNotification(kb_handler, notif) };
    if err != seL4_NoError { serial_print("[ps2] KB bind failed\n"); return None; }
    serial_print("[ps2] Keyboard ready\n");

    // Enable mouse on PS/2 controller
    ps2_flush(port_cap);
    ps2_cmd(port_cap, 0xA8); // enable second port

    // Read and update controller config
    ps2_cmd(port_cap, 0x20);
    ps2_wait_output(port_cap);
    let config = unsafe { native::sel4_ioport_in8(port_cap, PS2_DATA) };
    let new_config = (config | 0x03) & !0x30; // IRQ1 + IRQ12 on, both clocks on
    ps2_cmd(port_cap, 0x60);
    ps2_wait_input(port_cap);
    unsafe { native::sel4_ioport_out8(port_cap, PS2_DATA, new_config) };

    // Tell mouse to start sending data
    ps2_write_mouse(port_cap, 0xF4);
    ps2_wait_output(port_cap);
    let _ack = unsafe { native::sel4_ioport_in8(port_cap, PS2_DATA) };
    ps2_flush(port_cap);

    // Mouse IRQ — same notification as keyboard
    let mouse_handler = alloc.next_slot();
    let err = unsafe {
        sel4_shims::seL4_IRQControl_GetIOAPIC(
            seL4_CapIRQControl, seL4_CapInitThreadCNode, mouse_handler, 64,
            0, MOUSE_IRQ_PIN, 0, 0, MOUSE_VECTOR,
        )
    };
    let has_mouse = if err == seL4_NoError {
        let err = unsafe { seL4_IRQHandler_SetNotification(mouse_handler, notif) };
        if err == seL4_NoError {
            serial_print("[ps2] Mouse ready\n");
            true
        } else {
            serial_print("[ps2] Mouse bind failed\n");
            false
        }
    } else {
        serial_print("[ps2] Mouse IRQ failed: "); serial_print_num(err as usize); serial_print("\n");
        false
    };

    Some(Ps2 {
        port_cap, notif, kb_handler, mouse_handler,
        has_mouse, kb_extended: false, shift_held: false,
        mouse_packet: [0; 3], mouse_idx: 0,
        mouse_x: SCREEN_W / 2, mouse_y: SCREEN_H / 2,
        fb_vaddr, saved_bg: [0; 16 * 16 * 4], cursor_x: -1, cursor_y: -1, cursor_drawn: false, mouse_btn_down: false,
        drag_anchor_x: 0, drag_anchor_y: 0, drag_rect_drawn: false, drag_last_x: 0, drag_last_y: 0,
    })
}

impl Ps2 {
    pub fn read_event(&mut self) -> InputEvent {
        loop {
            unsafe { native::sel4_wait_notification(self.notif) };

            // Drain all available bytes
            loop {
                let status = unsafe { native::sel4_ioport_in8(self.port_cap, PS2_STATUS) };
                if status & 0x01 == 0 { break; } // no data

                let byte = unsafe { native::sel4_ioport_in8(self.port_cap, PS2_DATA) };
                let from_mouse = status & 0x20 != 0;

                if from_mouse && self.has_mouse {
                    if let Some(evt) = self.handle_mouse_byte(byte) {
                        self.ack_all();
                        return evt;
                    }
                } else {
                    if let Some(evt) = self.handle_kb_byte(byte) {
                        self.ack_all();
                        return evt;
                    }
                }
            }

            self.ack_all();
        }
    }

    fn ack_all(&self) {
        unsafe {
            seL4_IRQHandler_Ack(self.kb_handler);
            if self.has_mouse { seL4_IRQHandler_Ack(self.mouse_handler); }
        }
    }

    fn handle_kb_byte(&mut self, byte: u8) -> Option<InputEvent> {
        if byte == 0xE0 { self.kb_extended = true; return None; }

        // Key release (bit 7 set)
        if byte & 0x80 != 0 {
            let released = byte & 0x7F;
            if released == 0x2A || released == 0x36 { self.shift_held = false; }
            self.kb_extended = false;
            return None;
        }

        // Shift press
        if byte == 0x2A || byte == 0x36 { self.shift_held = true; return None; }

        if self.kb_extended {
            self.kb_extended = false;
            return match byte {
                0x48 => Some(InputEvent::Key(KEY_UP)),
                0x50 => Some(InputEvent::Key(KEY_DOWN)),
                0x4B => Some(InputEvent::Key(KEY_LEFT)),
                0x4D => Some(InputEvent::Key(KEY_RIGHT)),
                0x53 => Some(InputEvent::Key(0x7F)), // Delete key
                _ => None,
            };
        }

        match byte {
            0x3B => return Some(InputEvent::Key(KEY_F1)),
            0x3C => return Some(InputEvent::Key(KEY_F2)),
            0x3D => return Some(InputEvent::Key(KEY_F3)),
            0x3E => return Some(InputEvent::Key(KEY_F4)),
            0x3F => return Some(InputEvent::Key(KEY_F5)),
            0x40 => return Some(InputEvent::Key(KEY_F6)),
            0x41 => return Some(InputEvent::Key(KEY_F7)),
            _ => {}
        }

        let table = if self.shift_held { &SCANCODE_TO_ASCII_SHIFT } else { &SCANCODE_TO_ASCII };
        let ascii = table[byte as usize & 0x7F];
        if ascii != 0 { Some(InputEvent::Key(ascii)) } else { None }
    }

    fn handle_mouse_byte(&mut self, byte: u8) -> Option<InputEvent> {
        // First byte must have bit 3 set — use this to resync
        if self.mouse_idx == 0 && byte & 0x08 == 0 { return None; }

        self.mouse_packet[self.mouse_idx as usize] = byte;
        self.mouse_idx += 1;
        if self.mouse_idx < 3 { return None; }
        self.mouse_idx = 0;

        let dx = self.mouse_packet[1] as i32 - if self.mouse_packet[0] & 0x10 != 0 { 256 } else { 0 };
        let dy = -(self.mouse_packet[2] as i32 - if self.mouse_packet[0] & 0x20 != 0 { 256 } else { 0 });

        self.mouse_x = (self.mouse_x + dx).clamp(0, SCREEN_W - 1);
        self.mouse_y = (self.mouse_y + dy).clamp(0, SCREEN_H - 1);

        self.draw_cursor();

        let btn_now = self.mouse_packet[0] & 0x01 != 0;
        let was_down = self.mouse_btn_down;
        self.mouse_btn_down = btn_now;
        let pos = (self.mouse_x as usize, self.mouse_y as usize);

        if btn_now && !was_down {
            self.drag_anchor_x = self.mouse_x;
            self.drag_anchor_y = self.mouse_y;
            self.drag_last_x = self.mouse_x;
            self.drag_last_y = self.mouse_y;
            Some(InputEvent::MouseDown { x: pos.0, y: pos.1 })
        } else if !btn_now && was_down {
            Some(InputEvent::Click { x: pos.0, y: pos.1 })
        } else if btn_now && was_down && (dx != 0 || dy != 0) {
            self.drag_last_x = self.mouse_x;
            self.drag_last_y = self.mouse_y;
            Some(InputEvent::MouseDrag { x: pos.0, y: pos.1 })
        } else {
            None
        }
    }

    pub fn invalidate_cursor(&mut self) {
        self.cursor_drawn = false;
    }

    pub fn drag_rect(&self) -> Option<(usize, usize, usize, usize)> {
        if !self.mouse_btn_down { return None; }
        let x0 = self.drag_anchor_x.min(self.drag_last_x) as usize;
        let y0 = self.drag_anchor_y.min(self.drag_last_y) as usize;
        let x1 = self.drag_anchor_x.max(self.drag_last_x) as usize;
        let y1 = self.drag_anchor_y.max(self.drag_last_y) as usize;
        if x1 - x0 > 4 || y1 - y0 > 4 {
            Some((x0, y0, x1, y1))
        } else {
            None
        }
    }

    pub fn drag_anchor(&self) -> (usize, usize) {
        (self.drag_anchor_x as usize, self.drag_anchor_y as usize)
    }

    fn xor_dashed_rect(&self, x0: i32, y0: i32, x1: i32, y1: i32) {
        if self.fb_vaddr == 0 { return; }
        let vga = self.fb_vaddr as *mut u8;

        let left   = x0.min(x1).max(0) as usize;
        let top    = y0.min(y1).max(0) as usize;
        let right  = x0.max(x1).min(639) as usize;
        let bottom = y0.max(y1).min(479) as usize;

        if right - left < 3 || bottom - top < 3 { return; }

        // Dashed pattern: 4px on, 4px off
        // Top edge
        for px in left..=right {
            if ((px - left) / 4) % 2 == 0 {
                xor_pixel(vga, px, top);
            }
        }
        // Bottom edge
        for px in left..=right {
            if ((px - left) / 4) % 2 == 0 {
                xor_pixel(vga, px, bottom);
            }
        }
        // Left edge
        for py in (top + 1)..bottom {
            if ((py - top) / 4) % 2 == 0 {
                xor_pixel(vga, left, py);
            }
        }
        // Right edge
        for py in (top + 1)..bottom {
            if ((py - top) / 4) % 2 == 0 {
                xor_pixel(vga, right, py);
            }
        }
    }

    fn draw_cursor(&mut self) {
        if self.fb_vaddr == 0 { return; }
        let vga = self.fb_vaddr as *mut u8;

        // 1. Restore saved background at old position
        if self.cursor_drawn {
            let ox = self.cursor_x as usize;
            let oy = self.cursor_y as usize;
            for dy in 0..CURSOR_SIZE {
                for dx in 0..CURSOR_SIZE {
                    let sx = ox + dx;
                    let sy = oy + dy;
                    if sx < SCREEN_W as usize && sy < SCREEN_H as usize {
                        let off = (sy * SCREEN_W as usize + sx) * 4;
                        let si = (dy * CURSOR_SIZE + dx) * 4;
                        unsafe {
                            *vga.add(off) = self.saved_bg[si];
                            *vga.add(off + 1) = self.saved_bg[si + 1];
                            *vga.add(off + 2) = self.saved_bg[si + 2];
                            *vga.add(off + 3) = self.saved_bg[si + 3];
                        }
                    }
                }
            }
        }

        // 2. Save background at new position
        let nx = self.mouse_x as usize;
        let ny = self.mouse_y as usize;
        for dy in 0..CURSOR_SIZE {
            for dx in 0..CURSOR_SIZE {
                let sx = nx + dx;
                let sy = ny + dy;
                let si = (dy * CURSOR_SIZE + dx) * 4;
                if sx < SCREEN_W as usize && sy < SCREEN_H as usize {
                    let off = (sy * SCREEN_W as usize + sx) * 4;
                    unsafe {
                        self.saved_bg[si] = *vga.add(off);
                        self.saved_bg[si + 1] = *vga.add(off + 1);
                        self.saved_bg[si + 2] = *vga.add(off + 2);
                        self.saved_bg[si + 3] = *vga.add(off + 3);
                    }
                } else {
                    self.saved_bg[si] = 30;
                    self.saved_bg[si + 1] = 30;
                    self.saved_bg[si + 2] = 30;
                    self.saved_bg[si + 3] = 0xFF;
                }
            }
        }

        // 3. Draw cursor — ring + iris
        let over_button = ny >= 448 && ((nx >= 8 && nx < 80) || (nx >= 84 && nx < 164) ||
            (nx >= 168 && nx < 244) || (nx >= 248 && nx < 304) || (nx >= 308 && nx < 364) || (nx >= 368 && nx < 424));
        let (ir, ig, ib) = if over_button {
            (50u8, 255u8, 100u8) // bright green = go
        } else {
            (80, 160, 255) // blue
        };

        for dy in 0..CURSOR_SIZE {
            for dx in 0..CURSOR_SIZE {
                let sx = nx + dx;
                let sy = ny + dy;
                if sx >= SCREEN_W as usize || sy >= SCREEN_H as usize { continue; }
                let off = (sy * SCREEN_W as usize + sx) * 4;
                let is_iris = (CURSOR_SHAPE[dy] & (0x8000 >> dx)) != 0;
                let is_ring = (CURSOR_BORDER[dy] & (0x8000 >> dx)) != 0;
                if is_iris {
                    unsafe { *vga.add(off) = ib; *vga.add(off+1) = ig; *vga.add(off+2) = ir; }
                } else if is_ring {
                    unsafe { *vga.add(off) = 255; *vga.add(off+1) = 255; *vga.add(off+2) = 255; }
                }
            }
        }

        self.cursor_x = nx as i32;
        self.cursor_y = ny as i32;
        self.cursor_drawn = true;
    }
}

const SMOKE_R: u8 = 10;
const SMOKE_G: u8 = 10;
const SMOKE_B: u8 = 10;
const BG_R: u8 = 30;
const BG_G: u8 = 30;
const BG_B: u8 = 30;

fn stamp_smoke(fb_vaddr: u64, cx: usize, cy: usize) {
    let vga = fb_vaddr as *mut u8;
    for dy in 0..CURSOR_SIZE {
        let mask = CURSOR_BORDER[dy];
        for dx in 0..CURSOR_SIZE {
            if (mask & (0x8000 >> dx)) == 0 { continue; }
            let sx = cx + dx;
            let sy = cy + dy;
            if sx < SCREEN_W as usize && sy < SCREEN_H as usize {
                let off = (sy * SCREEN_W as usize + sx) * 4;
                unsafe {
                    let b = *vga.add(off);
                    let g = *vga.add(off + 1);
                    let r = *vga.add(off + 2);
                    if r == BG_R && g == BG_G && b == BG_B {
                        *vga.add(off) = SMOKE_B;
                        *vga.add(off + 1) = SMOKE_G;
                        *vga.add(off + 2) = SMOKE_R;
                    }
                }
            }
        }
    }
}

fn erase_smoke(fb_vaddr: u64, cx: usize, cy: usize) {
    let vga = fb_vaddr as *mut u8;
    for dy in 0..CURSOR_SIZE {
        let mask = CURSOR_BORDER[dy];
        for dx in 0..CURSOR_SIZE {
            if (mask & (0x8000 >> dx)) == 0 { continue; }
            let sx = cx + dx;
            let sy = cy + dy;
            if sx < SCREEN_W as usize && sy < SCREEN_H as usize {
                let off = (sy * SCREEN_W as usize + sx) * 4;
                unsafe {
                    if *vga.add(off) == SMOKE_B && *vga.add(off+1) == SMOKE_G && *vga.add(off+2) == SMOKE_R {
                        *vga.add(off) = BG_B;
                        *vga.add(off + 1) = BG_G;
                        *vga.add(off + 2) = BG_R;
                    }
                }
            }
        }
    }
}

fn erase_iris(fb_vaddr: u64, cx: usize, cy: usize) {
    let vga = fb_vaddr as *mut u8;
    for dy in 0..CURSOR_SIZE {
        let shape = CURSOR_SHAPE[dy];
        for dx in 0..CURSOR_SIZE {
            if (shape & (0x8000 >> dx)) == 0 { continue; }
            let sx = cx + dx;
            let sy = cy + dy;
            if sx < SCREEN_W as usize && sy < SCREEN_H as usize {
                let off = (sy * SCREEN_W as usize + sx) * 4;
                unsafe {
                    *vga.add(off) = BG_B;
                    *vga.add(off + 1) = BG_G;
                    *vga.add(off + 2) = BG_R;
                }
            }
        }
    }
}

fn solid_iris(fb_vaddr: u64, cx: usize, cy: usize, r: u8, g: u8, b: u8) {
    let vga = fb_vaddr as *mut u8;
    for dy in 0..CURSOR_SIZE {
        let shape = CURSOR_SHAPE[dy];
        for dx in 0..CURSOR_SIZE {
            if (shape & (0x8000 >> dx)) == 0 { continue; }
            let sx = cx + dx;
            let sy = cy + dy;
            if sx < SCREEN_W as usize && sy < SCREEN_H as usize {
                let off = (sy * SCREEN_W as usize + sx) * 4;
                unsafe {
                    *vga.add(off) = b;
                    *vga.add(off + 1) = g;
                    *vga.add(off + 2) = r;
                }
            }
        }
    }
}

fn xor_iris(fb_vaddr: u64, cx: usize, cy: usize) {
    let vga = fb_vaddr as *mut u8;
    for dy in 0..CURSOR_SIZE {
        let shape = CURSOR_SHAPE[dy];
        for dx in 0..CURSOR_SIZE {
            if (shape & (0x8000 >> dx)) == 0 { continue; }
            let sx = cx + dx;
            let sy = cy + dy;
            if sx < SCREEN_W as usize && sy < SCREEN_H as usize {
                let off = (sy * SCREEN_W as usize + sx) * 4;
                unsafe {
                    *vga.add(off) ^= 0xFF;
                    *vga.add(off + 1) ^= 0xFF;
                    *vga.add(off + 2) ^= 0xFF;
                }
            }
        }
    }
}

fn xor_pixel(vga: *mut u8, x: usize, y: usize) {
    if x < SCREEN_W as usize && y < SCREEN_H as usize {
        let off = (y * SCREEN_W as usize + x) * 4;
        unsafe {
            *vga.add(off) ^= 0x80;
            *vga.add(off + 1) ^= 0x80;
            *vga.add(off + 2) ^= 0x80;
        }
    }
}

fn xor_cursor(fb_vaddr: u64, cx: usize, cy: usize) {
    let vga = fb_vaddr as *mut u8;
    for dy in 0..CURSOR_SIZE {
        let mask = CURSOR_BORDER[dy];
        for dx in 0..CURSOR_SIZE {
            if (mask & (0x8000 >> dx)) == 0 { continue; }
            let sx = cx + dx;
            let sy = cy + dy;
            if sx < SCREEN_W as usize && sy < SCREEN_H as usize {
                let off = (sy * SCREEN_W as usize + sx) * 4;
                unsafe {
                    *vga.add(off) ^= 0xFF;
                    *vga.add(off + 1) ^= 0xFF;
                    *vga.add(off + 2) ^= 0xFF;
                }
            }
        }
    }
}

fn draw_cursor_solid(fb_vaddr: u64, cx: usize, cy: usize, white: bool) {
    let vga = fb_vaddr as *mut u8;
    for dy in 0..CURSOR_SIZE {
        for dx in 0..CURSOR_SIZE {
            let sx = cx + dx;
            let sy = cy + dy;
            if sx >= SCREEN_W as usize || sy >= SCREEN_H as usize { continue; }
            let off = (sy * SCREEN_W as usize + sx) * 4;
            let is_shape = (CURSOR_SHAPE[dy] & (0x8000 >> dx)) != 0;
            let is_border = (CURSOR_BORDER[dy] & (0x8000 >> dx)) != 0;
            if is_shape && white {
                unsafe { *vga.add(off)=255; *vga.add(off+1)=255; *vga.add(off+2)=255; }
            } else if is_border {
                unsafe { *vga.add(off)=0; *vga.add(off+1)=0; *vga.add(off+2)=0; }
            }
        }
    }
}

fn erase_cursor_black(fb_vaddr: u64, cx: usize, cy: usize) {
    let vga = fb_vaddr as *mut u8;
    let bg_r: u8 = 30; let bg_g: u8 = 30; let bg_b: u8 = 30; // DARK_BG
    for dy in 0..CURSOR_SIZE {
        let mask = CURSOR_BORDER[dy];
        for dx in 0..CURSOR_SIZE {
            if (mask & (0x8000 >> dx)) == 0 { continue; }
            let sx = cx + dx;
            let sy = cy + dy;
            if sx < SCREEN_W as usize && sy < SCREEN_H as usize {
                let off = (sy * SCREEN_W as usize + sx) * 4;
                unsafe { *vga.add(off)=bg_b; *vga.add(off+1)=bg_g; *vga.add(off+2)=bg_r; }
            }
        }
    }
}

const CURSOR_SIZE: usize = 16;
// SHAPE = iris only (filled center dot)
const CURSOR_SHAPE: [u16; 16] = [
    0b0000000000000000,
    0b0000000000000000,
    0b0000000000000000,
    0b0000000000000000,
    0b0000001110000000,
    0b0000011111000000,
    0b0000111111100000,
    0b0000111111100000,
    0b0000111111100000,
    0b0000011111000000,
    0b0000001110000000,
    0b0000000000000000,
    0b0000000000000000,
    0b0000000000000000,
    0b0000000000000000,
    0b0000000000000000,
];
// BORDER with SHAPE pixels removed (no overlap)
const CURSOR_BORDER: [u16; 16] = [
    0b0000011111000000,
    0b0001111111110000,
    0b0011100000111000,
    0b0110000000001100,
    0b1100000000000110,
    0b1100000000000110,
    0b1000000000000010,
    0b1000000000000010,
    0b1000000000000010,
    0b1100000000000110,
    0b1100000000000110,
    0b0110000000001100,
    0b0011100000111000,
    0b0001111111110000,
    0b0000011111000000,
    0b0000000000000000,
];

fn ps2_cmd(port_cap: u64, cmd: u8) {
    ps2_wait_input(port_cap);
    unsafe { native::sel4_ioport_out8(port_cap, PS2_STATUS, cmd) };
}

fn ps2_wait_input(port_cap: u64) {
    for _ in 0..100_000 {
        if unsafe { native::sel4_ioport_in8(port_cap, PS2_STATUS) } & 0x02 == 0 { return; }
        core::hint::spin_loop();
    }
}

fn ps2_wait_output(port_cap: u64) {
    for _ in 0..100_000 {
        if unsafe { native::sel4_ioport_in8(port_cap, PS2_STATUS) } & 0x01 != 0 { return; }
        core::hint::spin_loop();
    }
}

fn ps2_flush(port_cap: u64) {
    for _ in 0..32 {
        if unsafe { native::sel4_ioport_in8(port_cap, PS2_STATUS) } & 0x01 == 0 { break; }
        unsafe { native::sel4_ioport_in8(port_cap, PS2_DATA) };
    }
}

fn ps2_write_mouse(port_cap: u64, byte: u8) {
    ps2_cmd(port_cap, 0xD4);
    ps2_wait_input(port_cap);
    unsafe { native::sel4_ioport_out8(port_cap, PS2_DATA, byte) };
}
