#!/usr/bin/env python3
"""
Micro-OS Hugging Face Uploader
Uploads Micro-OS.iso and project documentation to Hugging Face Hub.
"""

import os
import sys
import argparse
from huggingface_hub import HfApi, create_repo, upload_file

def main():
    parser = argparse.ArgumentParser(description="Upload Micro-OS to Hugging Face")
    parser.add_argument("--repo-id", default=None, help="Hugging Face repository ID (e.g. 'YVyshnavi/Micro-OS')")
    parser.add_argument("--token", default=None, help="Hugging Face User Access Token (from https://huggingface.co/settings/tokens)")
    parser.add_argument("--repo-type", default="model", choices=["model", "dataset"], help="Repo type (default: model)")
    args = parser.parse_args()

    token = args.token or os.environ.get("HF_TOKEN")
    repo_id = args.repo_id

    iso_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "Micro-OS.iso")
    if not os.path.exists(iso_path):
        print("❌ Error: Micro-OS.iso not found! Run 'py build_iso.py' first.")
        sys.exit(1)

    print("==================================================")
    print("🤗 HUGGING FACE UPLOADER FOR MICRO-OS")
    print("==================================================")

    api = HfApi(token=token)

    # Check authentication
    try:
        user_info = api.whoami()
        username = user_info.get("name")
        print(f"✅ Authenticated as Hugging Face user: {username}")
        if not repo_id:
            repo_id = f"{username}/Micro-OS"
    except Exception as e:
        print("\n🔑 Hugging Face Authentication Required!")
        print("To push to Hugging Face, you need a free Access Token with 'Write' permissions.")
        print("1. Go to: https://huggingface.co/settings/tokens")
        print("2. Click 'Create new token' -> Select 'Write'")
        print("3. Run this command with your token:")
        print("   py push_to_hf.py --token <YOUR_HF_TOKEN>\n")
        sys.exit(1)

    print(f"📦 Target Repository: https://huggingface.co/{repo_id} ({args.repo_type})")
    print(f"💿 Uploading: Micro-OS.iso ({os.path.getsize(iso_path) / (1024*1024):.2f} MB)...")

    # Create repo if not exists
    create_repo(repo_id=repo_id, repo_type=args.repo_type, exist_ok=True, token=token)

    # Upload ISO file
    api.upload_file(
        path_or_fileobj=iso_path,
        path_in_repo="Micro-OS.iso",
        repo_id=repo_id,
        repo_type=args.repo_type,
        commit_message="Add bootable UEFI ISO (Micro-OS.iso)"
    )

    # Create and upload HF Model Card / README
    hf_readme = f"""---
license: mit
tags:
- operating-system
- uefi
- rust
- bare-metal
- microkernel
---

# Micro-OS (Rust UEFI 64-bit Microkernel)
**Author:** Yeddula Vyshnavi (USN: `25BTRCL294`, Section `E`, JAIN FET Campus)  
**Course:** 25PCCS201 - Operating Systems  

Bootable UEFI 64-bit graphical operating system written in `#![no_std]` Rust with hardware dirty-rect cursor buffering, linear 1080p GOP graphics, real laptop Wi-Fi integration, RTC clock, and native desktop applications.

## 💿 Bootable ISO Download
- **Filename:** `Micro-OS.iso`
- **Architecture:** `x86_64` (UEFI El Torito Hybrid ISO)
- **Compatibility:** Oracle VirtualBox, VMware, QEMU, Rufus, and Physical UEFI PCs.

## 🚀 How to Run
### In VirtualBox:
1. Create a new VM: Type `Other`, Version `Other/Unknown (64-bit)`.
2. Enable **EFI (special OSes only)** in Settings -> System.
3. Attach `Micro-OS.iso` to the Optical (SATA DVD) drive.
4. Click **Start**.
"""
    readme_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "HF_README.md")
    with open(readme_path, "w", encoding="utf-8") as f:
        f.write(hf_readme)

    api.upload_file(
        path_or_fileobj=readme_path,
        path_in_repo="README.md",
        repo_id=repo_id,
        repo_type=args.repo_type,
        commit_message="Add Micro-OS Model Card and documentation"
    )
    if os.path.exists(readme_path):
        os.remove(readme_path)

    print("==================================================")
    print(f"🎉 SUCCESS! Micro-OS.iso is live on Hugging Face:")
    print(f"👉 https://huggingface.co/{repo_id}")
    print("==================================================")

if __name__ == "__main__":
    main()
