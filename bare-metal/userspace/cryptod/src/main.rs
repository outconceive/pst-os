#![no_std]
#![no_main]

// Phase 3+ implementation target.
// Cryptography service — first service started by init.

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}
