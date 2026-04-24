// Native seL4 x86_64 syscall wrappers for Privion OS service processes.
//
// These implement seL4 object invocations and wait using Rust inline asm,
// bypassing libsel4's requirement for TLS-based IPC buffer pointer setup.
//
// Service processes (driverd, driver-nic, etc.) do NOT link libsel4.a or
// sel4runtime.a. They use these wrappers instead of the extern "C" bindings.
//
// Register convention (x86_64, `syscall` instruction):
//   IN:  rdx = syscall number, rdi = dest/src cap, rsi = msg_info_in
//        r10 = mr0, r8 = mr1, r9 = mr2, r15 = mr3
//   OUT: rsi = msg_info_out, rdi = badge, r10 = mr0_out
//   Clobbers: rcx (saved rip for sysretq), r11 (saved rflags).
//   rsp is preserved by the seL4 kernel (saved/restored from TCB context).
//   Note: rbx is NOT used — LLVM reserves it on x86_64 for internal use.
//
// Invocation labels computed from gen_headers/arch/api/invocation.h for:
//   !CONFIG_KERNEL_MCS, !CONFIG_IOMMU, !CONFIG_VTX, !CONFIG_HARDWARE_DEBUG_API

use super::seL4_CPtr;

// ---------------------------------------------------------------------------
// Syscall numbers
// ---------------------------------------------------------------------------

const SYS_CALL:  i64 = -1; // seL4_Call (object invocation)
const SYS_REPLY: i64 = -4; // seL4_Reply (send to implicit reply cap)
const SYS_RECV:  i64 = -5; // seL4_Recv (wait on endpoint or notification)

// ---------------------------------------------------------------------------
// Object invocation labels (x86_64, our kernel config)
// nInvocationLabels = 31, nSeL4ArchInvocationLabels = 33
// arch labels start at 33; IOPort labels at 42-48
// ---------------------------------------------------------------------------

const INV_X86_IOPORT_CONTROL_ISSUE: u64 = 42;
const INV_X86_IOPORT_IN8:           u64 = 43;
const INV_X86_IOPORT_IN16:          u64 = 44;
const INV_X86_IOPORT_IN32:          u64 = 45;
const INV_X86_IOPORT_OUT8:          u64 = 46;
const INV_X86_IOPORT_OUT16:         u64 = 47;
const INV_X86_IOPORT_OUT32:         u64 = 48;

// seL4 message info word: (label << 12) | (extra_caps << 7) | length
#[inline(always)]
const fn msg_info(label: u64, length: u64) -> u64 {
    (label << 12) | length
}

// ---------------------------------------------------------------------------
// Low-level seL4_Call: invoke cap, send (info, mr0-mr3), receive reply.
// Returns (info_out, mr0_out).
// ---------------------------------------------------------------------------

#[inline]
unsafe fn sel4_call(
    cap: u64, info_in: u64,
    mr0: u64, mr1: u64, mr2: u64, mr3: u64,
) -> (u64, u64) {
    let info_out: u64;
    let mr0_out:  u64;

    core::arch::asm!(
        "mov {save}, rsp",
        "syscall",
        "mov rsp, {save}",
        save = out(reg) _,
        in("rdx") SYS_CALL,
        inout("rdi") cap  => _,
        inout("rsi") info_in => info_out,
        inout("r10") mr0  => mr0_out,
        in("r8")  mr1,
        in("r9")  mr2,
        in("r15") mr3,
        out("rcx") _,
        lateout("r8")  _,
        lateout("r9")  _,
        lateout("r15") _,
        lateout("r11") _,
    );
    (info_out, mr0_out)
}

// ---------------------------------------------------------------------------
// Wait on a notification cap (blocks until IRQ fires or signal is sent).
// Returns the badge word.
// ---------------------------------------------------------------------------

#[inline]
pub unsafe fn sel4_wait_notification(notif_cap: seL4_CPtr) -> u64 {
    let badge: u64;
    core::arch::asm!(
        "syscall",
        in("rdx") SYS_RECV,
        inout("rdi") notif_cap as u64 => badge,
        out("rsi") _,
        out("r10") _,
        out("r8")  _,
        out("r9")  _,
        out("r15") _,
        out("rcx") _,
        lateout("r11") _,
        options(nostack),
    );
    badge
}

// ---------------------------------------------------------------------------
// Endpoint IPC — receive and reply (used by server-side service processes).
// ---------------------------------------------------------------------------

/// Block on an endpoint cap until a client sends a message.
/// Returns (msg_info, sender_badge, mr0, mr1, mr2, mr3).
/// For endpoint receives, sender_badge is the badge of the calling thread's cap.
#[inline]
pub unsafe fn sel4_recv(ep_cap: seL4_CPtr) -> (u64, u64, u64, u64, u64, u64) {
    let info:  u64;
    let badge: u64;
    let mr0:   u64;
    let mr1:   u64;
    let mr2:   u64;
    let mr3:   u64;
    core::arch::asm!(
        "syscall",
        in("rdx")    SYS_RECV,
        inout("rdi") ep_cap as u64 => badge,
        out("rsi")   info,
        out("r10")   mr0,
        out("r8")    mr1,
        out("r9")    mr2,
        out("r15")   mr3,
        out("rcx")   _,
        lateout("r11") _,
        options(nostack),
    );
    (info, badge, mr0, mr1, mr2, mr3)
}

/// Reply to the most recently received Call using the implicit reply capability.
/// info encodes (label << 12) | length. mr0/mr1 carry the reply data words.
#[inline]
pub unsafe fn sel4_reply(info: u64, mr0: u64, mr1: u64) {
    core::arch::asm!(
        "syscall",
        in("rdx")  SYS_REPLY,
        in("rdi")  0u64,   // null cap — kernel uses TCB's implicit reply cap
        in("rsi")  info,
        in("r10")  mr0,
        in("r8")   mr1,
        in("r9")   0u64,
        in("r15")  0u64,
        out("rcx") _,
        lateout("r8")  _,
        lateout("r9")  _,
        lateout("r10") _,
        lateout("r11") _,
        lateout("r15") _,
        options(nostack),
    );
}

// ---------------------------------------------------------------------------
// x86 I/O port read/write
// ---------------------------------------------------------------------------

/// Read 8 bits from port. Returns the value.
#[inline]
pub unsafe fn sel4_ioport_in8(cap: seL4_CPtr, port: u16) -> u8 {
    let (_, mr0) = sel4_call(cap as u64, msg_info(INV_X86_IOPORT_IN8, 1),
                             port as u64, 0, 0, 0);
    (mr0 & 0xff) as u8
}

/// Read 16 bits from port.
#[inline]
pub unsafe fn sel4_ioport_in16(cap: seL4_CPtr, port: u16) -> u16 {
    let (_, mr0) = sel4_call(cap as u64, msg_info(INV_X86_IOPORT_IN16, 1),
                             port as u64, 0, 0, 0);
    (mr0 & 0xffff) as u16
}

/// Read 32 bits from port.
#[inline]
pub unsafe fn sel4_ioport_in32(cap: seL4_CPtr, port: u16) -> u32 {
    let (_, mr0) = sel4_call(cap as u64, msg_info(INV_X86_IOPORT_IN32, 1),
                             port as u64, 0, 0, 0);
    mr0 as u32
}

/// Write 8 bits to port. mr0 = port, mr1 = value.
#[inline]
pub unsafe fn sel4_ioport_out8(cap: seL4_CPtr, port: u16, value: u8) {
    sel4_call(cap as u64, msg_info(INV_X86_IOPORT_OUT8, 2),
              port as u64, value as u64, 0, 0);
}

/// Write 16 bits to port.
#[inline]
pub unsafe fn sel4_ioport_out16(cap: seL4_CPtr, port: u16, value: u16) {
    sel4_call(cap as u64, msg_info(INV_X86_IOPORT_OUT16, 2),
              port as u64, value as u64, 0, 0);
}

/// Write 32 bits to port.
#[inline]
pub unsafe fn sel4_ioport_out32(cap: seL4_CPtr, port: u16, value: u32) {
    sel4_call(cap as u64, msg_info(INV_X86_IOPORT_OUT32, 2),
              port as u64, value as u64, 0, 0);
}
