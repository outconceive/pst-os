use sel4_sys::*;

static mut IPC_BUFFER: *mut seL4_IPCBuffer = core::ptr::null_mut();

pub unsafe fn set_ipc_buffer(buf: *mut seL4_IPCBuffer) {
    IPC_BUFFER = buf;
}

// seL4 syscall numbers
const SYS_CALL: i64 = -1;

// Invocation labels (from gen_headers)
const INV_UNTYPED_RETYPE: u64     = 1;
const INV_X86_PDPT_MAP: u64       = 31;
const INV_X86_PAGE_DIR_MAP: u64   = 33;
const INV_X86_PAGE_TABLE_MAP: u64 = 35;
const INV_X86_PAGE_MAP: u64       = 37;
const INV_IRQ_ACK: u64                  = 27;
const INV_IRQ_SET_HANDLER: u64          = 28;
const INV_X86_PAGE_GET_ADDRESS: u64 = 39;
const INV_X86_IOPORT_CONTROL_ISSUE: u64 = 42;
const INV_X86_IRQ_IOAPIC: u64          = 49;

// msg_info: (label << 12) | (extraCaps << 7) | (length & 0x7f)
#[inline(always)]
fn msg_info(label: u64, extra_caps: u64, length: u64) -> u64 {
    (label << 12) | ((extra_caps & 0x3) << 7) | (length & 0x7f)
}

#[inline(never)]
unsafe fn sel4_call(
    cap: u64, info: u64,
    mr0: u64, mr1: u64, mr2: u64, mr3: u64,
) -> (u64, u64) {
    let info_out: u64;
    let mr0_out: u64;
    core::arch::asm!(
        "mov {save}, rsp",
        "syscall",
        "mov rsp, {save}",
        save = out(reg) _,
        in("rdx") SYS_CALL as u64,
        inout("rdi") cap => _,
        inout("rsi") info => info_out,
        inout("r10") mr0 => mr0_out,
        in("r8") mr1,
        in("r9") mr2,
        in("r15") mr3,
        out("rcx") _,
        lateout("r11") _,
    );
    (info_out, mr0_out)
}

unsafe fn set_mr(index: usize, value: seL4_Word) {
    if !IPC_BUFFER.is_null() {
        (*IPC_BUFFER).msg[index] = value;
    }
}

unsafe fn set_cap(index: usize, cap: seL4_CPtr) {
    if !IPC_BUFFER.is_null() {
        (*IPC_BUFFER).caps_or_badges[index] = cap;
    }
}

#[no_mangle]
pub unsafe extern "C" fn seL4_Untyped_Retype(
    service: seL4_CPtr,
    r#type: seL4_Word,
    size_bits: seL4_Word,
    root: seL4_CPtr,
    node_index: seL4_Word,
    node_depth: seL4_Word,
    node_offset: seL4_Word,
    num_objects: seL4_Word,
) -> seL4_Error {
    // Untyped_Retype: label=1, extraCaps=1, length=6
    // MR0=type, MR1=size_bits, MR2=node_index, MR3=node_depth
    // MR4=node_offset (IPC buffer), MR5=num_objects (IPC buffer)
    // Extra cap 0 = root (IPC buffer caps_or_badges[0])

    set_mr(4, node_offset);
    set_mr(5, num_objects);
    set_cap(0, root);

    let (info_out, _) = sel4_call(
        service,
        msg_info(INV_UNTYPED_RETYPE, 1, 6),
        r#type, size_bits, node_index, node_depth,
    );

    (info_out >> 12) as seL4_Error
}

#[no_mangle]
pub unsafe extern "C" fn seL4_X86_Page_Map(
    page: seL4_CPtr,
    vspace: seL4_CPtr,
    vaddr: u64,
    rights: seL4_CapRights_t,
    attr: seL4_X86_VMAttributes,
) -> seL4_Error {
    // Page_Map: label=33, extraCaps=1, length=3
    // MR0=vaddr, MR1=rights, MR2=attr
    // Extra cap 0 = vspace

    set_cap(0, vspace);

    let (info_out, _) = sel4_call(
        page,
        msg_info(INV_X86_PAGE_MAP, 1, 3),
        vaddr, rights.words[0], attr as u64, 0,
    );

    (info_out >> 12) as seL4_Error
}

#[no_mangle]
pub unsafe extern "C" fn seL4_X86_PageTable_Map(
    pt: seL4_CPtr,
    vspace: seL4_CPtr,
    vaddr: u64,
    attr: seL4_X86_VMAttributes,
) -> seL4_Error {
    set_cap(0, vspace);

    let (info_out, _) = sel4_call(
        pt,
        msg_info(INV_X86_PAGE_TABLE_MAP, 1, 2),
        vaddr, attr as u64, 0, 0,
    );

    (info_out >> 12) as seL4_Error
}

#[no_mangle]
pub unsafe extern "C" fn seL4_X86_PageDirectory_Map(
    pd: seL4_CPtr,
    vspace: seL4_CPtr,
    vaddr: u64,
    attr: seL4_X86_VMAttributes,
) -> seL4_Error {
    set_cap(0, vspace);

    let (info_out, _) = sel4_call(
        pd,
        msg_info(INV_X86_PAGE_DIR_MAP, 1, 2),
        vaddr, attr as u64, 0, 0,
    );

    (info_out >> 12) as seL4_Error
}

#[no_mangle]
pub unsafe extern "C" fn seL4_X86_PDPT_Map(
    pdpt: seL4_CPtr,
    vspace: seL4_CPtr,
    vaddr: u64,
    attr: seL4_X86_VMAttributes,
) -> seL4_Error {
    set_cap(0, vspace);

    let (info_out, _) = sel4_call(
        pdpt,
        msg_info(INV_X86_PDPT_MAP, 1, 2),
        vaddr, attr as u64, 0, 0,
    );

    (info_out >> 12) as seL4_Error
}

#[no_mangle]
pub unsafe extern "C" fn seL4_X86_IOPortControl_Issue(
    service: seL4_CPtr,
    first_port: u16,
    last_port: u16,
    dest_root: seL4_CPtr,
    dest_index: seL4_Word,
    dest_depth: u8,
) -> seL4_Error {
    // IOPortControl_Issue: label=42, extraCaps=1, length=3
    // MR0=first_port, MR1=last_port, MR2=dest_index
    // MR3=dest_depth
    // Extra cap 0 = dest_root

    set_cap(0, dest_root);

    let (info_out, _) = sel4_call(
        service,
        msg_info(INV_X86_IOPORT_CONTROL_ISSUE, 1, 4),
        first_port as u64, last_port as u64, dest_index, dest_depth as u64,
    );

    (info_out >> 12) as seL4_Error
}

pub unsafe fn page_get_address(frame: seL4_CPtr) -> u64 {
    let (_, paddr) = sel4_call(
        frame,
        msg_info(INV_X86_PAGE_GET_ADDRESS, 0, 0),
        0, 0, 0, 0,
    );
    paddr
}

// ---------------------------------------------------------------------------
// IRQ shims (IOAPIC — CONFIG_IRQ_IOAPIC=1, generic IRQControl_Get won't work)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn seL4_IRQControl_GetIOAPIC(
    service: seL4_CPtr,
    root: seL4_CPtr,
    index: seL4_Word,
    depth: u8,
    ioapic: seL4_Word,
    pin: seL4_Word,
    level: seL4_Word,
    polarity: seL4_Word,
    vector: seL4_Word,
) -> seL4_Error {
    set_cap(0, root);
    set_mr(4, level);
    set_mr(5, polarity);
    set_mr(6, vector);

    let (info_out, _) = sel4_call(
        service,
        msg_info(INV_X86_IRQ_IOAPIC, 1, 7),
        index, depth as u64, ioapic, pin,
    );

    (info_out >> 12) as seL4_Error
}

#[no_mangle]
pub unsafe extern "C" fn seL4_IRQHandler_SetNotification(
    service: seL4_CPtr,
    notification: seL4_CPtr,
) -> seL4_Error {
    set_cap(0, notification);

    let (info_out, _) = sel4_call(
        service,
        msg_info(INV_IRQ_SET_HANDLER, 1, 0),
        0, 0, 0, 0,
    );

    (info_out >> 12) as seL4_Error
}

#[no_mangle]
pub unsafe extern "C" fn seL4_IRQHandler_Ack(service: seL4_CPtr) -> seL4_Error {
    let (info_out, _) = sel4_call(
        service,
        msg_info(INV_IRQ_ACK, 0, 0),
        0, 0, 0, 0,
    );

    (info_out >> 12) as seL4_Error
}
