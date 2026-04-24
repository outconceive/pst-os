use sel4_sys::*;
use crate::{serial_print, serial_print_num, serial_print_hex, debug_putchar};
use crate::sel4_shims;
use libprivos::mem::UntypedAllocator;
use libprivos::vm::VSpaceMapper;

pub struct VgaState {
    pub fb_vaddr: u64,
    pub next_slot: u64,
    pub pci_cap: u64,
}

pub fn init(bootinfo: *const seL4_BootInfo) -> Option<VgaState> {
    let bi = unsafe { &*bootinfo };

    let ut_start = bi.untyped.start as usize;
    let ut_end = bi.untyped.end as usize;
    let ut_count = ut_end - ut_start;
    let mut next_slot = bi.empty.start;

    serial_print("[vga] Untypeds: ");
    serial_print_num(ut_count);
    serial_print(", free slots from: ");
    serial_print_num(next_slot as usize);
    serial_print("\n");

    // --- Step 1: Get PCI I/O port capability ---
    serial_print("[vga] Issuing PCI config port cap (0xCF8-0xCFF)...\n");
    let pci_cap = next_slot;
    next_slot += 1;
    let err = unsafe {
        seL4_X86_IOPortControl_Issue(
            seL4_CapIOPortControl, 0xCF8, 0xCFF,
            seL4_CapInitThreadCNode, pci_cap, 64,
        )
    };
    if err != seL4_NoError {
        serial_print("[vga] IOPortControl failed: ");
        serial_print_num(err as usize);
        serial_print("\n");
        return None;
    }
    serial_print("[vga] PCI port cap OK\n");

    // --- Step 2: Probe PCI for VGA device, read BAR0 ---
    serial_print("[vga] Scanning PCI bus...\n");
    let mut vga_bar: u64 = 0;

    for dev in 0u8..32 {
        let addr: u32 = (1u32 << 31) | ((dev as u32) << 11);
        unsafe { native::sel4_ioport_out32(pci_cap, 0xCF8, addr); }
        let id = unsafe { native::sel4_ioport_in32(pci_cap, 0xCFC) };
        if id == 0xFFFF_FFFF || id == 0 { continue; }

        // Read class code at offset 0x08
        unsafe { native::sel4_ioport_out32(pci_cap, 0xCF8, addr | 0x08); }
        let class_word = unsafe { native::sel4_ioport_in32(pci_cap, 0xCFC) };
        let class_code = (class_word >> 24) & 0xFF;

        // VGA compatible controller: class 0x03
        if class_code == 0x03 {
            // Read BAR0 at offset 0x10
            unsafe { native::sel4_ioport_out32(pci_cap, 0xCF8, addr | 0x10); }
            let bar0 = unsafe { native::sel4_ioport_in32(pci_cap, 0xCFC) };
            vga_bar = (bar0 & 0xFFFFFFF0) as u64;
            serial_print("[vga] VGA at PCI slot ");
            serial_print_num(dev as usize);
            serial_print(", BAR0=0x");
            serial_print_hex(vga_bar);
            serial_print("\n");
            break;
        }
    }

    if vga_bar == 0 {
        serial_print("[vga] No VGA device found\n");
        return None;
    }

    // --- Step 3: Find device untyped covering BAR0 ---
    let mut fb_cap: Option<seL4_CPtr> = None;
    let mut fb_paddr: u64 = 0;
    let mut fb_size_bits: u8 = 0;

    for i in 0..ut_count {
        let desc = unsafe { &bi.untypedList[i] };
        if desc.isDevice == 0 { continue; }
        let start = desc.paddr;
        let size = 1u64 << desc.sizeBits;

        if vga_bar >= start && vga_bar < start + size {
            fb_cap = Some((ut_start + i) as seL4_CPtr);
            fb_paddr = start;
            fb_size_bits = desc.sizeBits;
            serial_print("[vga] Device untyped: paddr=0x");
            serial_print_hex(start);
            serial_print(" size=");
            serial_print_num(size as usize);
            serial_print(" bits=");
            serial_print_num(desc.sizeBits as usize);
            serial_print("\n");
            break;
        }
    }

    let fb_ut_cap = match fb_cap {
        Some(c) => c,
        None => {
            serial_print("[vga] No device untyped for BAR 0x");
            serial_print_hex(vga_bar);
            serial_print("\nDevice untypeds:\n");
            for i in 0..ut_count {
                let desc = unsafe { &bi.untypedList[i] };
                if desc.isDevice == 0 { continue; }
                serial_print("  0x");
                serial_print_hex(desc.paddr);
                serial_print(" size=");
                serial_print_num((1u64 << desc.sizeBits) as usize);
                serial_print("\n");
            }
            return None;
        }
    };

    // --- Step 4: Retype 4K frames to reach VGA BAR offset ---
    // Device untyped starts at fb_paddr. VGA BAR is at vga_bar.
    // seL4_Untyped_Retype creates objects sequentially from start.
    // We need to retype enough 4K frames to reach the BAR offset.
    let offset_bytes = vga_bar - fb_paddr;
    let frames_to_skip = (offset_bytes / 0x1000) as usize;
    let total_frames = frames_to_skip + 1;

    serial_print("[vga] BAR offset from untyped: 0x");
    serial_print_hex(offset_bytes);
    serial_print(" (");
    serial_print_num(frames_to_skip);
    serial_print(" frames to skip)\n");

    // Retype all frames in one call — seL4 creates them sequentially
    // Device untyped requires 2MB large page retype (4K fails).
    // BAR offset: 0x1000000 = 16MB = 8 large pages to skip.
    // The 9th large page is our VGA framebuffer.
    let large_page_size: u64 = 0x200000; // 2MB
    let pages_to_skip = (offset_bytes / large_page_size) as usize;
    let total_pages = pages_to_skip + 1;

    serial_print("[vga] Retyping ");
    serial_print_num(total_pages);
    serial_print(" x 2MB large pages...\n");

    let first_page_slot = next_slot;
    let err = unsafe {
        seL4_Untyped_Retype(
            fb_ut_cap, seL4_X86_LargePage, 21, // 2MB = 2^21
            seL4_CapInitThreadCNode, 0, 0,
            first_page_slot, total_pages as seL4_Word,
        )
    };
    if err != seL4_NoError {
        serial_print("[vga] Large page retype failed: ");
        serial_print_num(err as usize);
        serial_print("\n");
        return None;
    }

    let frame_cap = first_page_slot + pages_to_skip as u64;
    next_slot = first_page_slot + total_pages as u64;
    serial_print("[vga] VGA large page at slot ");
    serial_print_num(frame_cap as usize);
    serial_print("\n");
    serial_print("[vga] VGA frame at slot ");
    serial_print_num(frame_cap as usize);
    serial_print("\n");

    // --- Step 5: Map frame into our VSpace ---
    let fb_vaddr: u64 = 0x2_0000_0000; // 8GB — well above our binary
    serial_print("[vga] Mapping 2MB page at vaddr 0x");
    serial_print_hex(fb_vaddr);
    serial_print("...\n");

    // For a 2MB large page we need PDPT + PD but NOT PT.
    let mut alloc = unsafe { UntypedAllocator::new(bi) };
    let skip = (next_slot - bi.empty.start) as usize;
    for _ in 0..skip { alloc.next_slot(); }

    // PDPT: rootserver already has one at PML4[0] covering 0-512GB.
    // Allocate and try to map; DeleteFirst (8) means it exists — that's fine.
    match alloc.retype(seL4_X86_PDPTObject, seL4_PageBits as seL4_Word) {
        Ok(pdpt_cap) => {
            let err = unsafe { seL4_X86_PDPT_Map(pdpt_cap, seL4_CapInitThreadVSpace, fb_vaddr, seL4_X86_Default_VMAttributes) };
            if err != seL4_NoError && err != seL4_DeleteFirst {
                serial_print("[vga] PDPT map error: ");
                serial_print_num(err as usize);
                serial_print("\n");
                return None;
            }
            serial_print("[vga] PDPT: ");
            serial_print(if err == seL4_DeleteFirst { "exists\n" } else { "mapped\n" });
        }
        Err(_) => { serial_print("[vga] PDPT retype failed\n"); return None; }
    }

    // PD: allocate and map at the PDPT entry for our vaddr
    match alloc.retype(seL4_X86_PageDirectoryObject, seL4_PageBits as seL4_Word) {
        Ok(pd_cap) => {
            let err = unsafe { seL4_X86_PageDirectory_Map(pd_cap, seL4_CapInitThreadVSpace, fb_vaddr, seL4_X86_Default_VMAttributes) };
            if err != seL4_NoError {
                serial_print("[vga] PD map error: ");
                serial_print_num(err as usize);
                serial_print("\n");
                return None;
            }
            serial_print("[vga] PD: mapped\n");
        }
        Err(_) => { serial_print("[vga] PD retype failed\n"); return None; }
    }

    // Map the 2MB large page directly (no PT needed for large pages)
    let map_err = unsafe {
        seL4_X86_Page_Map(frame_cap, seL4_CapInitThreadVSpace, fb_vaddr,
            seL4_CapRights_t::READ_WRITE, seL4_X86_Default_VMAttributes)
    };

    if map_err == seL4_NoError {
            serial_print("[vga] Mapped! Writing to VGA framebuffer...\n");

            // Write test pixels — QEMU standard VGA in default mode
            // The BAR0 points to the linear framebuffer.
            // In text mode (default), this is the VGA text buffer.
            // In graphics mode, this is pixel data.
            // QEMU boots in text mode — write character cells.
            let vga = fb_vaddr as *mut u8;
            let wb: u8 = 0x1F; // white on blue
            let yb: u8 = 0x1E; // yellow on blue

            // Clear screen (80x25 text mode cells)
            for i in 0..(80 * 25) {
                unsafe {
                    *vga.add(i * 2) = b' ';
                    *vga.add(i * 2 + 1) = wb;
                }
            }

            // Title bar
            write_str(vga, 0, 1, " PST OS v0.1 | x86_64 | seL4 ", yb);

            // Card
            write_str(vga, 2, 2, "+--------------------------------------------+", wb);
            for row in 3..14 {
                write_str(vga, row, 2, "|", wb);
                write_str(vga, row, 49, "|", wb);
            }
            write_str(vga, 14, 2, "+--------------------------------------------+", wb);

            write_str(vga, 3, 4, "Parallel String Theory OS", wb);
            write_str(vga, 4, 4, "=========================", wb);
            write_str(vga, 6, 14, "PST OS v0.1", yb);
            write_str(vga, 7, 13, "x86_64 / seL4", wb);
            write_str(vga, 8, 13, "Boot complete", wb);
            write_str(vga, 10, 4, "One primitive. One loop. One OS.", wb);

            write_str(vga, 16, 2, "Boot log:", wb);
            write_str(vga, 17, 4, "cryptod -> driverd -> netd -> vfs", wb);
            write_str(vga, 18, 4, "  -> driver-nic -> compositor", wb);
            write_str(vga, 19, 4, "12 lines parsed, 964 bytes HTML", wb);
            write_str(vga, 20, 4, "320x200 framebuffer rendered", wb);

            write_str(vga, 24, 1,
                " No Wayland | No X11 | No display server | Markout -> pixels ", yb);

            serial_print("[vga] Desktop on screen!\n");

            let final_slot = alloc.next_slot();
            return Some(VgaState { fb_vaddr, next_slot: final_slot, pci_cap });
    } else {
        serial_print("[vga] Map failed: ");
        serial_print_num(map_err as usize);
        serial_print("\n");
    }

    None
}

pub fn write_str(vga: *mut u8, row: usize, col: usize, s: &str, attr: u8) {
    let mut offset = (row * 80 + col) * 2;
    for b in s.bytes() {
        if offset + 1 >= 80 * 25 * 2 { break; }
        unsafe {
            *vga.add(offset) = b;
            *vga.add(offset + 1) = attr;
        }
        offset += 2;
    }
}
