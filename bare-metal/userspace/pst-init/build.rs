use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let build_privos = env::var("SEL4_BUILD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
            manifest.ancestors().nth(2).unwrap().join("kernel/build-privos")
        });

    // Link seL4 userspace libraries via direct paths + whole-archive
    // to ensure _sel4_start, sel4_vsyscall, etc. are retained.

    if let Some(runtime_path) = find_file(&build_privos, "libsel4runtime.a") {
        println!("cargo:rustc-link-arg=--whole-archive");
        println!("cargo:rustc-link-arg={}", runtime_path.display());
        println!("cargo:rustc-link-arg=--no-whole-archive");
    } else {
        println!("cargo:warning=libsel4runtime.a not found");
    }

    if let Some(sel4_path) = find_file(&build_privos, "libsel4.a") {
        println!("cargo:rustc-link-arg={}", sel4_path.display());
    } else {
        println!("cargo:warning=libsel4.a not found");
    }

    if let Some(musl_path) = find_file(&build_privos, "libc.a") {
        println!("cargo:rustc-link-arg={}", musl_path.display());
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let link_ld = manifest_dir.parent().unwrap().join("link.ld");
    println!("cargo:rustc-link-arg=-T{}", link_ld.display());
    println!("cargo:rerun-if-changed={}", link_ld.display());
    println!("cargo:rerun-if-changed=build.rs");
}

fn find_file(root: &Path, name: &str) -> Option<PathBuf> {
    if !root.is_dir() { return None; }
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file(&path, name) {
                return Some(found);
            }
        } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
            return Some(path);
        }
    }
    None
}
