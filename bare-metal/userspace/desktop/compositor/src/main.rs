#![no_std]
#![no_main]

// Phase 8 implementation target.
// Wayland compositor — only process with framebuffer access.

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}
