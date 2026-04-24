#![no_std]
#![no_main]

// Phase 5 implementation target.
// NVMe block device driver.

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}
