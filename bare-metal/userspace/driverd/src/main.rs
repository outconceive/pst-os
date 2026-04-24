#![no_std]
#![no_main]

// Device driver manager for Privion OS.
//
// Spawned by init with the following CSpace layout:
//   slot 0: driver_ep    — endpoint for receiving device requests from netd/vfs
//   slot 1: pci_port_cap — IOPort cap for PCI config address/data (0xCF8-0xCFF)
//
// On startup, probes PCI bus 0 to detect connected devices.
// Then idles, waiting for device capability requests from other services.

use sel4_sys::native;

// PCI configuration space I/O ports.
const PCI_CONFIG_ADDR: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

// CSpace slot layout (as granted by init).
const SLOT_DRIVER_EP:  u64 = 0;
const SLOT_PCI_PORT:   u64 = 1;

/// Build a PCI configuration address word for 32-bit register access.
#[inline(always)]
fn pci_cfg_addr(bus: u8, dev: u8, reg: u8) -> u32 {
    0x8000_0000
        | ((bus as u32) << 16)
        | ((dev as u32) << 11)
        | (reg as u32 & 0xFC)
}

/// Read a 32-bit DWORD from PCI configuration space.
#[inline]
unsafe fn pci_read32(bus: u8, dev: u8, reg: u8) -> u32 {
    native::sel4_ioport_out32(SLOT_PCI_PORT, PCI_CONFIG_ADDR, pci_cfg_addr(bus, dev, reg));
    native::sel4_ioport_in32(SLOT_PCI_PORT, PCI_CONFIG_DATA)
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Probe PCI bus 0 for connected devices.
    // Config register 0 returns [31:16]=DeviceID, [15:0]=VendorID.
    // 0xFFFF_FFFF means no device present at that slot.
    for dev in 0u8..32 {
        let id = unsafe { pci_read32(0, dev, 0) };
        if id == 0xFFFF_FFFF || id == 0x0000_0000 {
            continue;
        }
        // Device found — future phases will dispatch a driver process for each device.
        let _ = id;
    }

    // Idle: block on driver endpoint, waiting for device capability requests
    // from netd, vfs, or other services.
    loop {
        let _badge = unsafe { native::sel4_wait_notification(SLOT_DRIVER_EP) };
    }
}
