#![no_std]
#![no_main]
#![feature(naked_functions)]

extern crate alloc;

mod sel4_shims;
mod vga;
mod keyboard;
mod shell;
mod desktop;
mod storage;
mod codeview;
mod editor;
mod browser;
mod convergence;
mod vgacon;
mod mouse;
mod input;
mod ps2;
mod gui_input;
mod net;
mod rng;

// Custom entry point: save bootinfo (rdi from kernel) before sel4runtime runs
#[no_mangle]
static mut KERNEL_BOOTINFO: u64 = 0;

core::arch::global_asm!(
    ".global _pst_entry",
    "_pst_entry:",
    "mov [rip + KERNEL_BOOTINFO], rdi",
    "jmp _sel4_start",
);

use alloc::string::String;

use libprivos::allocator;
use libprivos::mem::UntypedAllocator;
use libprivos::vm::VSpaceMapper;
use sel4_sys::*;

use libpst::constraint::Constraint;
use proctable::{ProcessTable, ProcessEntry, STATE_NEW, PRIV_SYSTEM, PRIV_DRIVER, PRIV_USER};
use pst_offset::{RootOffsetTable, SUB_PROCESS, PRIV_HARDWARE, PRIV_KERNEL};

// ---------------------------------------------------------------------------
// Serial output via seL4_DebugPutChar syscall (no capability needed)
// ---------------------------------------------------------------------------

#[inline(never)]
pub unsafe fn debug_putchar(c: u8) {
    // seL4_DebugPutChar: syscall number -9, char in rdi
    // seL4 x86_64 convention: save rsp to stack before syscall
    // (seL4 kernel preserves rsp, but we follow the C wrapper pattern)
    let saved_rsp: u64;
    core::arch::asm!(
        "mov {save}, rsp",
        "syscall",
        "mov rsp, {save}",
        save = out(reg) saved_rsp,
        in("rdx") -9i64 as u64,
        in("rdi") c as u64,
        in("rsi") 0u64,
        in("r10") 0u64,
        in("r8") 0u64,
        in("r9") 0u64,
        in("r15") 0u64,
        out("rcx") _,
        lateout("r11") _,
    );
    let _ = saved_rsp;
}

pub fn serial_print(s: &str) {
    for b in s.bytes() {
        if b == b'\n' {
            unsafe { debug_putchar(b'\r'); }
        }
        unsafe { debug_putchar(b); }
        vgacon::putchar(b);
    }
}

pub fn serial_print_num(mut n: usize) {
    if n == 0 {
        unsafe { debug_putchar(b'0'); }
        return;
    }
    let mut buf = [0u8; 20];
    let mut i = 0;
    while n > 0 {
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        unsafe { debug_putchar(buf[i]); }
    }
}

// ---------------------------------------------------------------------------
// Boot Markout document
// ---------------------------------------------------------------------------

const BOOT_MARKOUT: &str = "\
@card
| Parallel String Theory OS
| ========================
|
@parametric
| {label:title \"PST OS v0.1\"}
| {label:arch \"x86_64 / seL4\" center-x:title gap-y:8}
| {label:status \"Boot complete\" center-x:title gap-y:8:arch}
@end parametric
|
| One primitive. One loop. One OS.
@end card";

// ---------------------------------------------------------------------------
// Entry point — called by sel4runtime after TLS + stack setup
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn main(_bootinfo: *const seL4_BootInfo) -> ! {
    // First sign of life — print before anything else
    unsafe { debug_putchar(b'P'); }
    unsafe { debug_putchar(b'S'); }
    unsafe { debug_putchar(b'T'); }
    unsafe { debug_putchar(b'\n'); }

    // Initialize heap
    unsafe { allocator::init() };

    serial_print("\n");
    serial_print("========================================\n");
    serial_print("  Parallel String Theory OS\n");
    serial_print("  Booting on seL4 microkernel...\n");
    serial_print("========================================\n\n");

    // --- Phase 1: Immortal root ---
    serial_print("[pst-offset] Creating immortal root...\n");

    let mut offset_root = RootOffsetTable::new();
    offset_root.register(SUB_PROCESS, PRIV_HARDWARE, 0x0000);
    offset_root.register(SUB_PROCESS, PRIV_KERNEL, 0x0001);

    serial_print("[pst-offset] Position 0: bootloader (HARDWARE)\n");
    serial_print("[pst-offset] Position 1: solver (KERNEL)\n");
    serial_print("[pst-offset] Immortal root: 2 positions\n\n");

    // --- Phase 2: Process table + constraint solver ---
    serial_print("[proctable] Registering services...\n");

    let mut pt = ProcessTable::new();

    pt.register(ProcessEntry {
        name: String::from("cryptod"), state: STATE_NEW,
        privilege: PRIV_SYSTEM, priority: 200, affinity: 0,
        constraints: alloc::vec![],
    });
    pt.register(ProcessEntry {
        name: String::from("vfs"), state: STATE_NEW,
        privilege: PRIV_SYSTEM, priority: 180, affinity: 0,
        constraints: alloc::vec![Constraint::After(String::from("cryptod"))],
    });
    pt.register(ProcessEntry {
        name: String::from("netd"), state: STATE_NEW,
        privilege: PRIV_SYSTEM, priority: 180, affinity: 0,
        constraints: alloc::vec![Constraint::After(String::from("cryptod"))],
    });
    pt.register(ProcessEntry {
        name: String::from("driverd"), state: STATE_NEW,
        privilege: PRIV_DRIVER, priority: 190, affinity: 0,
        constraints: alloc::vec![],
    });
    pt.register(ProcessEntry {
        name: String::from("driver-nic"), state: STATE_NEW,
        privilege: PRIV_DRIVER, priority: 170, affinity: 0,
        constraints: alloc::vec![Constraint::After(String::from("driverd"))],
    });
    pt.register(ProcessEntry {
        name: String::from("compositor"), state: STATE_NEW,
        privilege: PRIV_USER, priority: 100, affinity: 0,
        constraints: alloc::vec![
            Constraint::After(String::from("vfs")),
            Constraint::After(String::from("netd")),
        ],
    });

    serial_print("[proctable] 6 services registered\n");
    serial_print("[pst-sched] Solving boot order...\n");

    let result = pt.solve_spawn_order();

    serial_print("[pst-sched] Boot order: ");
    for (i, name) in result.order.iter().enumerate() {
        if i > 0 { serial_print(" -> "); }
        serial_print(name);
    }
    serial_print("\n");

    if result.cycles.is_empty() {
        serial_print("[pst-sched] No cycles detected\n\n");
    }

    // --- Phase 3: Markout rendering ---
    serial_print("[pst-markout] Parsing Markout document...\n");

    let lines = pst_markout::parse::parse(BOOT_MARKOUT);
    serial_print("[pst-markout] Parsed ");
    serial_print_num(lines.len());
    serial_print(" lines\n");

    serial_print("[pst-markout] Rendering to VDOM...\n");
    let vdom = pst_markout::render::render(&lines);

    serial_print("[pst-markout] Serializing to HTML...\n");
    let html = pst_markout::html::to_html(&vdom);

    serial_print("[pst-markout] HTML output (");
    serial_print_num(html.len());
    serial_print(" bytes)\n\n");

    // --- Phase 4: Framebuffer rendering ---
    serial_print("[pst-framebuffer] Rendering to 320x200 framebuffer...\n");

    use pst_framebuffer::{Framebuffer, Color, render_markout};

    let mut fb = Framebuffer::new(320, 200);
    fb.clear(Color::DARK_BG);
    render_markout(&mut fb, BOOT_MARKOUT, Color::DARK_BG, Color::WHITE);

    serial_print("[pst-framebuffer] Rendered to memory.\n");

    // --- Phase 5: Map VGA text buffer to QEMU display ---
    let kernel_bi = unsafe { KERNEL_BOOTINFO };
    serial_print("[vga] kernel bootinfo (saved from rdi): 0x");
    serial_print_hex(kernel_bi);
    serial_print("\n");
    serial_print("[vga] sel4runtime bootinfo (_bootinfo): 0x");
    serial_print_hex(_bootinfo as u64);
    serial_print("\n");

    let bi_ptr = if kernel_bi > 0x1000 {
        kernel_bi as *const seL4_BootInfo
    } else {
        _bootinfo
    };

    if bi_ptr.is_null() || (bi_ptr as u64) < 0x1000 {
        serial_print("[vga] ERROR: no valid bootinfo pointer\n");
    } else {
        let bi = unsafe { &*bi_ptr };
        serial_print("[vga] Using bootinfo at 0x");
        serial_print_hex(bi_ptr as u64);
        serial_print("\n");

        // Get IPC buffer from bootinfo
        let ipc_buf = bi.ipcBuffer;
        serial_print("[vga] IPC buffer: 0x");
        serial_print_hex(ipc_buf as u64);
        serial_print("\n");

        if !ipc_buf.is_null() && (ipc_buf as u64) > 0x1000 {
            // Set the IPC buffer for our syscall shims
            unsafe { sel4_shims::set_ipc_buffer(ipc_buf); }
            serial_print("[vga] IPC buffer set. seL4 invocations enabled.\n");

            if let Some(vga_state) = vga::init(bi_ptr) {
                vgacon::init(vga_state.fb_vaddr);

                // Try to set up block storage
                let (store, next_slot) = storage::setup(
                    bi_ptr, vga_state.pci_cap, vga_state.next_slot,
                );

                // Try to set up network
                let (net_dev, next_slot) = net::setup_with_port(vga_state.pci_cap, next_slot);
                if net_dev.is_some() {
                    serial_print("[net] Network available\n");
                }

                serial_print("\n========================================\n");
                serial_print("  PST OS boot complete.\n");
                serial_print("  The thesis is proven.\n");
                serial_print("========================================\n\n");

                if let Some(mut ps2_dev) = ps2::setup(bi_ptr, next_slot, vga_state.fb_vaddr) {
                    desktop::run(&mut ps2_dev, store, net_dev, vga_state.fb_vaddr);
                }
            }
        } else {
            serial_print("[vga] ERROR: IPC buffer invalid\n");
        }
    }

    loop { core::hint::spin_loop(); }
}

pub fn serial_print_hex(mut n: u64) {
    if n == 0 {
        serial_print("0");
        return;
    }
    let mut buf = [0u8; 16];
    let mut i = 0;
    while n > 0 {
        let d = (n & 0xF) as u8;
        buf[i] = if d < 10 { b'0' + d } else { b'a' + d - 10 };
        n >>= 4;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        unsafe { debug_putchar(buf[i]); }
    }
}
