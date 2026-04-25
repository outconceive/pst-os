# Persistence

PST OS saves state to a virtio-blk disk.

## What's Saved

- Desktop state (window contents)
- Files created with the editor
- Browser pages seeded to `/pst/`
- Configuration files

## How It Works

The virtio-blk driver speaks the virtio protocol over I/O ports. Block reads and writes go through a virtqueue with DMA buffers.

### Disk Layout

| Blocks | Content |
|--------|---------|
| 0-15 | Desktop state (window titles + content) |
| 32 | File directory (names + block ranges) |
| 33-48 | File directory entries |
| 64+ | File content |

## Saving

- **Esc** from desktop saves window state
- **Esc** from editor saves the file
- Files persist across reboots

## QEMU Setup

The boot script creates a 1MB disk image:

```bash
dd if=/dev/zero of=pst-disk.img bs=1M count=1
qemu-system-x86_64 -cdrom pst-os.iso -drive file=pst-disk.img,format=raw,if=virtio
```

VirtualBox doesn't have virtio-blk, so persistence is QEMU-only for now.
