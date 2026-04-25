use sel4_sys::*;
use crate::{serial_print, serial_print_num};
use crate::sel4_shims;
use crate::vgacon;
use pst_framebuffer::font::{GLYPH_WIDTH, GLYPH_HEIGHT};

const PS2_DATA: u16 = 0x60;
const PS2_CMD: u16 = 0x64;
const MOUSE_IRQ_PIN: u64 = 12;
const MOUSE_VECTOR: u64 = 44;

const SCREEN_W: usize = 640;
const SCREEN_H: usize = 480;

pub struct Mouse {
    notif_cap: seL4_CPtr,
    handler_cap: seL4_CPtr,
    port_cap: seL4_CPtr,
    pub x: i32,
    pub y: i32,
    pub buttons: u8,
    packet_idx: u8,
    packet: [u8; 3],
    fb_vaddr: u64,
    cursor_saved: [u8; 16 * 16 * 4],
    cursor_x: i32,
    cursor_y: i32,
}

pub fn setup(bootinfo: *const seL4_BootInfo, kb_port_cap: u64, mut next_slot: u64, fb_vaddr: u64) -> Option<Mouse> {
    let bi = unsafe { &*bootinfo };

    let mut alloc = unsafe { libprivos::mem::UntypedAllocator::new(bi) };
    let skip = (next_slot - bi.empty.start) as usize;
    for _ in 0..skip { alloc.next_slot(); }

    // Enable the mouse on the PS/2 controller
    // Wait for controller input buffer to be empty
    ps2_wait_input(kb_port_cap);
    unsafe { native::sel4_ioport_out8(kb_port_cap, PS2_CMD, 0xA8) }; // enable second port

    // Enable IRQ12 in the controller config
    ps2_wait_input(kb_port_cap);
    unsafe { native::sel4_ioport_out8(kb_port_cap, PS2_CMD, 0x20) }; // read config
    ps2_wait_output(kb_port_cap);
    let mut config = unsafe { native::sel4_ioport_in8(kb_port_cap, PS2_DATA) };
    config |= 0x02; // enable IRQ12
    config &= !0x20; // enable second port clock
    ps2_wait_input(kb_port_cap);
    unsafe { native::sel4_ioport_out8(kb_port_cap, PS2_CMD, 0x60) }; // write config
    ps2_wait_input(kb_port_cap);
    unsafe { native::sel4_ioport_out8(kb_port_cap, PS2_DATA, config) };

    // Send "enable data reporting" to the mouse
    ps2_write_mouse(kb_port_cap, 0xF4);
    ps2_wait_output(kb_port_cap);
    let _ack = unsafe { native::sel4_ioport_in8(kb_port_cap, PS2_DATA) };

    // Register IRQ 12 via IOAPIC
    let notif_cap = match alloc.create_notification() {
        Ok(cap) => cap,
        Err(_) => { serial_print("[mouse] Notification alloc failed\n"); return None; }
    };

    let handler_slot = alloc.next_slot();
    let err = unsafe {
        sel4_shims::seL4_IRQControl_GetIOAPIC(
            seL4_CapIRQControl,
            seL4_CapInitThreadCNode,
            handler_slot, 64,
            0, MOUSE_IRQ_PIN, 0, 0, MOUSE_VECTOR,
        )
    };
    if err != seL4_NoError {
        serial_print("[mouse] IOAPIC IRQ failed: ");
        serial_print_num(err as usize);
        serial_print("\n");
        return None;
    }

    let err = unsafe { seL4_IRQHandler_SetNotification(handler_slot, notif_cap) };
    if err != seL4_NoError {
        serial_print("[mouse] SetNotification failed: ");
        serial_print_num(err as usize);
        serial_print("\n");
        return None;
    }

    serial_print("[mouse] PS/2 mouse ready\n");

    Some(Mouse {
        notif_cap, handler_cap: handler_slot, port_cap: kb_port_cap,
        x: SCREEN_W as i32 / 2, y: SCREEN_H as i32 / 2,
        buttons: 0, packet_idx: 0, packet: [0; 3],
        fb_vaddr, cursor_saved: [0; 16 * 16 * 4],
        cursor_x: -1, cursor_y: -1,
    })
}

impl Mouse {
    pub fn poll(&mut self) -> Option<MouseEvent> {
        // Non-blocking: check if there's a notification
        // For now, this is blocking — call from a separate path
        unsafe { native::sel4_wait_notification(self.notif_cap) };

        let byte = unsafe { native::sel4_ioport_in8(self.port_cap, PS2_DATA) };
        unsafe { seL4_IRQHandler_Ack(self.handler_cap) };

        self.packet[self.packet_idx as usize] = byte;
        self.packet_idx += 1;

        if self.packet_idx < 3 { return None; }
        self.packet_idx = 0;

        // Validate first byte: bit 3 should always be set
        if self.packet[0] & 0x08 == 0 {
            // Desync — skip
            return None;
        }

        let dx = self.packet[1] as i32 - if self.packet[0] & 0x10 != 0 { 256 } else { 0 };
        let dy = -(self.packet[2] as i32 - if self.packet[0] & 0x20 != 0 { 256 } else { 0 });
        let buttons = self.packet[0] & 0x07;

        self.x = (self.x + dx).clamp(0, SCREEN_W as i32 - 1);
        self.y = (self.y + dy).clamp(0, SCREEN_H as i32 - 1);
        self.buttons = buttons;

        self.draw_cursor();

        let left = buttons & 1 != 0;
        if left {
            Some(MouseEvent::Click { x: self.x as usize, y: self.y as usize })
        } else {
            Some(MouseEvent::Move { x: self.x as usize, y: self.y as usize })
        }
    }

    fn draw_cursor(&mut self) {
        if self.fb_vaddr == 0 { return; }
        let vga = self.fb_vaddr as *mut u8;

        // Restore pixels under old cursor
        if self.cursor_x >= 0 {
            let ox = self.cursor_x as usize;
            let oy = self.cursor_y as usize;
            for dy in 0..8usize {
                for dx in 0..8usize {
                    let sx = ox + dx;
                    let sy = oy + dy;
                    if sx < SCREEN_W && sy < SCREEN_H {
                        let off = (sy * SCREEN_W + sx) * 4;
                        let si = (dy * 8 + dx) * 4;
                        unsafe {
                            *vga.add(off) = self.cursor_saved[si];
                            *vga.add(off + 1) = self.cursor_saved[si + 1];
                            *vga.add(off + 2) = self.cursor_saved[si + 2];
                            *vga.add(off + 3) = self.cursor_saved[si + 3];
                        }
                    }
                }
            }
        }

        // Save pixels under new cursor and draw
        let nx = self.x as usize;
        let ny = self.y as usize;
        for dy in 0..8usize {
            for dx in 0..8usize {
                let sx = nx + dx;
                let sy = ny + dy;
                if sx < SCREEN_W && sy < SCREEN_H {
                    let off = (sy * SCREEN_W + sx) * 4;
                    let si = (dy * 8 + dx) * 4;
                    unsafe {
                        self.cursor_saved[si] = *vga.add(off);
                        self.cursor_saved[si + 1] = *vga.add(off + 1);
                        self.cursor_saved[si + 2] = *vga.add(off + 2);
                        self.cursor_saved[si + 3] = *vga.add(off + 3);
                    }
                    // Arrow cursor shape: white with dark border
                    let in_cursor = CURSOR_SHAPE[dy] & (0x80 >> dx) != 0;
                    let in_border = CURSOR_BORDER[dy] & (0x80 >> dx) != 0;
                    if in_cursor {
                        unsafe {
                            *vga.add(off) = 255; *vga.add(off + 1) = 255;
                            *vga.add(off + 2) = 255; *vga.add(off + 3) = 255;
                        }
                    } else if in_border {
                        unsafe {
                            *vga.add(off) = 0; *vga.add(off + 1) = 0;
                            *vga.add(off + 2) = 0; *vga.add(off + 3) = 255;
                        }
                    }
                }
            }
        }
        self.cursor_x = nx as i32;
        self.cursor_y = ny as i32;
    }
}

pub enum MouseEvent {
    Move { x: usize, y: usize },
    Click { x: usize, y: usize },
}

// 8x8 arrow cursor
const CURSOR_SHAPE: [u8; 8] = [
    0b10000000,
    0b11000000,
    0b11100000,
    0b11110000,
    0b11100000,
    0b11000000,
    0b10100000,
    0b00010000,
];

const CURSOR_BORDER: [u8; 8] = [
    0b11000000,
    0b11100000,
    0b11110000,
    0b11111000,
    0b11111000,
    0b11110000,
    0b11110000,
    0b00111000,
];

fn ps2_wait_input(port_cap: u64) {
    for _ in 0..10000 {
        let status = unsafe { native::sel4_ioport_in8(port_cap, PS2_CMD) };
        if status & 0x02 == 0 { return; }
        core::hint::spin_loop();
    }
}

fn ps2_wait_output(port_cap: u64) {
    for _ in 0..10000 {
        let status = unsafe { native::sel4_ioport_in8(port_cap, PS2_CMD) };
        if status & 0x01 != 0 { return; }
        core::hint::spin_loop();
    }
}

fn ps2_write_mouse(port_cap: u64, byte: u8) {
    ps2_wait_input(port_cap);
    unsafe { native::sel4_ioport_out8(port_cap, PS2_CMD, 0xD4) }; // "next byte to mouse"
    ps2_wait_input(port_cap);
    unsafe { native::sel4_ioport_out8(port_cap, PS2_DATA, byte) };
}
