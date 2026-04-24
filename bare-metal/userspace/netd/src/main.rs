#![no_std]
#![no_main]

// Phase 7 implementation target.
// Onion-routing network daemon.

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}
