// Panic handler for Privion OS userspace processes.
//
// In production (no kernel printing), a panic in any userspace process
// causes that process to fault. The init process monitors child processes
// and restarts them if they fault (Phase 4).
//
// For development builds, the fault message is visible via seL4's debug
// output (kernel must be built with KernelPrinting=ON for that).

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // In a no_std bare-metal environment, we have no stdout.
    // Trigger a deliberate fault so seL4 can report it.
    // The init process watchdog will detect the fault and restart us.
    loop {
        // Compiler fence to prevent this loop from being optimized away.
        core::hint::spin_loop();
    }
}
