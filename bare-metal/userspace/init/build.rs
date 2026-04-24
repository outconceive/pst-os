// build.rs for the init binary.
//
// This script:
//   1. Locates libsel4.a and libsel4runtime.a in the seL4 build directory.
//   2. Tells cargo to link against them.
//   3. Sets the linker script (userspace/link.ld).
//   4. Embeds the initrd (built by tools/build/build-initrd.sh) if present.

use std::env;
use std::path::{Path, PathBuf};

fn main() {
    // Path to the seL4 build directory (set by build-initrd.sh, or default).
    let build_privos = env::var("SEL4_BUILD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            // Default: relative to the workspace root
            let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
            manifest
                .ancestors()
                .nth(2)              // userspace/init → userspace → privos
                .unwrap()
                .join("kernel/build-privos")
        });

    // --- Link against libsel4.a ---
    // During `cargo check` the kernel may not be built yet; emit a warning
    // rather than panicking so type-checking still works.
    if let Some(libsel4_dir) = find_library(&build_privos, "libsel4.a") {
        println!("cargo:rustc-link-search=native={}", libsel4_dir.display());
        println!("cargo:rustc-link-lib=static=sel4");
    } else {
        println!("cargo:warning=libsel4.a not found — run tools/build/build-kernel.sh before building the final binary");
    }

    // --- Link against libsel4runtime.a ---
    // sel4runtime provides _start, TLS setup, and passes bootinfo to main().
    if let Some(runtime_dir) = find_library(&build_privos, "libsel4runtime.a") {
        println!("cargo:rustc-link-search=native={}", runtime_dir.display());
        println!("cargo:rustc-link-lib=static=sel4runtime");
    } else {
        // If sel4runtime isn't found, our own _start (in start.S or Rust) is used.
        println!("cargo:warning=libsel4runtime.a not found — using built-in _start");
    }

    // --- Link against libmuslc.a (C runtime for sel4runtime) ---
    if let Some(musl_dir) = find_library(&build_privos, "libmuslc.a") {
        println!("cargo:rustc-link-search=native={}", musl_dir.display());
        println!("cargo:rustc-link-lib=static=muslc");
    }

    // --- Apply the linker script ---
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let link_ld = manifest_dir.parent().unwrap().join("link.ld"); // userspace/link.ld
    println!("cargo:rustc-link-arg=-T{}", link_ld.display());
    println!("cargo:rustc-link-arg=-nostartfiles");
    println!("cargo:rerun-if-changed={}", link_ld.display());

    // --- Embed the initrd ---
    let initrd_path = build_privos.parent().unwrap()
        .join("tools/build/initrd.bin");
    if initrd_path.exists() {
        println!("cargo:rustc-env=INITRD_PATH={}", initrd_path.display());
        println!("cargo:rerun-if-changed={}", initrd_path.display());
    } else {
        println!("cargo:warning=initrd.bin not found — run tools/build/build-initrd.sh to package services");
    }

    println!("cargo:rerun-if-changed=build.rs");
}

/// Recursively search `root` for a file named `name`, returning its
/// containing directory if found.
fn find_library(root: &Path, name: &str) -> Option<PathBuf> {
    if !root.is_dir() { return None; }
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_library(&path, name) {
                return Some(found);
            }
        } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
            return Some(path.parent().unwrap().to_path_buf());
        }
    }
    None
}
