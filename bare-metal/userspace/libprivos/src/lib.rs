#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod elf;
pub mod initrd;
pub mod ipc;
pub mod ioport;
pub mod irq;
pub mod mem;
pub mod process;
pub mod vm;

#[cfg(feature = "global-allocator")]
pub mod allocator;

#[cfg(feature = "global-allocator")]
pub mod panic;
