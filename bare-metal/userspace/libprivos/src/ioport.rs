// Safe x86 I/O port wrapper for Privion OS.
//
// Uses sel4_sys::native (Rust inline asm) so service processes do not need
// libsel4's TLS setup. Init does not call these methods directly.

use sel4_sys::{seL4_CPtr, native};

pub struct IoPort { cap: seL4_CPtr }

impl IoPort {
    /// # Safety: `cap` must be a valid IOPort cap in the current CSpace.
    pub unsafe fn from_cap(cap: seL4_CPtr) -> Self { Self { cap } }
    pub fn cap(&self) -> seL4_CPtr { self.cap }

    pub fn in8 (&self, port: u16) -> u8  { unsafe { native::sel4_ioport_in8 (self.cap, port) } }
    pub fn in16(&self, port: u16) -> u16 { unsafe { native::sel4_ioport_in16(self.cap, port) } }
    pub fn in32(&self, port: u16) -> u32 { unsafe { native::sel4_ioport_in32(self.cap, port) } }

    pub fn out8 (&self, port: u16, v: u8)  { unsafe { native::sel4_ioport_out8 (self.cap, port, v) } }
    pub fn out16(&self, port: u16, v: u16) { unsafe { native::sel4_ioport_out16(self.cap, port, v) } }
    pub fn out32(&self, port: u16, v: u32) { unsafe { native::sel4_ioport_out32(self.cap, port, v) } }
}
