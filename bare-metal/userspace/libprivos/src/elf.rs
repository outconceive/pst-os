// ELF64 parser for loading service binaries into new VSpaces.
//
// Handles only the subset of ELF64 needed for Privion OS services:
//   - x86_64 little-endian statically linked executables
//   - PT_LOAD segments (the only ones that matter for loading)
// Does not handle dynamic linking, relocations, or shared libraries.

// ---------------------------------------------------------------------------
// ELF64 constants
// ---------------------------------------------------------------------------

const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1; // little-endian
const ET_EXEC: u16 = 2;    // executable file
const EM_X86_64: u16 = 62;
const PT_LOAD: u32 = 1;

// ELF PT_LOAD flags
pub const PF_X: u32 = 0x1; // execute
pub const PF_W: u32 = 0x2; // write
pub const PF_R: u32 = 0x4; // read

// ---------------------------------------------------------------------------
// ELF64 structures (repr(C) to match the binary layout exactly)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
#[repr(C)]
struct Elf64Ehdr {
    e_ident:     [u8; 16],
    e_type:      u16,
    e_machine:   u16,
    e_version:   u32,
    e_entry:     u64,
    e_phoff:     u64,  // offset of program header table
    e_shoff:     u64,
    e_flags:     u32,
    e_ehsize:    u16,
    e_phentsize: u16,  // size of one program header entry
    e_phnum:     u16,  // number of program header entries
    e_shentsize: u16,
    e_shnum:     u16,
    e_shstrndx:  u16,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Elf64Phdr {
    pub p_type:   u32,
    pub p_flags:  u32,
    pub p_offset: u64, // byte offset in the ELF file
    pub p_vaddr:  u64, // virtual address in process memory
    pub p_paddr:  u64, // physical address (ignored for our purposes)
    pub p_filesz: u64, // bytes in the file image (may be 0)
    pub p_memsz:  u64, // bytes in the memory image (p_memsz >= p_filesz)
    pub p_align:  u64, // alignment (must be a power of 2)
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum ElfError {
    /// Not a valid ELF file (bad magic, class, endianness, or arch).
    InvalidHeader,
    /// File is truncated — can't read a required section.
    Truncated,
    /// Segment offset or size would read past end of the file.
    SegmentOutOfBounds,
}

// ---------------------------------------------------------------------------
// ElfBinary — a parsed view of an ELF64 binary
// ---------------------------------------------------------------------------

/// A parsed ELF64 binary. Borrows the underlying byte slice.
pub struct ElfBinary<'a> {
    data:   &'a [u8],
    header: &'a Elf64Ehdr,
}

impl<'a> ElfBinary<'a> {
    /// Parse and validate an ELF64 binary from a byte slice.
    pub fn parse(data: &'a [u8]) -> Result<Self, ElfError> {
        let hdr_size = core::mem::size_of::<Elf64Ehdr>();
        if data.len() < hdr_size {
            return Err(ElfError::Truncated);
        }

        // SAFETY: We checked data.len() >= size_of::<Elf64Ehdr>() above.
        let header = unsafe { &*(data.as_ptr() as *const Elf64Ehdr) };

        if header.e_ident[0..4] != ELF_MAGIC    { return Err(ElfError::InvalidHeader); }
        if header.e_ident[4]    != ELFCLASS64    { return Err(ElfError::InvalidHeader); }
        if header.e_ident[5]    != ELFDATA2LSB   { return Err(ElfError::InvalidHeader); }
        if header.e_machine     != EM_X86_64     { return Err(ElfError::InvalidHeader); }

        Ok(Self { data, header })
    }

    /// Virtual address of the process entry point.
    pub fn entry_point(&self) -> u64 {
        self.header.e_entry
    }

    /// Iterate over PT_LOAD segments.
    pub fn load_segments(&self) -> impl Iterator<Item = LoadSegment<'a>> + '_ {
        let phoff    = self.header.e_phoff as usize;
        let phnum    = self.header.e_phnum as usize;
        let phentsz  = self.header.e_phentsize as usize;
        let data     = self.data;

        (0..phnum).filter_map(move |i| {
            let off = phoff + i * phentsz;
            if off + phentsz > data.len() {
                return None;
            }
            // SAFETY: bounds checked above; Elf64Phdr is repr(C) packed.
            let ph = unsafe { &*(data[off..].as_ptr() as *const Elf64Phdr) };
            if ph.p_type != PT_LOAD {
                return None;
            }
            let file_start = ph.p_offset as usize;
            let file_end   = file_start + ph.p_filesz as usize;
            if file_end > data.len() {
                return None;
            }
            Some(LoadSegment {
                vaddr:     ph.p_vaddr,
                memsz:     ph.p_memsz,
                flags:     ph.p_flags,
                file_data: &data[file_start..file_end],
            })
        })
    }
}

// ---------------------------------------------------------------------------
// LoadSegment — a single PT_LOAD segment
// ---------------------------------------------------------------------------

/// A PT_LOAD segment: the data to write into memory plus its destination.
pub struct LoadSegment<'a> {
    /// Virtual address in the new process's address space.
    pub vaddr:     u64,
    /// Total memory size (bytes after file_data must be zeroed).
    pub memsz:     u64,
    /// Flags: PF_R | PF_W | PF_X combinations.
    pub flags:     u32,
    /// Raw bytes from the file (length = p_filesz; may be 0).
    pub file_data: &'a [u8],
}

impl LoadSegment<'_> {
    /// Start of the first page that covers this segment.
    pub fn page_start(&self) -> u64 {
        self.vaddr & !0xfff
    }

    /// One past the last page that covers this segment.
    pub fn page_end(&self) -> u64 {
        (self.vaddr + self.memsz + 0xfff) & !0xfff
    }

    /// Number of 4 KiB pages needed.
    pub fn page_count(&self) -> usize {
        ((self.page_end() - self.page_start()) / 0x1000) as usize
    }

    pub fn is_executable(&self) -> bool { self.flags & PF_X != 0 }
    pub fn is_writable(&self)   -> bool { self.flags & PF_W != 0 }
}
