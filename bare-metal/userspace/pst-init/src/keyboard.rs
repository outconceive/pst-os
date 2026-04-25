use sel4_sys::*;
use crate::{serial_print, serial_print_num};
use crate::sel4_shims;
use libprivos::mem::UntypedAllocator;

const PS2_DATA_PORT: u16 = 0x60;
const PS2_STATUS_PORT: u16 = 0x64;
const KEYBOARD_IRQ_PIN: u64 = 1;
const KEYBOARD_VECTOR: u64 = 33;

static SCANCODE_TO_ASCII: [u8; 128] = {
    let mut t = [0u8; 128];
    t[0x01] = 0x1B; // Esc
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

pub struct Keyboard {
    notif_cap: seL4_CPtr,
    handler_cap: seL4_CPtr,
    port_cap: seL4_CPtr,
}

pub fn setup(bootinfo: *const seL4_BootInfo, mut next_slot: u64) -> Option<Keyboard> {
    let bi = unsafe { &*bootinfo };

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
        return None;
    }
    serial_print("[kb] Port cap OK (0x60-0x64)\n");

    let mut alloc = unsafe { UntypedAllocator::new(bi) };
    let skip = (next_slot - bi.empty.start) as usize;
    for _ in 0..skip { alloc.next_slot(); }

    let notif_cap = match alloc.create_notification() {
        Ok(cap) => cap,
        Err(_) => { serial_print("[kb] Notification alloc failed\n"); return None; }
    };

    let handler_slot = alloc.next_slot();
    let err = unsafe {
        sel4_shims::seL4_IRQControl_GetIOAPIC(
            seL4_CapIRQControl,
            seL4_CapInitThreadCNode,
            handler_slot,
            64,
            0,
            KEYBOARD_IRQ_PIN,
            0,
            0,
            KEYBOARD_VECTOR,
        )
    };
    if err != seL4_NoError {
        serial_print("[kb] IOAPIC IRQ get failed: ");
        serial_print_num(err as usize);
        serial_print("\n");
        return None;
    }

    let err = unsafe { seL4_IRQHandler_SetNotification(handler_slot, notif_cap) };
    if err != seL4_NoError {
        serial_print("[kb] SetNotification failed: ");
        serial_print_num(err as usize);
        serial_print("\n");
        return None;
    }

    serial_print("[kb] Keyboard ready\n");
    Some(Keyboard { notif_cap, handler_cap: handler_slot, port_cap: kb_port_cap })
}

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

impl Keyboard {
    pub fn read_key(&self) -> u8 {
        let mut extended = false;
        loop {
            unsafe { native::sel4_wait_notification(self.notif_cap) };

            let scancode = unsafe { native::sel4_ioport_in8(self.port_cap, PS2_DATA_PORT) };
            unsafe { seL4_IRQHandler_Ack(self.handler_cap) };

            if scancode == 0xE0 { extended = true; continue; }
            if scancode & 0x80 != 0 { extended = false; continue; }

            if extended {
                extended = false;
                match scancode {
                    0x48 => return KEY_UP,
                    0x50 => return KEY_DOWN,
                    0x4B => return KEY_LEFT,
                    0x4D => return KEY_RIGHT,
                    _ => continue,
                }
            }

            // F-keys (not in ASCII table)
            match scancode {
                0x3B => return KEY_F1,
                0x3C => return KEY_F2,
                0x3D => return KEY_F3,
                0x3E => return KEY_F4,
                0x3F => return KEY_F5,
                0x40 => return KEY_F6,
                _ => {}
            }

            let ascii = SCANCODE_TO_ASCII[scancode as usize & 0x7F];
            if ascii != 0 { return ascii; }
        }
    }
}
