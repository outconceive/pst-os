# seL4 Integration

PST OS runs on the seL4 microkernel — the only formally verified OS kernel in production use.

## Why seL4

- **Formal verification**: memory isolation and capability enforcement proven correct by mathematical proof
- **Capability-based security**: every resource accessed through unforgeable capabilities
- **Minimal kernel**: ~10K lines of C, verified end-to-end
- **Zero telemetry**: architectural consequence of capability isolation

## Boot Sequence

1. GRUB loads kernel.elf and init.elf via multiboot2
2. seL4 kernel boots, creates rootserver (pst-init)
3. Custom entry point saves bootinfo from kernel's rdi register
4. Heap allocator initializes (4MB static BSS)
5. Constraint solver computes process boot order
6. Markout parser renders boot document
7. VGA framebuffer mapped via PCI probe + page table setup
8. Bochs VBE switches to 640x480x32 graphics mode
9. Keyboard and mouse IRQs registered via IOAPIC
10. Desktop enters interactive loop

## Syscall Shims

seL4's C API functions are inline functions in libsel4. PST OS reimplements them in Rust inline assembly:

- `seL4_Call` — object invocation
- `seL4_Recv` — wait on endpoint/notification  
- `seL4_Untyped_Retype` — create kernel objects
- `seL4_X86_Page_Map` — map pages into VSpace
- `seL4_IRQControl_GetIOAPIC` — register IRQ handlers

All with explicit rsp save/restore for x86-64 syscall convention.

## Device Drivers

| Driver | PCI | IRQ | Protocol |
|--------|-----|-----|----------|
| VGA | BAR auto-detect (MMIO/IO) | — | Bochs VBE mode set |
| Keyboard | Port 0x60-0x64 | IRQ 1 (IOAPIC) | PS/2 scancode set 1 |
| Mouse | Port 0x60-0x64 | IRQ 12 (IOAPIC) | PS/2 3-byte packets |
| Block | Port (virtio legacy) | — | Virtio-blk virtqueue |
| Network | Port (virtio legacy) | — | Virtio-net + smoltcp |

## Binary Size

217KB static ELF. Kernel interface, all drivers, Markout parser, constraint solver, five renderers, all applications, smoltcp TCP/IP stack.
