#!/usr/bin/env bash
# Generate Rust FFI bindings from seL4 C headers using bindgen.
#
# Run this after Phase 1 (init-sel4.sh) has built the kernel.
# The build step generates architecture-specific headers needed by bindgen.
#
# The generated bindings.rs will eventually replace the manual types in
# userspace/bindings/sel4-sys/src/lib.rs. For now, the manual types are
# used to allow the workspace to compile without running this script.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

KERNEL_SRC="$REPO_ROOT/kernel/kernel"
BUILD_DIR="$REPO_ROOT/kernel/build-privos"
OUT="$REPO_ROOT/userspace/bindings/sel4-sys/src/bindings.rs"

# seL4 header paths (source + build-generated)
SEL4_HEADER="$KERNEL_SRC/libsel4/include/sel4/sel4.h"
INCLUDE_DIRS=(
    "$KERNEL_SRC/libsel4/include"
    "$KERNEL_SRC/libsel4/arch_include/x86"
    "$KERNEL_SRC/libsel4/sel4_arch_include/x86_64"
    "$BUILD_DIR/kernel/gen_headers"
    "$BUILD_DIR/kernel/generated"
)

if [ ! -f "$SEL4_HEADER" ]; then
    echo "ERROR: seL4 headers not found."
    echo "  Expected: $SEL4_HEADER"
    echo "Run setup/init-sel4.sh first, then tools/build/build-kernel.sh."
    exit 1
fi

# Build the -I flags
INCLUDE_FLAGS=()
for dir in "${INCLUDE_DIRS[@]}"; do
    if [ -d "$dir" ]; then
        INCLUDE_FLAGS+=("-I" "$dir")
    else
        echo "WARNING: include directory not found: $dir"
    fi
done

echo "Generating sel4-sys bindings..."
echo "  Source: $SEL4_HEADER"
echo "  Output: $OUT"

bindgen "$SEL4_HEADER" \
    --use-core \
    --no-layout-tests \
    --no-doc-comments \
    --raw-line "#![allow(non_upper_case_globals)]" \
    --raw-line "#![allow(non_camel_case_types)]" \
    --raw-line "#![allow(non_snake_case)]" \
    --raw-line "#![allow(dead_code)]" \
    "${INCLUDE_FLAGS[@]}" \
    -- \
    -target x86_64-unknown-linux-gnu \
    -DCONFIG_KERNEL_MCS=0 \
    -o "$OUT"

echo ""
echo "Done: $OUT"
echo "NOTE: The generated file replaces the manual types in sel4-sys/src/lib.rs."
echo "      Update lib.rs to include!(\"bindings.rs\") and remove manual definitions."
