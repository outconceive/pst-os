#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;

use libprivos::allocator;
use sel4_sys::seL4_BootInfo;

use libpst::constraint::Constraint;
use proctable::{ProcessTable, ProcessEntry, STATE_NEW, PRIV_SYSTEM, PRIV_DRIVER, PRIV_USER};
use pst_offset::{RootOffsetTable, SUB_PROCESS, PRIV_HARDWARE, PRIV_KERNEL};

// ---------------------------------------------------------------------------
// Serial output via seL4_DebugPutChar syscall (no capability needed)
// ---------------------------------------------------------------------------

#[inline(never)]
unsafe fn debug_putchar(c: u8) {
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

fn serial_print(s: &str) {
    for b in s.bytes() {
        if b == b'\n' {
            unsafe { debug_putchar(b'\r'); }
        }
        unsafe { debug_putchar(b); }
    }
}

fn serial_print_num(mut n: usize) {
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

    serial_print("[pst-framebuffer] Rendered. Dumping as PPM over serial...\n");

    // Output PPM header + pixel data
    // PPM format: P6\n<width> <height>\n255\n<RGB bytes>
    serial_print("PPM_START\n");
    serial_print("P6\n320 200\n255\n");
    // Write raw RGB bytes (strip alpha from BGRA)
    for y in 0..200 {
        for x in 0..320 {
            let off = y * fb.stride + x * 4;
            let r = fb.pixels[off + 2];
            let g = fb.pixels[off + 1];
            let b = fb.pixels[off];
            unsafe {
                debug_putchar(r);
                debug_putchar(g);
                debug_putchar(b);
            }
        }
    }
    serial_print("\nPPM_END\n");

    // --- Done ---
    serial_print("\n========================================\n");
    serial_print("  PST OS boot complete.\n");
    serial_print("  Markout rendered to pixels on bare metal.\n");
    serial_print("  No Wayland. No X11. No display server.\n");
    serial_print("  The thesis is proven.\n");
    serial_print("========================================\n");

    loop { core::hint::spin_loop(); }
}
