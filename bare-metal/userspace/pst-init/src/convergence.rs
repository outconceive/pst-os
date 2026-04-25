use alloc::string::String;
use crate::serial_print;

const CONVERGENCE_DOC: &str = "\
@card
| Outconceive × PST OS
| =====================
|
@parametric
| {label:title \"Universal UI Primitive\"}
| {label:sub \"One document. Every surface.\" center-x:title gap-y:8}
@end parametric
|
| {input:name}  Username
| {password:pass}  Password
| {checkbox:remember}  Remember me
|
| {button:login \"Sign In\" primary}  {button:cancel \"Cancel\" ghost}
|
| ---
|
| This document renders identically on:
|   Browser  → Outconceive WASM → DOM
|   Terminal → pst-terminal → ANSI
|   Desktop  → pst-framebuffer → VGA pixels
|   Serial   → pst-terminal → serial console
|
| Same parser. Same solver. Same VNode tree.
| Different renderer. Same result.
@end card";

pub fn run_with_ps2(ps2: &mut crate::ps2::Ps2) {
    // Render via pst-terminal (ANSI to serial)
    serial_print("\x1b[2J\x1b[H");
    serial_print("\x1b[1;33m");
    serial_print("  === Outconceive Convergence Proof ===\n");
    serial_print("\x1b[0m\n");

    let ansi = pst_terminal::render(CONVERGENCE_DOC, 80, 24);
    serial_print(&ansi);

    serial_print("\n\x1b[2m");
    serial_print("  Rendered via: pst-terminal → ANSI escape sequences\n");
    serial_print("  Same document produces HTML (browser) and pixels (VGA)\n");
    serial_print("\n");

    // Show the raw Markout source alongside
    serial_print("  ─── Source (Markout) ───\n");
    serial_print("\x1b[36m");
    for line in CONVERGENCE_DOC.lines() {
        serial_print("  ");
        serial_print(line);
        serial_print("\n");
    }
    serial_print("\x1b[0m\n");

    // Show what html::to_html produces from the same document
    let lines = pst_markout::parse::parse(CONVERGENCE_DOC);
    let vdom = pst_markout::render::render(&lines);
    let html = pst_markout::html::to_html(&vdom);

    serial_print("  ─── HTML Output (same VNode tree) ───\n");
    serial_print("\x1b[32m");
    let preview = if html.len() > 300 { &html[..300] } else { &html };
    serial_print("  ");
    serial_print(preview);
    serial_print("...\n");
    serial_print("\x1b[0m\n");

    serial_print("  ─── Proof ───\n");
    serial_print("  Parser:    pst-markout (no_std, shared)\n");
    serial_print("  Solver:    libpst constraint solver (shared)\n");
    serial_print("  VNode:     same tree, same structure\n");
    serial_print("  Renderers: html::to_html | pst-terminal | pst-framebuffer\n");
    serial_print("  \x1b[1;32mThe web framework and the OS are the same thing.\x1b[0m\n\n");

    serial_print("\x1b[2m  Press any key to return\x1b[0m\n");
    ps2.read_event();
}
