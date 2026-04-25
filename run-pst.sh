#!/bin/bash
echo "=================================================="
echo "  Parallel String Theory OS"
echo "  One primitive. One loop. One OS."
echo "=================================================="
echo

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ISO="$SCRIPT_DIR/bare-metal/tools/image/pst-os.iso"

if [ ! -f "$ISO" ]; then
    echo "ERROR: ISO not found at $ISO"
    echo "Build it first: cd bare-metal && bash tools/build/boot-pst.sh --build"
    exit 1
fi

QEMU=$(which qemu-system-x86_64 2>/dev/null)
if [ -z "$QEMU" ]; then
    echo "QEMU not found. Install it:"
    if [ "$(uname)" = "Darwin" ]; then
        echo "  brew install qemu"
    else
        echo "  sudo apt install qemu-system-x86"
    fi
    exit 1
fi

echo "QEMU: $QEMU"
echo "ISO:  $ISO"
echo
echo "Booting PST OS... Ctrl+A then X to quit."
echo

$QEMU -cdrom "$ISO" -m 2G -serial stdio -no-reboot
