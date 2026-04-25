use alloc::string::String;
use alloc::vec::Vec;
use sel4_sys::*;
use crate::{serial_print, serial_print_num, serial_print_hex};
use pst_blk::block::BLOCK_SIZE;
use pst_blk::virtio::*;

const MAGIC: [u8; 4] = *b"PSTD";

#[repr(C, align(4096))]
struct PageBuf([u8; 4096]);

static mut QUEUE_BUF: PageBuf = PageBuf([0u8; 4096]);
static mut REQ_BUF: PageBuf = PageBuf([0u8; 4096]);

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

    // Use static BSS buffers — already mapped in the rootserver's VSpace
    let queue_vaddr = unsafe { &mut QUEUE_BUF.0 as *mut [u8; 4096] as u64 };
    let req_vaddr = unsafe { &mut REQ_BUF.0 as *mut [u8; 4096] as u64 };

    // Get physical addresses for DMA via seL4_X86_Page_GetAddress
    // For BSS pages, we need the frame cap. Use the userImageFrames from bootinfo.
    // Simpler: the rootserver's pages are identity-mapped by seL4 at low addresses,
    // so for QEMU with <2GB RAM, vaddr ≈ paddr for the rootserver image.
    let queue_paddr = queue_vaddr;
    let req_paddr = req_vaddr;

    serial_print("[blk] Queue vaddr=0x"); serial_print_hex(queue_vaddr);
    serial_print(" Req vaddr=0x"); serial_print_hex(req_vaddr); serial_print("\n");

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

    (Some(Storage {
        port_cap, base_port, queue_vaddr, queue_paddr,
        req_vaddr, req_paddr, last_used_idx: 0, capacity,
    }), next_slot)
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

// ---------------------------------------------------------------------------
// Flat filesystem: files stored by name on blocks 32+
// Block 32: directory (magic PSTF, count, then entries)
// Each entry: 1-byte name_len, 59-byte name, 2-byte start_block, 2-byte size
// Blocks 64+: file content
// ---------------------------------------------------------------------------

const FS_MAGIC: [u8; 4] = *b"PSTF";
const DIR_BLOCK: u64 = 32;
const DIR_ENTRIES_START: u64 = 33;
const CONTENT_START: u64 = 64;
const MAX_FILES: usize = 16;
const ENTRY_SIZE: usize = 64;

impl Storage {
    pub fn save_file(&mut self, path: &str, content: &str) -> bool {
        let mut dir = self.read_directory();

        // Find existing or allocate new slot
        let slot = dir.iter().position(|e| e.name == path);
        let content_block = if let Some(i) = slot {
            dir[i].start_block
        } else {
            if dir.len() >= MAX_FILES { return false; }
            let next = if dir.is_empty() { CONTENT_START }
                else { dir.iter().map(|e| e.start_block + blocks_needed(e.size)).max().unwrap_or(CONTENT_START) };
            next
        };

        let bytes = content.as_bytes();
        let needed = blocks_needed(bytes.len());

        // Write content
        let mut block = [0u8; BLOCK_SIZE];
        let mut offset = 0usize;
        for b in 0..needed {
            block = [0u8; BLOCK_SIZE];
            let chunk = (bytes.len() - offset).min(BLOCK_SIZE);
            block[..chunk].copy_from_slice(&bytes[offset..offset + chunk]);
            if !self.write_block(content_block + b, &block) { return false; }
            offset += chunk;
        }

        // Update directory
        if let Some(i) = slot {
            dir[i].size = bytes.len();
        } else {
            dir.push(FileEntry {
                name: String::from(path),
                start_block: content_block,
                size: bytes.len(),
            });
        }
        self.write_directory(&dir)
    }

    pub fn load_file(&mut self, path: &str) -> Option<String> {
        let dir = self.read_directory();
        let entry = dir.iter().find(|e| e.name == path)?;

        let mut content = Vec::new();
        let mut block = [0u8; BLOCK_SIZE];
        let needed = blocks_needed(entry.size);
        let mut remaining = entry.size;

        for b in 0..needed {
            if !self.read_block(entry.start_block + b, &mut block) { return None; }
            let chunk = remaining.min(BLOCK_SIZE);
            content.extend_from_slice(&block[..chunk]);
            remaining -= chunk;
        }

        core::str::from_utf8(&content).ok().map(String::from)
    }

    pub fn list_files(&mut self) -> Vec<String> {
        self.read_directory().into_iter().map(|e| e.name).collect()
    }

    fn read_directory(&mut self) -> Vec<FileEntry> {
        let mut block = [0u8; BLOCK_SIZE];
        if !self.read_block(DIR_BLOCK, &mut block) { return Vec::new(); }
        if block[0..4] != FS_MAGIC { return Vec::new(); }

        let count = block[4] as usize;
        let mut entries = Vec::new();

        for i in 0..count.min(MAX_FILES) {
            if !self.read_block(DIR_ENTRIES_START + i as u64, &mut block) { break; }
            let nlen = block[0] as usize;
            if nlen == 0 || nlen > 59 { continue; }
            let name = core::str::from_utf8(&block[1..1 + nlen]).unwrap_or("");
            let start = u16::from_le_bytes([block[60], block[61]]) as u64;
            let size = u16::from_le_bytes([block[62], block[63]]) as usize;
            entries.push(FileEntry {
                name: String::from(name),
                start_block: start,
                size,
            });
        }
        entries
    }

    fn write_directory(&mut self, entries: &[FileEntry]) -> bool {
        let mut block = [0u8; BLOCK_SIZE];
        block[0..4].copy_from_slice(&FS_MAGIC);
        block[4] = entries.len() as u8;
        if !self.write_block(DIR_BLOCK, &block) { return false; }

        for (i, entry) in entries.iter().enumerate() {
            block = [0u8; BLOCK_SIZE];
            let nb = entry.name.as_bytes();
            let nlen = nb.len().min(59);
            block[0] = nlen as u8;
            block[1..1 + nlen].copy_from_slice(&nb[..nlen]);
            let start_bytes = (entry.start_block as u16).to_le_bytes();
            block[60] = start_bytes[0];
            block[61] = start_bytes[1];
            let size_bytes = (entry.size as u16).to_le_bytes();
            block[62] = size_bytes[0];
            block[63] = size_bytes[1];
            if !self.write_block(DIR_ENTRIES_START + i as u64, &block) { return false; }
        }
        true
    }
}

struct FileEntry {
    name: String,
    start_block: u64,
    size: usize,
}

fn blocks_needed(size: usize) -> u64 {
    ((size + BLOCK_SIZE - 1) / BLOCK_SIZE).max(1) as u64
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
