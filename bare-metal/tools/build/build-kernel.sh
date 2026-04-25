#!/usr/bin/env bash
# Phase 2 — Build the Privion OS production kernel
#
# Produces: privos/kernel/build-privos/kernel/kernel.elf
#
# Usage:
#   bash tools/build/build-kernel.sh           # simulation build (QEMU)
#   bash tools/build/build-kernel.sh --hw      # hardware build (real machine)
#
# The only difference: --hw enables KernelIOMMU, which QEMU doesn't support.
# All other settings are production-grade either way.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
KERNEL_DIR="$REPO_ROOT/kernel"
BUILD_DIR="$KERNEL_DIR/build-privos"

# --- Parse flags ---
SIMULATION=TRUE
IOMMU=OFF
for arg in "$@"; do
    case "$arg" in
        --hw)
            SIMULATION=FALSE
            IOMMU=ON
            ;;
    esac
done

echo "=== Privion OS: Building seL4 kernel ==="
echo "    Mode:       $([ "$SIMULATION" = "TRUE" ] && echo "QEMU simulation" || echo "Hardware")"
echo "    IOMMU:      $IOMMU"
echo "    Output:     $BUILD_DIR/kernel/kernel.elf"
echo ""

# Activate the Python venv (needed for seL4 build tools / nanopb)
VENV="$HOME/.privos-venv"
if [ ! -d "$VENV" ]; then
    echo "ERROR: venv not found at $VENV. Run setup/init-sel4.sh first."
    exit 1
fi
# shellcheck source=/dev/null
source "$VENV/bin/activate"

# --- Configure ---
mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"

# Only re-run cmake if not already configured
if [ ! -f "$BUILD_DIR/build.ninja" ]; then
    "$KERNEL_DIR/init-build.sh" \
        -DPLATFORM=x86_64 \
        -DSIMULATION="$SIMULATION" \
        -DKernelDebugBuild=ON \
        -DKernelPrinting=ON \
        -DKernelFastpath=ON \
        -DKernelRootCNodeSizeBits=19 \
        -DKernelMaxNumNodes=16 \
        -DKernelHugePage=OFF \
        -DKernelIOMMU="$IOMMU"
fi

# --- Build ---
ninja kernel/kernel.elf

echo ""
echo "=== Kernel built ==="
echo "    $(ls -lh "$BUILD_DIR/kernel/kernel.elf")"
echo ""
echo "Verify the kernel loaded correctly in QEMU:"
echo "    ./simulate --extra-qemu-args='-kernel kernel/kernel.elf'"
