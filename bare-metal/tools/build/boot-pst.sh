#!/bin/bash
# boot-pst.sh — Build and boot PST OS on QEMU
#
# Prerequisites:
#   1. seL4 kernel built: bash tools/build/build-kernel.sh
#   2. Dependencies installed: bash setup/install-deps.sh
#
# Usage:
#   bash tools/build/boot-pst.sh          # Build and boot
#   bash tools/build/boot-pst.sh --build  # Build only
#   bash tools/build/boot-pst.sh --boot   # Boot only (assumes built)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BUILD_DIR="$REPO_ROOT/kernel/build-privos"
IMAGE_DIR="$REPO_ROOT/tools/image"
TARGET_DIR="$REPO_ROOT/target/x86_64-unknown-none/release"

BUILD_ONLY=false
BOOT_ONLY=false

for arg in "$@"; do
    case $arg in
        --build) BUILD_ONLY=true ;;
        --boot)  BOOT_ONLY=true ;;
    esac
done

# =========================================================================
# Build
# =========================================================================

if [ "$BOOT_ONLY" = false ]; then
    echo "=== Building PST OS ==="
    echo ""

    # Check kernel
    if [ ! -f "$BUILD_DIR/kernel/kernel.elf" ]; then
        echo "ERROR: seL4 kernel not built."
        echo "Run: bash tools/build/build-kernel.sh"
        exit 1
    fi
    echo "[ok] seL4 kernel found"

    # Build pst-init
    echo "[..] Compiling pst-init for x86_64-unknown-none..."
    cd "$REPO_ROOT"
    SEL4_BUILD_DIR="$BUILD_DIR" \
        cargo build --release -p pst-init

    if [ ! -f "$TARGET_DIR/pst-init" ]; then
        echo "ERROR: pst-init binary not found at $TARGET_DIR/pst-init"
        exit 1
    fi
    echo "[ok] pst-init compiled ($(wc -c < "$TARGET_DIR/pst-init") bytes)"

    # Create bootable ISO
    echo "[..] Creating boot image..."
    mkdir -p "$IMAGE_DIR"
    ISO_TMP=$(mktemp -d)
    mkdir -p "$ISO_TMP/boot/grub"

    cp "$BUILD_DIR/kernel/kernel.elf" "$ISO_TMP/boot/kernel.elf"
    cp "$TARGET_DIR/pst-init" "$ISO_TMP/boot/init.elf"

    cat > "$ISO_TMP/boot/grub/grub.cfg" << 'GRUBCFG'
set timeout=0
set default=0

menuentry "PST OS — Parallel String Theory" {
    multiboot2 /boot/kernel.elf
    module2    /boot/init.elf
    boot
}
GRUBCFG

    grub-mkrescue -o "$IMAGE_DIR/pst-os.iso" "$ISO_TMP" 2>/dev/null
    rm -rf "$ISO_TMP"

    echo "[ok] Boot image: $IMAGE_DIR/pst-os.iso"
    echo ""
fi

# =========================================================================
# Boot
# =========================================================================

if [ "$BUILD_ONLY" = false ]; then
    if [ ! -f "$IMAGE_DIR/pst-os.iso" ]; then
        echo "ERROR: pst-os.iso not found. Run without --boot first."
        exit 1
    fi

    echo "=== Booting PST OS in QEMU ==="
    echo "    (serial output on stdio, Ctrl+A X to quit)"
    echo ""

    # Create persistence disk if it doesn't exist
    DISK="$IMAGE_DIR/pst-disk.img"
    if [ ! -f "$DISK" ]; then
        dd if=/dev/zero of="$DISK" bs=1M count=1 2>/dev/null
        echo "[ok] Created 1MB persistence disk"
    fi

    qemu-system-x86_64 \
        -cdrom "$IMAGE_DIR/pst-os.iso" \
        -drive file="$DISK",format=raw,if=virtio \
        -device virtio-net-pci,netdev=net0 \
        -netdev user,id=net0 \
        -cpu qemu64,+pdpe1gb \
        -m 2G \
        -smp 2 \
        -serial stdio \
        -display none \
        -no-reboot
fi
