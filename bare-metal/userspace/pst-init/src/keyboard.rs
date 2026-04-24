use sel4_sys::*;
use crate::{serial_print, serial_print_num};
use crate::sel4_shims;

const PS2_DATA_PORT: u16 = 0x60;
const PS2_STATUS_PORT: u16 = 0x64;
const KEYBOARD_IRQ_PIN: u64 = 1;
const KEYBOARD_VECTOR: u64 = 33;

static SCANCODE_TO_ASCII: [u8; 128] = {
    let mut t = [0u8; 128];
    t[0x02] = b'1'; t[0x03] = b'2'; t[0x04] = b'3'; t[0x05] = b'4';
    t[0x06] = b'5'; t[0x07] = b'6'; t[0x08] = b'7'; t[0x09] = b'8';
    t[0x0A] = b'9'; t[0x0B] = b'0'; t[0x0C] = b'-'; t[0x0D] = b'=';
    t[0x0E] = 0x08; // backspace
    t[0x0F] = b'\t';
    t[0x10] = b'q'; t[0x11] = b'w'; t[0x12] = b'e'; t[0x13] = b'r';
    t[0x14] = b't'; t[0x15] = b'y'; t[0x16] = b'u'; t[0x17] = b'i';
    t[0x18] = b'o'; t[0x19] = b'p'; t[0x1A] = b'['; t[0x1B] = b']';
    t[0x1C] = b'\n'; // enter
    t[0x1E] = b'a'; t[0x1F] = b's'; t[0x20] = b'd'; t[0x21] = b'f';
    t[0x22] = b'g'; t[0x23] = b'h'; t[0x24] = b'j'; t[0x25] = b'k';
    t[0x26] = b'l'; t[0x27] = b';'; t[0x28] = b'\'';
    t[0x29] = b'`';
    t[0x2B] = b'\\';
    t[0x2C] = b'z'; t[0x2D] = b'x'; t[0x2E] = b'c'; t[0x2F] = b'v';
    t[0x30] = b'b'; t[0x31] = b'n'; t[0x32] = b'm'; t[0x33] = b',';
    t[0x34] = b'.'; t[0x35] = b'/';
    t[0x39] = b' '; // space
    t
};

pub fn run(bootinfo: *const seL4_BootInfo, mut next_slot: u64, fb_vaddr: u64) {
    let bi = unsafe { &*bootinfo };

    // Issue I/O port cap for keyboard ports 0x60-0x64
    let kb_port_cap = next_slot;
    next_slot += 1;
    let err = unsafe {
        seL4_X86_IOPortControl_Issue(
            seL4_CapIOPortControl, PS2_DATA_PORT, PS2_STATUS_PORT,
            seL4_CapInitThreadCNode, kb_port_cap, 64,
        )
    };
    if err != seL4_NoError {
        serial_print("[kb] Port cap failed: ");
        serial_print_num(err as usize);
        serial_print("\n");
        return;
    }
    serial_print("[kb] Port cap OK (0x60-0x64)\n");

    // Allocate notification for keyboard IRQ
    let mut alloc = unsafe { UntypedAllocator::new(bi) };
    let skip = (next_slot - bi.empty.start) as usize;
    for _ in 0..skip { alloc.next_slot(); }

    let notif_cap = match alloc.create_notification() {
        Ok(cap) => cap,
        Err(_) => { serial_print("[kb] Notification alloc failed\n"); return; }
    };

    // Get IRQ handler via IOAPIC (CONFIG_IRQ_IOAPIC=1)
    let handler_slot = alloc.next_slot();
    let err = unsafe {
        sel4_shims::seL4_IRQControl_GetIOAPIC(
            seL4_CapIRQControl,
            seL4_CapInitThreadCNode,
            handler_slot,
            64,
            0,                // ioapic 0
            KEYBOARD_IRQ_PIN, // pin 1
            0,                // edge triggered
            0,                // active high
            KEYBOARD_VECTOR,
        )
    };
    if err != seL4_NoError {
        serial_print("[kb] IOAPIC IRQ get failed: ");
        serial_print_num(err as usize);
        serial_print("\n");
        return;
    }

    // Bind notification to handler
    let err = unsafe { seL4_IRQHandler_SetNotification(handler_slot, notif_cap) };
    if err != seL4_NoError {
        serial_print("[kb] SetNotification failed: ");
        serial_print_num(err as usize);
        serial_print("\n");
        return;
    }

    serial_print("[kb] Keyboard IRQ registered, entering input loop\n");

    let vga = fb_vaddr as *mut u8;
    let mut cursor_row: usize = 22;
    let mut cursor_col: usize = 2;
    let attr: u8 = 0x0F; // white on black

    // Prompt
    write_char(vga, cursor_row, cursor_col, b'>', attr);
    cursor_col += 1;
    write_char(vga, cursor_row, cursor_col, b' ', attr);
    cursor_col += 1;

    loop {
        unsafe { native::sel4_wait_notification(notif_cap) };

        let scancode = unsafe { native::sel4_ioport_in8(kb_port_cap, PS2_DATA_PORT) };

        // Ignore key releases (bit 7 set) and extended scancodes
        if scancode & 0x80 != 0 || scancode == 0xE0 {
            unsafe { seL4_IRQHandler_Ack(handler_slot) };
            continue;
        }

        let ascii = SCANCODE_TO_ASCII[scancode as usize & 0x7F];
        if ascii == 0 {
            unsafe { seL4_IRQHandler_Ack(handler_slot) };
            continue;
        }

        if ascii == b'\n' {
            serial_print("\n");
            cursor_row += 1;
            cursor_col = 2;
            if cursor_row >= 24 { cursor_row = 22; }
            write_char(vga, cursor_row, cursor_col, b'>', attr);
            cursor_col += 1;
            write_char(vga, cursor_row, cursor_col, b' ', attr);
            cursor_col += 1;
        } else if ascii == 0x08 {
            if cursor_col > 4 {
                cursor_col -= 1;
                write_char(vga, cursor_row, cursor_col, b' ', attr);
            }
        } else {
            unsafe { crate::debug_putchar(ascii) };
            if cursor_col < 78 {
                write_char(vga, cursor_row, cursor_col, ascii, attr);
                cursor_col += 1;
            }
        }

        unsafe { seL4_IRQHandler_Ack(handler_slot) };
    }
}

fn write_char(vga: *mut u8, row: usize, col: usize, ch: u8, attr: u8) {
    let offset = (row * 80 + col) * 2;
    if offset + 1 < 80 * 25 * 2 {
        unsafe {
            *vga.add(offset) = ch;
            *vga.add(offset + 1) = attr;
        }
    }
}

use libprivos::mem::UntypedAllocator;
