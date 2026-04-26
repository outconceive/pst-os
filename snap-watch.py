"""
Pokemon Snap for PST OS Storybook

Watches the VM serial output for <<SNAP:name>> markers,
takes a VirtualBox screenshot on each one, zips them all.

Usage:
  1. Boot PST OS in VirtualBox
  2. Open the Storybook (click button or F7)
  3. Run this script:  python snap-storybook.py
  4. Press 'P' in the storybook to start Snap All
  5. Screenshots appear in storybook-snaps.zip

Prerequisites:
  - VBoxManage on PATH
  - VM named "PST-OS" (or pass --vm <name>)
  - pip install pyserial (optional, for COM port)
"""

import subprocess
import time
import os
import sys
import zipfile
import argparse
from pathlib import Path

def find_serial_log():
    """Find the PST OS serial log."""
    candidates = [
        Path("pst-serial.log"),
        Path("bare-metal/tools/image/pst-serial.log"),
    ]
    for c in candidates:
        if c.exists():
            return c
    return None

def vbox_screenshot(vm_name, output_path):
    """Take a VirtualBox screenshot."""
    try:
        subprocess.run(
            ["VBoxManage", "controlvm", vm_name, "screenshotpng", str(output_path)],
            check=True, capture_output=True, timeout=10)
        return True
    except (subprocess.CalledProcessError, FileNotFoundError, subprocess.TimeoutExpired) as e:
        print(f"  Screenshot failed: {e}")
        return False

def sanitize_filename(name):
    """Make a string safe for filenames."""
    return "".join(c if c.isalnum() or c in "-_ " else "_" for c in name).strip()

def watch_serial_pipe(vm_name, output_dir):
    """Watch VBoxManage showvminfo for serial port, poll the log file."""
    snaps = []
    print("Watching for SNAP markers on serial output...")
    print("Press 'P' in the storybook to start Snap All.")
    print()

    # Try to find serial log via VBoxManage
    try:
        info = subprocess.run(
            ["VBoxManage", "showvminfo", vm_name, "--machinereadable"],
            capture_output=True, text=True, timeout=5)
        for line in info.stdout.splitlines():
            if "uart" in line.lower() and "file" in line.lower():
                # Extract log path
                parts = line.split("=", 1)
                if len(parts) == 2:
                    path = parts[1].strip('"').strip()
                    if os.path.exists(path):
                        print(f"  Serial log: {path}")
    except Exception:
        pass

    # Poll serial log file
    serial_log = find_serial_log()
    if serial_log is None:
        # Try reading from VBox serial pipe via COM port name
        print("  No serial log found. Will poll via VBoxManage screenshotpng timing.")
        print("  For best results, configure VM serial port to a file.")

    # Fallback: just take timed screenshots
    if serial_log is None:
        return watch_timed(vm_name, output_dir)

    print(f"  Watching: {serial_log}")
    print()

    seen_markers = set()
    done = False

    while not done:
        time.sleep(0.05)

        with open(serial_log, "r", errors="replace") as f:
            content = f.read()

        for line in content.splitlines():
            line = line.strip()

            if "<<SNAP_BEGIN>>" in line:
                started = True
                print("  Snap All started!")
                continue

            if "<<SNAP:" in line and ">>" in line[line.index("<<SNAP:"):]:
                start_idx = line.index("<<SNAP:") + 7
                end_idx = line.index(">>", start_idx)
                name = line[start_idx:end_idx]

                if name in seen_markers:
                    continue
                seen_markers.add(name)

                safe = sanitize_filename(name)
                filename = f"{len(snaps)+1:02d}_{safe}.png"
                filepath = output_dir / filename

                if vbox_screenshot(vm_name, filepath):
                    snaps.append((name, filepath))
                    print(f"  [{len(snaps):2d}] {name}")
                continue

            if "<<SNAP_END:" in line:
                done = True
                break

    return snaps

def watch_timed(vm_name, output_dir):
    """Fallback: take screenshots every 2 seconds."""
    print("  Timed mode: taking a screenshot every 2 seconds.")
    print("  Press Ctrl+C when done.")
    snaps = []
    try:
        i = 0
        while True:
            filename = f"{i+1:02d}_snap.png"
            filepath = output_dir / filename
            if vbox_screenshot(vm_name, filepath):
                snaps.append((f"snap_{i+1}", filepath))
                print(f"  [{i+1}] captured")
            i += 1
            time.sleep(2)
    except KeyboardInterrupt:
        print("\n  Stopped.")
    return snaps

def main():
    parser = argparse.ArgumentParser(description="Pokemon Snap for PST OS Storybook")
    parser.add_argument("--vm", default="PST-OS", help="VirtualBox VM name")
    parser.add_argument("--out", default="storybook-snaps", help="Output directory")
    args = parser.parse_args()

    output_dir = Path(args.out)
    output_dir.mkdir(exist_ok=True)

    print("=" * 50)
    print("  PST OS Storybook Snap")
    print(f"  VM: {args.vm}")
    print(f"  Output: {output_dir}/")
    print("=" * 50)
    print()

    snaps = watch_serial_pipe(args.vm, output_dir)

    if not snaps:
        print("No snaps captured.")
        return

    # Zip them up
    zip_path = Path(f"{args.out}.zip")
    with zipfile.ZipFile(zip_path, 'w', zipfile.ZIP_DEFLATED) as zf:
        for name, filepath in snaps:
            zf.write(filepath, filepath.name)

    print()
    print(f"  {len(snaps)} snaps captured")
    print(f"  Zip: {zip_path} ({zip_path.stat().st_size // 1024} KB)")
    print()

    # Summary
    print("  Contents:")
    for i, (name, filepath) in enumerate(snaps):
        size = filepath.stat().st_size // 1024
        print(f"    {i+1:2d}. {name} ({size} KB)")

if __name__ == "__main__":
    main()
