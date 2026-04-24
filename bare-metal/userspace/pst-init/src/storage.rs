use alloc::string::String;
use alloc::vec::Vec;
use sel4_sys::*;
use crate::{serial_print, serial_print_num, serial_print_hex};
use pst_blk::block::BLOCK_SIZE;
use pst_blk::virtio::*;

const MAGIC: [u8; 4] = *b"PSTD";

pub struct Storage {
    port_cap: u64,
    base_port: u16,
    queue_vaddr: u64,
    queue_paddr: u64,
    req_vaddr: u64,
    req_paddr: u64,
    last_used_idx: u16,
    capacity: u64,
}

pub fn setup(bootinfo: *const seL4_BootInfo, pci_cap: u64, mut next_slot: u64) -> (Option<Storage>, u64) {
    let bi = unsafe { &*bootinfo };

    // Scan PCI for virtio-blk
    let mut blk_bar: u64 = 0;
    for dev in 0u8..32 {
        let addr: u32 = (1u32 << 31) | ((dev as u32) << 11);
        unsafe { native::sel4_ioport_out32(pci_cap, 0xCF8, addr); }
        let id = unsafe { native::sel4_ioport_in32(pci_cap, 0xCFC) };
        if id == 0xFFFF_FFFF || id == 0 { continue; }

        let vendor = (id & 0xFFFF) as u16;
        let device = ((id >> 16) & 0xFFFF) as u16;

        if vendor == VIRTIO_VENDOR && device == VIRTIO_BLK_DEVICE_LEGACY {
            unsafe { native::sel4_ioport_out32(pci_cap, 0xCF8, addr | 0x10); }
            let bar0 = unsafe { native::sel4_ioport_in32(pci_cap, 0xCFC) };
            if bar0 & 1 == 1 {
                blk_bar = (bar0 & 0xFFFFFFFC) as u64;
                serial_print("[blk] virtio-blk at slot ");
                serial_print_num(dev as usize);
                serial_print(", port=0x");
                serial_print_hex(blk_bar);
                serial_print("\n");
                break;
            }
        }
    }

    if blk_bar == 0 {
        serial_print("[blk] No virtio-blk found (add -drive file=disk.img,if=virtio to QEMU)\n");
        return (None, next_slot);
    }

    let base_port = blk_bar as u16;

    // Issue port cap
    let port_cap = next_slot;
    next_slot += 1;
    let err = unsafe {
        seL4_X86_IOPortControl_Issue(
            seL4_CapIOPortControl, base_port, base_port + 0x3F,
            seL4_CapInitThreadCNode, port_cap, 64,
        )
    };
    if err != seL4_NoError {
        serial_print("[blk] Port cap failed: ");
        serial_print_num(err as usize);
        serial_print("\n");
        return (None, next_slot);
    }

    // Allocate frames for virtqueue and request buffers
    let mut alloc = unsafe { libprivos::mem::UntypedAllocator::new(bi) };
    let skip = (next_slot - bi.empty.start) as usize;
    for _ in 0..skip { alloc.next_slot(); }

    let queue_frame = match alloc.alloc_frame() {
        Ok(f) => f, Err(_) => { serial_print("[blk] Queue alloc failed\n"); return (None, alloc.next_slot()); }
    };
    let req_frame = match alloc.alloc_frame() {
        Ok(f) => f, Err(_) => { serial_print("[blk] Req alloc failed\n"); return (None, alloc.next_slot()); }
    };

    // Map a PT for our storage address range
    let queue_vaddr: u64 = 0x2_0020_0000;
    let req_vaddr: u64 = 0x2_0020_1000;

    match alloc.retype(seL4_X86_PageTableObject, seL4_PageBits as seL4_Word) {
        Ok(pt) => {
            let err = unsafe { seL4_X86_PageTable_Map(pt, seL4_CapInitThreadVSpace, queue_vaddr, seL4_X86_Default_VMAttributes) };
            if err != seL4_NoError && err != seL4_DeleteFirst {
                serial_print("[blk] PT map err: "); serial_print_num(err as usize); serial_print("\n");
                return (None, alloc.next_slot());
            }
        }
        Err(_) => { serial_print("[blk] PT alloc failed\n"); return (None, alloc.next_slot()); }
    }

    let err = unsafe { seL4_X86_Page_Map(queue_frame, seL4_CapInitThreadVSpace, queue_vaddr, seL4_CapRights_t::READ_WRITE, seL4_X86_Default_VMAttributes) };
    if err != seL4_NoError { serial_print("[blk] Queue map err\n"); return (None, alloc.next_slot()); }

    let err = unsafe { seL4_X86_Page_Map(req_frame, seL4_CapInitThreadVSpace, req_vaddr, seL4_CapRights_t::READ_WRITE, seL4_X86_Default_VMAttributes) };
    if err != seL4_NoError { serial_print("[blk] Req map err\n"); return (None, alloc.next_slot()); }

    // Get physical addresses for DMA
    // seL4_X86_Page_GetAddress returns the physical address of a frame
    let queue_paddr = unsafe { page_get_paddr(queue_frame) };
    let req_paddr = unsafe { page_get_paddr(req_frame) };
    serial_print("[blk] Queue paddr=0x"); serial_print_hex(queue_paddr);
    serial_print(" Req paddr=0x"); serial_print_hex(req_paddr); serial_print("\n");

    // Zero queue memory
    unsafe { core::ptr::write_bytes(queue_vaddr as *mut u8, 0, 4096); }

    // Initialize virtio device via I/O ports
    port_out8(port_cap, base_port + REG_DEVICE_STATUS, 0); // reset
    port_out8(port_cap, base_port + REG_DEVICE_STATUS, STATUS_ACKNOWLEDGE);
    port_out8(port_cap, base_port + REG_DEVICE_STATUS, STATUS_ACKNOWLEDGE | STATUS_DRIVER);

    let _features = port_in32(port_cap, base_port + REG_DEVICE_FEATURES);
    port_out32(port_cap, base_port + REG_GUEST_FEATURES, 0);

    // Select queue 0
    port_out16(port_cap, base_port + REG_QUEUE_SELECT, 0);
    let qsize = port_in16(port_cap, base_port + REG_QUEUE_SIZE);
    serial_print("[blk] Queue size: "); serial_print_num(qsize as usize); serial_print("\n");

    // Set queue address (physical page frame number)
    port_out32(port_cap, base_port + REG_QUEUE_ADDRESS, (queue_paddr >> 12) as u32);

    // Read capacity
    let cap_lo = port_in32(port_cap, base_port + REG_CAPACITY_LO) as u64;
    let cap_hi = port_in32(port_cap, base_port + REG_CAPACITY_HI) as u64;
    let capacity = (cap_hi << 32) | cap_lo;

    port_out8(port_cap, base_port + REG_DEVICE_STATUS,
        STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_DRIVER_OK);

    serial_print("[blk] Ready, ");
    serial_print_num(capacity as usize);
    serial_print(" sectors (");
    serial_print_num((capacity * 512 / 1024) as usize);
    serial_print(" KiB)\n");

    let ns = alloc.next_slot();
    (Some(Storage {
        port_cap, base_port, queue_vaddr, queue_paddr,
        req_vaddr, req_paddr, last_used_idx: 0, capacity,
    }), ns)
}

impl Storage {
    fn do_request(&mut self, typ: u32, sector: u64, data: Option<&[u8; BLOCK_SIZE]>) -> bool {
        let req = self.req_vaddr as *mut u8;
        let req_paddr = self.req_paddr;

        unsafe {
            // Header at offset 0
            let hdr = req as *mut VirtioBlkReqHeader;
            (*hdr).typ = typ;
            (*hdr)._reserved = 0;
            (*hdr).sector = sector;

            // Data at offset 16
            let data_ptr = req.add(16);
            if let Some(d) = data {
                core::ptr::copy_nonoverlapping(d.as_ptr(), data_ptr, BLOCK_SIZE);
            }

            // Status at offset 16 + 512
            let status_ptr = req.add(16 + BLOCK_SIZE);
            *status_ptr = 0xFF;

            // Set up descriptor chain in queue memory
            let desc = self.queue_vaddr as *mut VirtqDesc;

            (*desc.add(0)).addr = req_paddr;
            (*desc.add(0)).len = 16;
            (*desc.add(0)).flags = VIRTQ_DESC_F_NEXT;
            (*desc.add(0)).next = 1;

            (*desc.add(1)).addr = req_paddr + 16;
            (*desc.add(1)).len = BLOCK_SIZE as u32;
            (*desc.add(1)).flags = if typ == VIRTIO_BLK_T_IN { VIRTQ_DESC_F_WRITE | VIRTQ_DESC_F_NEXT } else { VIRTQ_DESC_F_NEXT };
            (*desc.add(1)).next = 2;

            (*desc.add(2)).addr = req_paddr + 16 + BLOCK_SIZE as u64;
            (*desc.add(2)).len = 1;
            (*desc.add(2)).flags = VIRTQ_DESC_F_WRITE;
            (*desc.add(2)).next = 0;

            // Available ring is after descriptors (16 descs * 16 bytes = 256)
            let avail = (self.queue_vaddr + 256) as *mut VirtqAvail;
            let idx = (*avail).idx;
            (*avail).ring[(idx % 16) as usize] = 0;
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            (*avail).idx = idx.wrapping_add(1);
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

            // Notify queue 0
            port_out16(self.port_cap, self.base_port + REG_QUEUE_NOTIFY, 0);

            // Poll used ring (after avail, at page-aligned offset)
            let used = (self.queue_vaddr + 4096 - 256) as *mut VirtqUsed;
            let mut spins = 0u32;
            while core::ptr::read_volatile(&(*used).idx) == self.last_used_idx {
                core::hint::spin_loop();
                spins += 1;
                if spins > 10_000_000 { return false; }
            }
            self.last_used_idx = (*used).idx;

            *status_ptr == 0
        }
    }

    pub fn read_block(&mut self, lba: u64, buf: &mut [u8; BLOCK_SIZE]) -> bool {
        if lba >= self.capacity { return false; }
        if !self.do_request(VIRTIO_BLK_T_IN, lba, None) { return false; }
        unsafe {
            let data_ptr = (self.req_vaddr as *const u8).add(16);
            core::ptr::copy_nonoverlapping(data_ptr, buf.as_mut_ptr(), BLOCK_SIZE);
        }
        true
    }

    pub fn write_block(&mut self, lba: u64, buf: &[u8; BLOCK_SIZE]) -> bool {
        if lba >= self.capacity { return false; }
        self.do_request(VIRTIO_BLK_T_OUT, lba, Some(buf))
    }

    pub fn save_desktop(&mut self, windows: &[(String, Vec<String>)]) -> bool {
        let mut block = [0u8; BLOCK_SIZE];
        block[0..4].copy_from_slice(&MAGIC);
        block[4] = windows.len() as u8;
        if !self.write_block(0, &block) { return false; }

        for (i, (title, lines)) in windows.iter().enumerate() {
            block = [0u8; BLOCK_SIZE];
            let tb = title.as_bytes();
            let tlen = tb.len().min(63);
            block[0] = tlen as u8;
            block[1..1 + tlen].copy_from_slice(&tb[..tlen]);
            block[64] = lines.len() as u8;

            let mut off = 128usize;
            for line in lines {
                let lb = line.as_bytes();
                let ll = lb.len().min(127);
                if off + 1 + ll > BLOCK_SIZE { break; }
                block[off] = ll as u8;
                block[off + 1..off + 1 + ll].copy_from_slice(&lb[..ll]);
                off += 1 + ll;
            }
            if !self.write_block(1 + i as u64, &block) { return false; }
        }
        serial_print("[blk] Desktop saved\n");
        true
    }

    pub fn load_desktop(&mut self) -> Option<Vec<(String, Vec<String>)>> {
        let mut block = [0u8; BLOCK_SIZE];
        if !self.read_block(0, &mut block) { return None; }
        if block[0..4] != MAGIC { return None; }

        let count = block[4] as usize;
        let mut windows = Vec::new();

        for i in 0..count {
            if !self.read_block(1 + i as u64, &mut block) { break; }
            let tlen = block[0] as usize;
            let title = core::str::from_utf8(&block[1..1 + tlen]).unwrap_or("?");
            let lcount = block[64] as usize;

            let mut lines = Vec::new();
            let mut off = 128usize;
            for _ in 0..lcount {
                if off >= BLOCK_SIZE { break; }
                let ll = block[off] as usize;
                off += 1;
                if off + ll > BLOCK_SIZE { break; }
                let s = core::str::from_utf8(&block[off..off + ll]).unwrap_or("");
                lines.push(String::from(s));
                off += ll;
            }
            windows.push((String::from(title), lines));
        }
        serial_print("[blk] Desktop restored\n");
        Some(windows)
    }
}

// I/O port helpers using native seL4 syscalls
fn port_in8(cap: u64, port: u16) -> u8 { unsafe { native::sel4_ioport_in8(cap, port) } }
fn port_in16(cap: u64, port: u16) -> u16 { unsafe { native::sel4_ioport_in16(cap, port) } }
fn port_in32(cap: u64, port: u16) -> u32 { unsafe { native::sel4_ioport_in32(cap, port) } }
fn port_out8(cap: u64, port: u16, v: u8) { unsafe { native::sel4_ioport_out8(cap, port, v); } }
fn port_out16(cap: u64, port: u16, v: u16) { unsafe { native::sel4_ioport_out16(cap, port, v); } }
fn port_out32(cap: u64, port: u16, v: u32) { unsafe { native::sel4_ioport_out32(cap, port, v); } }

unsafe fn page_get_paddr(frame_cap: u64) -> u64 {
    // seL4_X86_Page_GetAddress: label=39, extraCaps=0, length=0
    // Returns paddr in MR0
    use crate::sel4_shims;
    sel4_shims::page_get_address(frame_cap)
}
