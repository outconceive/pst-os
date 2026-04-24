#!/usr/bin/env bash
# Build the complete Privion OS bootable image.
#
# Prerequisites:
#   - Phase 1: setup/init-sel4.sh (seL4 kernel source fetched)
#   - Phase 2: tools/build/build-kernel.sh (kernel.elf built)
#   - Phase 4: tools/build/build-initrd.sh (services packaged, init built)
#
# Output: tools/image/privion.iso + tools/image/privion.iso.sha256
#
# Usage:
#   bash tools/build/build-image.sh           # simulation build (QEMU)
#   bash tools/build/build-image.sh --hw      # hardware build (IOMMU enabled)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
IMAGE_DIR="$SCRIPT_DIR/../image"

SIMULATION=TRUE
for arg in "$@"; do
    [[ "$arg" == "--hw" ]] && SIMULATION=FALSE
done

source "$HOME/.privos-venv/bin/activate" 2>/dev/null || true

echo "=== Privion OS: Building bootable image ==="
echo "    Simulation: $SIMULATION"

KERNEL_ELF="$REPO_ROOT/kernel/build-privos/kernel/kernel.elf"
INIT_ELF="$REPO_ROOT/userspace/target/x86_64-unknown-none/release/init"

for f in "$KERNEL_ELF" "$INIT_ELF"; do
    if [ ! -f "$f" ]; then
        echo "ERROR: Required binary not found: $f"
        echo "Run the prerequisite build steps first."
        exit 1
    fi
done

mkdir -p "$IMAGE_DIR/iso/boot/grub"

# --- [1/4] Copy kernel ---
cp "$KERNEL_ELF" "$IMAGE_DIR/iso/boot/kernel.elf"

# --- [2/4] Create the seL4 system image ---
# The seL4 build system provides a tool (elfloader) that combines the kernel
# ELF with the rootserver (init) ELF into a single loadable image.
# Locate the elfloader binary from the seL4 build.
ELFLOADER="$REPO_ROOT/kernel/build-privos/elfloader/elfloader"

if [ -f "$ELFLOADER" ]; then
    echo "[2/4] Packaging kernel + init with elfloader..."
    "$ELFLOADER" \
        -k "$KERNEL_ELF" \
        -l "$INIT_ELF" \
        -o "$IMAGE_DIR/iso/boot/privion.img"
else
    # Fallback: use the seL4 Python image creation tool
    echo "[2/4] elfloader not found, trying seL4 image tool..."
    python3 "$REPO_ROOT/kernel/tools/seL4/cmake-tool/helpers/image.py" \
        --kernel "$KERNEL_ELF" \
        --loader "$INIT_ELF" \
        --output "$IMAGE_DIR/iso/boot/privion.img" 2>/dev/null \
    || {
        # Last resort: multiboot2 load them separately (works for QEMU testing)
        echo "[2/4] Using multiboot2 direct load..."
        cp "$INIT_ELF" "$IMAGE_DIR/iso/boot/init.elf"
    }
fi

# --- [3/4] Write GRUB config ---
echo "[3/4] Writing GRUB config..."
cat > "$IMAGE_DIR/iso/boot/grub/grub.cfg" << 'GRUB'
set default=0
set timeout=3

menuentry "Privion OS" {
    multiboot2 /boot/kernel.elf
    module2    /boot/init.elf
    boot
}

menuentry "Privion OS (debug kernel output)" {
    # Rebuild with KernelPrinting=ON for this to show seL4 debug output
    multiboot2 /boot/kernel.elf
    module2    /boot/init.elf
    boot
}
GRUB

cp "$INIT_ELF" "$IMAGE_DIR/iso/boot/init.elf" 2>/dev/null || true

# --- [4/4] Create ISO and checksums ---
echo "[4/4] Creating ISO..."
grub-mkrescue -o "$IMAGE_DIR/privion.iso" "$IMAGE_DIR/iso/"
sha256sum "$IMAGE_DIR/privion.iso" > "$IMAGE_DIR/privion.iso.sha256"

echo ""
echo "=== Build complete ==="
echo "    ISO:      $IMAGE_DIR/privion.iso"
echo "    Checksum: $IMAGE_DIR/privion.iso.sha256"
echo ""
echo "Test in QEMU:"
echo "  qemu-system-x86_64 \\"
echo "    -cdrom $IMAGE_DIR/privion.iso \\"
echo "    -m 2G -smp 2 -enable-kvm \\"
echo "    -device virtio-net-pci,netdev=net0 \\"
echo "    -netdev user,id=net0 \\"
echo "    -serial stdio -display none"
