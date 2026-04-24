#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

pub mod native;

// ---------------------------------------------------------------------------
// Fundamental types
// ---------------------------------------------------------------------------

/// The machine word type — 64 bits on x86_64.
pub type seL4_Word = u64;

/// A capability pointer — an index into a CSpace.
pub type seL4_CPtr = seL4_Word;

/// A node ID (for multi-node seL4 configurations).
pub type seL4_NodeId = seL4_Word;

/// Error codes returned by seL4 API calls.
pub type seL4_Error = u32;

// seL4 error constants
pub const seL4_NoError: seL4_Error = 0;
pub const seL4_InvalidArgument: seL4_Error = 1;
pub const seL4_InvalidCapability: seL4_Error = 2;
pub const seL4_IllegalOperation: seL4_Error = 3;
pub const seL4_RangeError: seL4_Error = 4;
pub const seL4_AlignmentError: seL4_Error = 5;
pub const seL4_FailedLookup: seL4_Error = 6;
pub const seL4_TruncatedMessage: seL4_Error = 7;
pub const seL4_DeleteFirst: seL4_Error = 8;
pub const seL4_RevokeFirst: seL4_Error = 9;
pub const seL4_NotEnoughMemory: seL4_Error = 10;

// ---------------------------------------------------------------------------
// Well-known initial capability slots
// These are the capability slots seL4 gives to the initial (root) thread.
// ---------------------------------------------------------------------------

pub const seL4_CapNull: seL4_CPtr = 0;
pub const seL4_CapInitThreadTCB: seL4_CPtr = 1;
pub const seL4_CapInitThreadCNode: seL4_CPtr = 2;
pub const seL4_CapInitThreadVSpace: seL4_CPtr = 3;
pub const seL4_CapIRQControl: seL4_CPtr = 4;
pub const seL4_CapASIDControl: seL4_CPtr = 5;
pub const seL4_CapInitThreadASIDPool: seL4_CPtr = 6;
pub const seL4_CapIOPortControl: seL4_CPtr = 7;
pub const seL4_CapIOSpace: seL4_CPtr = 8;
pub const seL4_CapBootInfoFrame: seL4_CPtr = 9;
pub const seL4_CapInitThreadIPCBuffer: seL4_CPtr = 10;
pub const seL4_CapDomain: seL4_CPtr = 11;
pub const seL4_NumInitialCaps: seL4_CPtr = 12;

// ---------------------------------------------------------------------------
// Object types (architecture-independent)
// Used with seL4_Untyped_Retype to create kernel objects.
// ---------------------------------------------------------------------------

pub const seL4_UntypedObject: seL4_Word = 0;
pub const seL4_TCBObject: seL4_Word = 1;
pub const seL4_EndpointObject: seL4_Word = 2;
pub const seL4_NotificationObject: seL4_Word = 3;
pub const seL4_CapTableObject: seL4_Word = 4;

// ---------------------------------------------------------------------------
// Object types (x86_64-specific)
// Numbering continues from seL4_NonArchObjectTypeCount = 5.
// ---------------------------------------------------------------------------

pub const seL4_X86_4K: seL4_Word = 5;               // 4 KiB page frame
pub const seL4_X86_LargePage: seL4_Word = 6;         // 2 MiB large page
pub const seL4_X86_PageTableObject: seL4_Word = 7;   // page table (PT)
pub const seL4_X86_PageDirectoryObject: seL4_Word = 8; // page directory (PD)
pub const seL4_X86_PDPTObject: seL4_Word = 9;        // page directory pointer table
pub const seL4_X86_PML4Object: seL4_Word = 10;       // top-level paging structure (VSpace root)

// ---------------------------------------------------------------------------
// Object size bits (log2 of the object size in bytes)
// ---------------------------------------------------------------------------

pub const seL4_EndpointBits: u8 = 4;    // 16 bytes
pub const seL4_NotificationBits: u8 = 6; // 64 bytes
pub const seL4_TCBBits: u8 = 11;        // 2048 bytes
pub const seL4_PageBits: u8 = 12;       // 4096 bytes (4 KiB)

// ---------------------------------------------------------------------------
// IPC message limits
// ---------------------------------------------------------------------------

pub const seL4_MsgMaxLength: usize = 120;
pub const seL4_MsgMaxExtraCaps: usize = 3;
pub const seL4_NilData: seL4_Word = 0;

// ---------------------------------------------------------------------------
// seL4_MessageInfo_t
//
// Packed into a single word:
//   bits [63:12] = label     (52-bit message label)
//   bits [11:9]  = capsUnwrapped
//   bits [8:7]   = extraCaps
//   bits [6:0]   = length    (number of message registers used)
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct seL4_MessageInfo_t {
    pub words: [seL4_Word; 1],
}

impl seL4_MessageInfo_t {
    #[inline]
    pub fn new(
        label: seL4_Word,
        caps_unwrapped: seL4_Word,
        extra_caps: seL4_Word,
        length: seL4_Word,
    ) -> Self {
        let word = (label << 12)
            | ((caps_unwrapped & 0x7) << 9)
            | ((extra_caps & 0x3) << 7)
            | (length & 0x7f);
        seL4_MessageInfo_t { words: [word] }
    }

    #[inline]
    pub fn get_label(self) -> seL4_Word {
        self.words[0] >> 12
    }

    #[inline]
    pub fn get_length(self) -> seL4_Word {
        self.words[0] & 0x7f
    }

    #[inline]
    pub fn get_extra_caps(self) -> seL4_Word {
        (self.words[0] >> 7) & 0x3
    }
}

// ---------------------------------------------------------------------------
// seL4_SlotRegion — a contiguous range of capability slots [start, end)
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct seL4_SlotRegion {
    pub start: seL4_CPtr,
    pub end: seL4_CPtr,
}

// ---------------------------------------------------------------------------
// seL4_UntypedDesc — describes one untyped memory capability in BootInfo
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct seL4_UntypedDesc {
    /// Physical address of the untyped region.
    pub paddr: seL4_Word,
    /// log2 of the region size in bytes.
    pub sizeBits: u8,
    /// Non-zero if this is device memory (MMIO), not regular RAM.
    pub isDevice: u8,
    /// Padding to align to seL4_Word size.
    pub padding: [u8; 6],
}

// ---------------------------------------------------------------------------
// seL4_IPCBuffer — IPC buffer, mapped into each thread's address space
// ---------------------------------------------------------------------------

#[derive(Copy, Clone)]
#[repr(C)]
pub struct seL4_IPCBuffer {
    pub tag: seL4_MessageInfo_t,
    pub msg: [seL4_Word; seL4_MsgMaxLength],
    pub userData: seL4_Word,
    pub caps_or_badges: [seL4_Word; seL4_MsgMaxExtraCaps],
    pub receiveCNode: seL4_CPtr,
    pub receiveIndex: seL4_CPtr,
    pub receiveDepth: seL4_Word,
}

// ---------------------------------------------------------------------------
// seL4_BootInfo — the structure seL4 passes to the initial thread
// ---------------------------------------------------------------------------

/// Maximum number of untyped capabilities in BootInfo.
/// Determined at kernel build time; 230 is the seL4 default.
pub const CONFIG_MAX_NUM_BOOTINFO_UNTYPED_CAPS: usize = 230;

#[repr(C)]
pub struct seL4_BootInfo {
    pub extraLen: u32,
    pub nodeID: seL4_NodeId,
    pub numNodes: seL4_Word,
    pub numIOPTLevels: seL4_Word,
    pub ipcBuffer: *mut seL4_IPCBuffer,
    pub empty: seL4_SlotRegion,
    pub sharedFrames: seL4_SlotRegion,
    pub userImageFrames: seL4_SlotRegion,
    pub userImagePaging: seL4_SlotRegion,
    pub ioSpaceCaps: seL4_SlotRegion,
    pub extraBIPages: seL4_SlotRegion,
    pub initThreadCNodeSizeBits: seL4_Word,
    pub initThreadDomain: seL4_Word,
    pub untyped: seL4_SlotRegion,
    /// Variable-length array at the end; capacity is CONFIG_MAX_NUM_BOOTINFO_UNTYPED_CAPS.
    pub untypedList: [seL4_UntypedDesc; CONFIG_MAX_NUM_BOOTINFO_UNTYPED_CAPS],
}

// ---------------------------------------------------------------------------
// seL4 API — extern "C" declarations
//
// These functions are implemented in libsel4 (built as part of the seL4
// userspace library). They will be linked in Phase 4 when we build the
// final init binary against libsel4.a.
// ---------------------------------------------------------------------------

extern "C" {
    // --- IPC ---

    /// Send a message to an endpoint (non-blocking if receiver not waiting).
    pub fn seL4_Send(dest: seL4_CPtr, msgInfo: seL4_MessageInfo_t);

    /// Receive a message from an endpoint (blocks until a sender arrives).
    pub fn seL4_Recv(src: seL4_CPtr, sender: *mut seL4_Word) -> seL4_MessageInfo_t;

    /// Reply to the sender of the most recently received message.
    pub fn seL4_Reply(msgInfo: seL4_MessageInfo_t);

    /// Send then immediately wait for a reply (combines Send + Recv).
    pub fn seL4_Call(dest: seL4_CPtr, msgInfo: seL4_MessageInfo_t) -> seL4_MessageInfo_t;

    // --- Message Registers ---

    /// Read message register i from the IPC buffer.
    pub fn seL4_GetMR(i: i32) -> seL4_Word;

    /// Write message register i in the IPC buffer.
    pub fn seL4_SetMR(i: i32, word: seL4_Word);

    // --- Untyped memory ---

    /// Retype an untyped capability into one or more kernel objects.
    ///
    /// # Parameters
    /// - `service`: untyped capability to retype from
    /// - `r#type`: object type (use the seL4_*Object constants)
    /// - `size_bits`: for variable-size objects (CNodes, Untypeds), log2 of size
    /// - `root`, `node_index`, `node_depth`: CSpace path to place new caps
    /// - `node_offset`: first slot in the destination CNode
    /// - `num_objects`: number of objects to create
    pub fn seL4_Untyped_Retype(
        service: seL4_CPtr,
        r#type: seL4_Word,
        size_bits: seL4_Word,
        root: seL4_CPtr,
        node_index: seL4_Word,
        node_depth: seL4_Word,
        node_offset: seL4_Word,
        num_objects: seL4_Word,
    ) -> seL4_Error;

    // --- TCB (Thread Control Block) ---

    pub fn seL4_TCB_Configure(
        service: seL4_CPtr,
        faultEP: seL4_CPtr,
        cspaceRoot: seL4_CPtr,
        cspaceRootData: seL4_Word,
        vspaceRoot: seL4_CPtr,
        vspaceRootData: seL4_Word,
        buffer: seL4_Word,
        bufferFrame: seL4_CPtr,
    ) -> seL4_Error;

    pub fn seL4_TCB_SetPriority(
        service: seL4_CPtr,
        authority: seL4_CPtr,
        priority: seL4_Word,
    ) -> seL4_Error;

    pub fn seL4_TCB_WriteRegisters(
        service: seL4_CPtr,
        resume_target: i32,
        arch_flags: u8,
        count: seL4_Word,
        regs: *const seL4_UserContext,
    ) -> seL4_Error;

    pub fn seL4_TCB_Resume(service: seL4_CPtr) -> seL4_Error;

    // --- CNode ---

    pub fn seL4_CNode_Copy(
        dest_root: seL4_CPtr,
        dest_index: seL4_Word,
        dest_depth: u8,
        src_root: seL4_CPtr,
        src_index: seL4_Word,
        src_depth: u8,
        rights: seL4_CapRights_t,
    ) -> seL4_Error;

    // --- x86_64 Virtual Memory ---
    // Maps page table structures and page frames into a VSpace.
    // All map calls take the PML4 (VSpace root) cap; seL4 traverses
    // the page table hierarchy automatically.

    /// Map a PDPT (level 3 page table) into a PML4.
    pub fn seL4_X86_PDPT_Map(
        service: seL4_CPtr,   // PDPT cap
        pml4: seL4_CPtr,      // VSpace root
        vaddr: seL4_Word,     // virtual address (must be PDPT-aligned: 512 GiB)
        attr: seL4_X86_VMAttributes,
    ) -> seL4_Error;

    /// Map a PageDirectory (level 2) into a VSpace.
    pub fn seL4_X86_PageDirectory_Map(
        service: seL4_CPtr,
        pml4: seL4_CPtr,
        vaddr: seL4_Word,
        attr: seL4_X86_VMAttributes,
    ) -> seL4_Error;

    /// Map a PageTable (level 1) into a VSpace.
    pub fn seL4_X86_PageTable_Map(
        service: seL4_CPtr,
        pml4: seL4_CPtr,
        vaddr: seL4_Word,
        attr: seL4_X86_VMAttributes,
    ) -> seL4_Error;

    /// Map a 4 KiB page frame into a VSpace.
    pub fn seL4_X86_Page_Map(
        service: seL4_CPtr,      // frame cap
        pml4: seL4_CPtr,         // VSpace root
        vaddr: seL4_Word,        // virtual address (4 KiB aligned)
        rights: seL4_CapRights_t,
        attr: seL4_X86_VMAttributes,
    ) -> seL4_Error;

    /// Unmap a 4 KiB page frame from whatever VSpace it is currently mapped in.
    pub fn seL4_X86_Page_Unmap(service: seL4_CPtr) -> seL4_Error;

    /// Copy bytes from one process's mapped frame to another (via direct ptr).
    /// In practice we use this after mapping a frame in our own VSpace to init it.
    pub fn seL4_X86_Page_GetAddress(service: seL4_CPtr) -> seL4_X86_Page_GetAddress_t;

    /// Set the IPC buffer for a thread (needed for Phase 4 child processes).
    pub fn seL4_TCB_SetIPCBuffer(
        service: seL4_CPtr,
        buffer: seL4_Word,     // virtual address of IPC buffer in thread's VSpace
        bufferFrame: seL4_CPtr,
    ) -> seL4_Error;
}

// ---------------------------------------------------------------------------
// Supporting types for the extern functions above
// ---------------------------------------------------------------------------

/// x86_64 user-space register context (passed to TCB_WriteRegisters).
#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct seL4_UserContext {
    pub rip: seL4_Word,
    pub rsp: seL4_Word,
    pub rflags: seL4_Word,
    pub rax: seL4_Word,
    pub rbx: seL4_Word,
    pub rcx: seL4_Word,
    pub rdx: seL4_Word,
    pub rsi: seL4_Word,
    pub rdi: seL4_Word,
    pub rbp: seL4_Word,
    pub r8: seL4_Word,
    pub r9: seL4_Word,
    pub r10: seL4_Word,
    pub r11: seL4_Word,
    pub r12: seL4_Word,
    pub r13: seL4_Word,
    pub r14: seL4_Word,
    pub r15: seL4_Word,
    pub tls_base: seL4_Word,
    pub fs_base: seL4_Word,
    pub gs_base: seL4_Word,
}

/// Capability rights word.
#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct seL4_CapRights_t {
    pub words: [seL4_Word; 1],
}

impl seL4_CapRights_t {
    pub const ALL: Self = seL4_CapRights_t { words: [0xf] };       // RW + Grant + GrantReply
    pub const READ_WRITE: Self = seL4_CapRights_t { words: [0x3] }; // Read + Write
    pub const READ_ONLY: Self = seL4_CapRights_t { words: [0x1] };  // Read only
}

// ---------------------------------------------------------------------------
// x86_64 VM attributes — cache behaviour for mapped pages
// ---------------------------------------------------------------------------

/// Passed to seL4_X86_Page_Map and page table mapping calls.
pub type seL4_X86_VMAttributes = seL4_Word;

/// Write-back caching (default for code/data).
pub const seL4_X86_Default_VMAttributes: seL4_X86_VMAttributes = 0;
/// Write-combining (suitable for framebuffers).
pub const seL4_X86_WriteCombining: seL4_X86_VMAttributes = 1;
/// Strong uncacheable (for strict MMIO ordering).
pub const seL4_X86_StrongUncacheable: seL4_X86_VMAttributes = 2;
/// Uncached — no caching at all (standard MMIO).
pub const seL4_X86_Uncacheable: seL4_X86_VMAttributes = 3;

/// Return type of seL4_X86_Page_GetAddress.
#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct seL4_X86_Page_GetAddress_t {
    pub paddr: seL4_Word,
    pub error: seL4_Error,
}

// ---------------------------------------------------------------------------
// Return types for I/O port reads
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct seL4_X86_IOPort_In8_t  { pub result: u8,  pub error: seL4_Error }

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct seL4_X86_IOPort_In16_t { pub result: u16, pub error: seL4_Error }

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct seL4_X86_IOPort_In32_t { pub result: u32, pub error: seL4_Error }

// ---------------------------------------------------------------------------
// x86 I/O port API
// seL4_X86_IOPortControl_Issue: carves out a port range from the control cap.
// seL4_X86_IOPort_In*/Out*: read/write a single port through the cap.
// ---------------------------------------------------------------------------

extern "C" {
    pub fn seL4_X86_IOPortControl_Issue(
        service: seL4_CPtr,     // seL4_CapIOPortControl
        first_port: u16,
        last_port: u16,
        dest_root: seL4_CPtr,
        dest_index: seL4_Word,
        dest_depth: u8,
    ) -> seL4_Error;

    pub fn seL4_X86_IOPort_In8 (service: seL4_CPtr, port: u16) -> seL4_X86_IOPort_In8_t;
    pub fn seL4_X86_IOPort_In16(service: seL4_CPtr, port: u16) -> seL4_X86_IOPort_In16_t;
    pub fn seL4_X86_IOPort_In32(service: seL4_CPtr, port: u16) -> seL4_X86_IOPort_In32_t;

    pub fn seL4_X86_IOPort_Out8 (service: seL4_CPtr, port: u16, value: u8)  -> seL4_Error;
    pub fn seL4_X86_IOPort_Out16(service: seL4_CPtr, port: u16, value: u16) -> seL4_Error;
    pub fn seL4_X86_IOPort_Out32(service: seL4_CPtr, port: u16, value: u32) -> seL4_Error;
}

// ---------------------------------------------------------------------------
// IRQ API
// ---------------------------------------------------------------------------

extern "C" {
    /// Allocate an IRQ handler capability for the given IRQ number.
    pub fn seL4_IRQControl_Get(
        service: seL4_CPtr,     // seL4_CapIRQControl
        irq: seL4_Word,
        dest_root: seL4_CPtr,
        dest_index: seL4_Word,
        dest_depth: u8,
    ) -> seL4_Error;

    /// Bind a notification object to this IRQ handler.
    /// When the IRQ fires, the kernel signals the notification.
    pub fn seL4_IRQHandler_SetNotification(
        service: seL4_CPtr,
        notification: seL4_CPtr,
    ) -> seL4_Error;

    /// Re-enable the IRQ at the hardware level after handling it.
    pub fn seL4_IRQHandler_Ack(service: seL4_CPtr) -> seL4_Error;

    /// Unbind the notification and mask the IRQ.
    pub fn seL4_IRQHandler_Clear(service: seL4_CPtr) -> seL4_Error;
}

// ---------------------------------------------------------------------------
// Notification API
// ---------------------------------------------------------------------------

extern "C" {
    /// Signal a notification (non-blocking, sets badge bit).
    pub fn seL4_Signal(dest: seL4_CPtr);

    /// Wait on a notification or endpoint (blocks).
    /// Returns the badge word; for endpoints also returns message info.
    pub fn seL4_Wait(src: seL4_CPtr, sender: *mut seL4_Word) -> seL4_MessageInfo_t;

    /// Non-blocking poll of a notification.
    pub fn seL4_Poll(src: seL4_CPtr, sender: *mut seL4_Word) -> seL4_MessageInfo_t;
}

// ---------------------------------------------------------------------------
// auxv constants for service-process stack setup
// Values from sel4runtime/include/sel4runtime/auxv.h
// ---------------------------------------------------------------------------

pub const AT_NULL:                 u64 = 0;
pub const AT_SEL4_BOOT_INFO:       u64 = 64;
pub const AT_SEL4_IPC_BUFFER_PTR:  u64 = 67;
pub const AT_SEL4_TCB:             u64 = 69;
