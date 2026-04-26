#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod styles;
pub mod line;
pub mod cursor;
pub mod history;
pub mod document;
pub mod toolbar;
