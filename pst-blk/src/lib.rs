#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod virtio;
pub mod block;

pub use block::{BlockDevice, BlockError};
