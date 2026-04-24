#![no_std]
#![no_main]

extern crate alloc;

use libprivos::allocator;
use libprivos::irq::IrqHandler;
use libprivos::mem::UntypedAllocator;
use libprivos::process::ProcessBuilder;
use sel4_sys::{
    seL4_BootInfo, seL4_CapIOPortControl, seL4_CapInitThreadCNode,
    seL4_NoError, seL4_X86_IOPortControl_Issue,
};

/// Entry point — seL4 passes a pointer to BootInfo as the first argument.
///
/// sel4runtime calls this after setting up the initial thread's stack and
/// IPC buffer. Signature must match what sel4runtime expects.
#[no_mangle]
pub extern "C" fn main(bootinfo: *const seL4_BootInfo) -> ! {
    // SAFETY: seL4 guarantees bootinfo is valid and mapped at startup.
    let bi = unsafe { &*bootinfo };

    // Initialize heap before any alloc usage.
    // SAFETY: called once, single-threaded, before any allocation.
    unsafe { allocator::init() };

    // Build the capability allocator from BootInfo.
    // SAFETY: bi is a valid seL4 BootInfo reference.
    let mut alloc = unsafe { UntypedAllocator::new(bi) };

    // --- Allocate IPC endpoints for each core service ---
    // Each endpoint is unforgeable. Services communicate ONLY through these.

    let crypto_ep = alloc.create_endpoint().expect("crypto endpoint");
    let vfs_ep    = alloc.create_endpoint().expect("vfs endpoint");
    let net_ep    = alloc.create_endpoint().expect("net endpoint");
    let driver_ep = alloc.create_endpoint().expect("driver endpoint");

    // --- Issue PCI configuration space port cap ---
    // Ports 0xCF8-0xCFF are the PCI bus master address/data registers.
    // Both driverd (bus probe) and driver-nic (device init) need this.

    let pci_port_cap = alloc.next_slot();
    let err = unsafe {
        seL4_X86_IOPortControl_Issue(
            seL4_CapIOPortControl,
            0xCF8, 0xCFF,
            seL4_CapInitThreadCNode,
            pci_port_cap,
            64,
        )
    };
    if err != seL4_NoError {
        panic!("failed to issue PCI config port cap");
    }

    // --- Register NIC IRQ ---
    // IRQ 11 is the standard PCI NIC interrupt line on QEMU x86.
    // init registers it here and passes the notification cap to driver-nic.

    let nic_irq = IrqHandler::register(11, &mut alloc)
        .expect("failed to register NIC IRQ");

    // --- Spawn services in dependency order ---
    // cryptod first — vfs and netd both depend on it.

    ProcessBuilder::new("cryptod", &mut alloc)
        .grant_endpoint(crypto_ep)
        .spawn()
        .expect("failed to start cryptod");

    ProcessBuilder::new("vfs", &mut alloc)
        .grant_endpoint(vfs_ep)
        .grant_endpoint(crypto_ep)
        .spawn()
        .expect("failed to start vfs");

    ProcessBuilder::new("netd", &mut alloc)
        .grant_endpoint(net_ep)
        .grant_endpoint(crypto_ep)
        .spawn()
        .expect("failed to start netd");

    // driverd: receives device requests; uses PCI config ports for bus probing.
    ProcessBuilder::new("driverd", &mut alloc)
        .grant_endpoint(driver_ep)
        .grant_endpoint(pci_port_cap)
        .spawn()
        .expect("failed to start driverd");

    // driver-nic: virtio-net driver; gets PCI ports + NIC IRQ notification.
    ProcessBuilder::new("driver-nic", &mut alloc)
        .grant_endpoint(driver_ep)
        .grant_endpoint(pci_port_cap)
        .grant_endpoint(nic_irq.notif_cap)
        .spawn()
        .expect("failed to start driver-nic");

    ProcessBuilder::new("compositor", &mut alloc)
        .grant_endpoint(vfs_ep)
        .grant_endpoint(net_ep)
        // Phase 8: also grant framebuffer cap
        .spawn()
        .expect("failed to start compositor");

    // Watchdog loop — init monitors child processes.
    // Phase 4: implement fault handling via a fault endpoint.
    loop {
        core::hint::spin_loop();
    }
}
