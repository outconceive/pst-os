# Getting Started

PST OS is an operating system where everything — processes, files, messages, UI — is a flat table of parallel strings. No trees, no pointer graphs, no hierarchical data structures.

## What You'll Need

- **VirtualBox** or **QEMU** (any platform)
- The `pst-os.iso` boot image

## Quick Start

### Windows (VirtualBox)

Double-click `run-pst-win.bat`. It creates a VM and boots the ISO.

### Any Platform (QEMU)

```bash
qemu-system-x86_64 -cdrom pst-os.iso -m 2G -serial stdio
```

### Mac/Linux

```bash
bash run-pst.sh
```

## What You'll See

PST OS boots to a windowed desktop with:
- Multiple windows (Tab to switch)
- GUI buttons (click with mouse)
- A Markout shell (type Markout, see it render)
- A text editor (F1)
- A document browser (F3)
- A code stepper (F4)

All rendered from Markout — the same declarative language that works in the browser, terminal, and on bare metal.

## Next Steps

- [Your First Markout](/guide/first-markout) — write your first UI
- [Components](/guide/components) — all 18 component types
- [Architecture](/architecture/overview) — how it works under the hood
