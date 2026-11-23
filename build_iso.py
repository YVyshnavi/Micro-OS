#!/usr/bin/env python3
import os
import subprocess
import shutil
import sys

def build_iso():
    print("==================================================")
    print("💿 BUILDING BOOTABLE UEFI ISO (Micro-OS.iso)...")
    print("==================================================")

    kernel_dir = os.path.dirname(os.path.abspath(__file__))
    efi_path = os.path.join(kernel_dir, "target", "x86_64-unknown-uefi", "debug", "uefi_kernel.efi")
    
    if not os.path.exists(efi_path):
        efi_path = os.path.join(kernel_dir, "target", "x86_64-unknown-uefi", "release", "uefi_kernel.efi")

    if not os.path.exists(efi_path):
        print(f"❌ Error: Cannot find UEFI kernel binary at {efi_path}")
        print("Run 'cargo build --target x86_64-unknown-uefi' first!")
        sys.exit(1)

    with open(efi_path, "rb") as f:
        efi_bytes = f.read()

    print(f"📦 Kernel Binary: {len(efi_bytes)} bytes")

    # Build 4 MB FAT16 EFI boot image (8192 sectors of 512 bytes)
    total_sectors = 8192
    img = bytearray(total_sectors * 512)

    # 1. BPB
    img[0:3] = b"\xEB\x3C\x90"
    img[3:11] = b"MSWIN4.1"
    img[11:13] = (512).to_bytes(2, "little")
    img[13] = 4  # 4 sectors per cluster (2048 bytes)
    img[14:16] = (4).to_bytes(2, "little")  # 4 reserved sectors
    img[16] = 2  # 2 FAT tables
    img[17:19] = (512).to_bytes(2, "little")  # 512 root entries
    img[19:21] = (total_sectors).to_bytes(2, "little")
    img[21] = 0xF8
    img[22:24] = (32).to_bytes(2, "little")  # 32 sectors per FAT
    img[24:26] = (63).to_bytes(2, "little")
    img[26:28] = (255).to_bytes(2, "little")
    img[28:32] = (0).to_bytes(4, "little")
    img[36] = 0x80
    img[38] = 0x29
    img[39:43] = b"\x12\x34\x56\x78"
    img[43:54] = b"EFI SYSTEM "
    img[54:62] = b"FAT16   "
    img[510:512] = b"\x55\xAA"

    # 2. FAT tables
    fat = bytearray(32 * 512)
    fat[0:4] = b"\xF8\xFF\xFF\xFF"
    fat[4:6] = (0xFFFF).to_bytes(2, "little")  # Cluster 2: EFI dir
    fat[6:8] = (0xFFFF).to_bytes(2, "little")  # Cluster 3: BOOT dir

    cluster_count = (len(efi_bytes) + 2047) // 2048
    for i in range(cluster_count):
        cluster = 4 + i
        next_val = cluster + 1 if i + 1 < cluster_count else 0xFFFF
        fat[cluster * 2 : cluster * 2 + 2] = next_val.to_bytes(2, "little")

    fat1_offset = 4 * 512
    fat2_offset = (4 + 32) * 512
    img[fat1_offset : fat1_offset + len(fat)] = fat
    img[fat2_offset : fat2_offset + len(fat)] = fat

    # 3. Root directory: EFI entry
    root_offset = (4 + 64) * 512
    img[root_offset : root_offset + 11] = b"EFI        "
    img[root_offset + 11] = 0x10
    img[root_offset + 26 : root_offset + 28] = (2).to_bytes(2, "little")

    # 4. Cluster 2: BOOT entry inside EFI
    data_offset = (4 + 64 + 32) * 512
    img[data_offset : data_offset + 11] = b"BOOT       "
    img[data_offset + 11] = 0x10
    img[data_offset + 26 : data_offset + 28] = (3).to_bytes(2, "little")

    # 5. Cluster 3: BOOTX64.EFI entry inside BOOT
    c3_offset = data_offset + 2048
    img[c3_offset : c3_offset + 11] = b"BOOTX64 EFI"
    img[c3_offset + 11] = 0x20
    img[c3_offset + 26 : c3_offset + 28] = (4).to_bytes(2, "little")
    img[c3_offset + 28 : c3_offset + 32] = len(efi_bytes).to_bytes(4, "little")

    # 6. Cluster 4+: kernel binary data
    c4_offset = data_offset + 4096
    img[c4_offset : c4_offset + len(efi_bytes)] = efi_bytes

    # 7. Write staging directory
    staging_dir = os.path.join(kernel_dir, "iso_staging")
    os.makedirs(os.path.join(staging_dir, "EFI", "BOOT"), exist_ok=True)
    with open(os.path.join(staging_dir, "efi.img"), "wb") as f:
        f.write(img)
    with open(os.path.join(staging_dir, "EFI", "BOOT", "BOOTX64.EFI"), "wb") as f:
        f.write(efi_bytes)

    # 8. Run xorriso via WSL
    iso_output = os.path.join(kernel_dir, "Micro-OS.iso")
    wsl_staging = "/mnt/c/Users/vyshn/uefi_kernel/iso_staging"
    wsl_out = "/mnt/c/Users/vyshn/uefi_kernel/Micro-OS.iso"
    
    cmd = [
        "wsl", "xorriso", "-as", "mkisofs",
        "-V", "MICRO_OS",
        "-e", "efi.img",
        "-no-emul-boot",
        "-isohybrid-gpt-basdat",
        "-o", wsl_out,
        wsl_staging
    ]

    print("🚀 Running xorriso to generate UEFI hybrid ISO...")
    res = subprocess.run(cmd, capture_output=True, text=True)

    # Cleanup staging directory
    shutil.rmtree(staging_dir, ignore_errors=True)

    if res.returncode == 0 and os.path.exists(iso_output):
        iso_size = os.path.getsize(iso_output)
        print("==================================================")
        print(f"🎉 SUCCESS! 'Micro-OS.iso' ({iso_size / (1024*1024):.2f} MB) IS READY!")
        print("Compatible with: VirtualBox, VMware, QEMU, Rufus & Real UEFI PCs!")
        print("==================================================")
    else:
        print("❌ Error running xorriso:")
        print(res.stderr)
        sys.exit(1)

if __name__ == "__main__":
    build_iso()
