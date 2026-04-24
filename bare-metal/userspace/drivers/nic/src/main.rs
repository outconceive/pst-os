#![no_std]
#![no_main]

// Virtio-net NIC driver for Privion OS.
//
// Spawned by init with the following CSpace layout:
//   slot 0: driver_ep   — endpoint for sending/receiving frames with netd
//   slot 1: pci_port_cap — IOPort cap for PCI config space (0xCF8-0xCFF)
//   slot 2: irq_notif   — notification cap bound to NIC IRQ (IRQ 11)
//
// Startup sequence:
//   1. Probe PCI bus for a virtio-net device (vendor 0x1AF4, device 0x1000).
//   2. Enter the main IRQ wait loop — each IRQ signals an incoming packet
//      or TX completion from the virtio queues.

use sel4_sys::native;

// PCI configuration space I/O ports (bus master registers).
const PCI_CONFIG_ADDR: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

// CSpace slot layout.
const SLOT_DRIVER_EP:  u64 = 0;
const SLOT_PCI_PORT:   u64 = 1;
const SLOT_IRQ_NOTIF:  u64 = 2;

// Virtio-net PCI vendor and device identifiers.
const VIRTIO_VENDOR_ID: u16 = 0x1AF4;
const VIRTIO_NET_DEV_ID: u16 = 0x1000;

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

/// Scan PCI bus 0 for a virtio-net device. Returns the device slot if found.
unsafe fn find_virtio_net() -> Option<u8> {
    for dev in 0u8..32 {
        let word = pci_read32(0, dev, 0);
        let vendor = (word & 0xFFFF) as u16;
        let device = (word >> 16) as u16;
        if vendor == VIRTIO_VENDOR_ID && device == VIRTIO_NET_DEV_ID {
            return Some(dev);
        }
    }
    None
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Locate the virtio-net device on the PCI bus.
    let _nic_slot = unsafe { find_virtio_net() };

    // Phase 5: device located. Future phases will:
    //   - Read BAR0 to find the virtio I/O port base.
    //   - Negotiate virtio features and set up virtqueue rings.
    //   - Register receive/transmit buffers.

    // Main loop: block until the NIC fires an IRQ (packet received or TX done).
    loop {
        let _badge = unsafe { native::sel4_wait_notification(SLOT_IRQ_NOTIF) };
        // Phase 6: dequeue virtio RX/TX rings and forward frames to netd.
    }
}
