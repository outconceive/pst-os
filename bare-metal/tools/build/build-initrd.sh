#!/usr/bin/env bash
# Build all service binaries and package them into initrd.bin.
#
# initrd.bin is embedded by init/build.rs into the init binary at compile
# time. init reads it at runtime to find and spawn service processes.
#
# Run this BEFORE building init:
#   bash tools/build/build-initrd.sh
#   cargo build --release -p init
#
# The initrd format is defined in libprivos/src/initrd.rs.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUT="$SCRIPT_DIR/initrd.bin"

# Activate Rust venv / cargo
source "$HOME/.cargo/env" 2>/dev/null || true
source "$HOME/.privos-venv/bin/activate" 2>/dev/null || true

echo "=== Building Privion OS service binaries ==="

SERVICES=(
    "cryptod"
    "vfs"
    "netd"
    "driverd"
    "driver-nic"
    "compositor"
)

# Build all service binaries for x86_64-unknown-none
cd "$REPO_ROOT/userspace"
cargo build --release $(printf -- '-p %s ' "${SERVICES[@]}")

TARGET_DIR="$REPO_ROOT/userspace/target/x86_64-unknown-none/release"

echo ""
echo "=== Packaging initrd ==="

# Write the initrd binary
python3 - "$OUT" "${SERVICES[@]}" <<'PYTHON'
import sys
import struct
import os

out_path = sys.argv[1]
names    = sys.argv[2:]

target_dir = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(out_path))),
    "userspace/target/x86_64-unknown-none/release"
)

entries = []
for name in names:
    elf_path = os.path.join(target_dir, name)
    if not os.path.exists(elf_path):
        print(f"WARNING: {elf_path} not found, skipping")
        continue
    with open(elf_path, "rb") as f:
        data = f.read()
    entries.append((name, data))
    print(f"  {name}: {len(data):,} bytes")

# Calculate offsets
# Header: 4 (magic) + 4 (count) = 8 bytes
# Entry headers: count * (32 + 8 + 8) bytes
header_size = 8 + len(entries) * 48
offset = header_size

with open(out_path, "wb") as f:
    # Magic + count
    f.write(b"PRIV")
    f.write(struct.pack("<I", len(entries)))

    # Entry headers
    for name, data in entries:
        name_bytes = name.encode("utf-8")[:31]
        name_field = name_bytes + b"\x00" * (32 - len(name_bytes))
        f.write(name_field)
        f.write(struct.pack("<Q", offset))
        f.write(struct.pack("<Q", len(data)))
        offset += len(data)

    # ELF data
    for _, data in entries:
        f.write(data)

print(f"\nInitrd: {out_path} ({os.path.getsize(out_path):,} bytes, {len(entries)} services)")
PYTHON

echo ""
echo "=== Building init binary ==="

cd "$REPO_ROOT/userspace"
SEL4_BUILD_DIR="$REPO_ROOT/kernel/build-privos" \
    cargo build --release -p init

INIT_ELF="$REPO_ROOT/userspace/target/x86_64-unknown-none/release/init"
echo "init ELF: $INIT_ELF ($(ls -lh "$INIT_ELF" | awk '{print $5}'))"
echo ""
echo "Next: bash tools/build/build-image.sh"
