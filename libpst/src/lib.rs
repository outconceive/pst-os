#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod table;
pub mod offset;
pub mod constraint;
pub mod solver;
pub mod concurrent;
