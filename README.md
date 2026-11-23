# VyshOS - Bare-Metal UEFI Microkernel & Graphical OS in Rust

A standalone, bare-metal 64-bit UEFI graphical operating system written completely in Rust (`no_std`). VyshOS boots directly from UEFI firmware, initializes a 1080p high-resolution linear framebuffer via UEFI GOP, interacts directly with x86 hardware I/O ports, and provides a multi-tasking graphical desktop environment with interactive applications and smooth touchpad/mousepad integration.

---

## 🎓 Academic Information
- **Author:** Yeddula Vyshnavi
- **USN:** `25BTRCL294` (Section `E`)
- **Institution:** JAIN (Deemed-to-be University), Faculty of Engineering and Technology (FET Campus)
- **Course:** `25PCCS201: Operating Systems`

---

## 🚀 Key Features & Architecture

### 1. Bare-Metal UEFI Runtime (`x86_64-unknown-uefi`)
- Completely freestanding `#![no_std]` and `#![no_main]` Rust implementation.
- Directly invokes UEFI Boot Services and Runtime Services via native EFI ABI function pointers.
- Queries and sets the highest supported display mode (1920×1080 Full HD) via UEFI Graphics Output Protocol (GOP).

### 2. Native x86 Hardware PS/2 Mouse & Touchpad Driver
- Direct inline assembly communication (`in al, dx` and `out dx, al`) with the motherboard 8042 PS/2 controller at I/O ports `0x60` and `0x64`.
- Decodes 3-byte standard mouse packets for `(dx, dy)` movement and left/right clicks.
- Compatible with VirtualBox `--mouse ps2` and physical laptop touchpads.

### 3. Flicker-Free Dirty-Rect Hardware Cursor Buffer
- Employs a dedicated `MouseCursorBuffer` (saving/restoring 256 pixels) under a high-visibility 16×16 arrow pointer.
- Eliminates 100% of display tearing, smearing, and screen flicker by repainting only the cursor region during motion rather than redrawing the 2-million-pixel desktop background.

### 4. Real-Time Hardware Clock (RTC)
- Synchronizes time and date directly with the PC hardware CMOS / UEFI Runtime Services `GetTime`.
- Dynamically updates the taskbar clock every minute without UI freezing.

### 5. Built-in Desktop Applications
- **Interactive Shell (`vysh-sh`)**: An interactive command-line terminal supporting real keyboard typing, command history, and utilities:
  - `help` - List available commands
  - `wifi scan` & `wifi connect` - Wireless network discovery and hotspot tethering
  - `wifi status` & `ping` - Network diagnostics
  - `ipconfig` - Network adapter details
  - `whoami` - Student and college credentials
  - `ls` / `dir` - Virtual file directory
  - `time` & `date` - Hardware RTC clock readout
  - `uname -a` - Microkernel architecture info
  - `clear` - Clear terminal screen
- **Scientific Calculator**: Full 4-function mathematical engine with interactive on-screen click buttons and physical number pad support.
- **Wi-Fi Hotspot Center**: 802.11 network management with one-click hotspot connection.
- **Laptop / This PC**: Hardware diagnostics and CPU/RAM resource overview.
- **Settings**: System configuration and workstation preferences.
- **Research Paper Viewer**: Operating system scheduling and microkernel architecture paper.

---

## 💿 Bootable ISO & Virtual Machine Images
You can download the bootable image directly from this repository to test or install Micro-OS:
- 💿 [**Download Bootable UEFI ISO (`Micro-OS.iso`)**](Micro-OS.iso) *(4.48 MB - Bootable on VirtualBox, VMware, QEMU, Rufus, or physical UEFI PCs)*
- 💻 **VirtualBox VDI:** Built automatically using `cargo run --bin builder` to output `kernel.vdi`.

---

## 📁 Repository Structure

```
uefi_kernel/
├── .gitignore                                       # Ignores build artifacts and disk images
├── Cargo.toml                                       # Rust package manifest
├── Cargo.lock                                       # Dependency lockfile
├── README.md                                        # Project documentation
├── Micro-OS.iso                                     # Bootable UEFI Hybrid ISO (4.48 MB)
├── build_iso.py                                     # Automated UEFI ISO builder (El Torito + FAT16)
├── push_to_hf.py                                    # One-click Hugging Face repository uploader
├── 25BTRCL294_Y_Vyshnavi_Linux.pdf                  # Linux Systems Engineering Empirical Report
├── 25BTRCL294_Y.VYSHNAVI_investigation_report.pdf   # OS Investigation & CPU Scheduling Research
├── 25BTRCL294_Y.VYSHNAVI_research_writeup.pdf       # Workload-Aware Scheduling Research Paper
├── 25BTRCL294_Y.VYSHNAVI_lab report.pdf             # Comprehensive OS Laboratory Report
├── docs/                                            # Academic reports folder
│   ├── 25BTRCL294_Y_Vyshnavi_Linux.pdf
│   ├── 25BTRCL294_Y.VYSHNAVI_investigation_report.pdf
│   ├── 25BTRCL294_Y.VYSHNAVI_research_writeup.pdf
│   └── 25BTRCL294_Y.VYSHNAVI_lab report.pdf
└── src/
    ├── main.rs                                      # Complete microkernel, GOP graphics, drivers, & apps
    └── bin/
        └── builder.rs                               # Disk builder: creates FAT16 EFI partition, VDI image, & ISO
```

---

## 📚 Academic Research Papers & Continuous Assessment Reports
All reports submitted by the author (**Yeddula Vyshnavi**, USN: **25BTRCL294**, Section E, JAIN FET Campus) are included directly in the repository:

- 📄 [**25BTRCL294_Y_Vyshnavi_Linux.pdf**](25BTRCL294_Y_Vyshnavi_Linux.pdf)  
  *Investigating the Impact of Workload Characteristics on CPU Scheduling Policy Selection: A Linux Systems Engineering Empirical Study* (CA-5 Report, 27 pages).
- 📄 [**25BTRCL294_Y.VYSHNAVI_investigation_report.pdf**](25BTRCL294_Y.VYSHNAVI_investigation_report.pdf)  
  *Workload-Aware Scheduling: Can Workload Characteristics Be Used to Select a More Suitable CPU Scheduling Strategy?* (CA-4 Investigation Report).
- 📄 [**25BTRCL294_Y.VYSHNAVI_research_writeup.pdf**](25BTRCL294_Y.VYSHNAVI_research_writeup.pdf)  
  *OS Continuous Assessment Extended Research Writeup: Empirical CPU Scheduling Simulation & Policy Selection.*
- 📄 [**25BTRCL294_Y.VYSHNAVI_lab report.pdf**](<25BTRCL294_Y.VYSHNAVI_lab report.pdf>)  
  *Comprehensive Operating Systems Laboratory Report and Practical Implementations (12.4 MB).*
- 📄 [**25BTRCL294_Yeddula_Vyshnavi Design_Debugging.pdf**](<25BTRCL294_Yeddula_Vyshnavi Design_Debugging.pdf>)  
  *Continuous Assessment CA-3: Design & Debugging Analysis Report (409 KB).*

---

## 🛠️ How to Build and Run

### Prerequisites
1. **Rust Toolchain:**
   ```bash
   rustup target add x86_64-unknown-uefi
   ```
2. **Oracle VM VirtualBox:**
   Installed at default location (`C:\Program Files\Oracle\VirtualBox\VBoxManage.exe`).

### Step 1: Compile the UEFI Kernel
```bash
cargo build --target x86_64-unknown-uefi
```
This compiles the kernel binary to `target/x86_64-unknown-uefi/debug/uefi_kernel.efi`.

### Step 2: Build the VirtualBox Disk Image (`kernel.vdi`)
```bash
cargo run --bin builder
```
The builder tool:
1. Creates a 32 MB disk image (`disk.img`).
2. Formats an MBR partition table with an EFI System Partition (FAT16).
3. Copies `uefi_kernel.efi` to `\EFI\BOOT\BOOTX64.EFI`.
4. Automatically calls `VBoxManage convertfromraw` to produce `kernel.vdi`.

### Step 3: Run in Oracle VirtualBox (Using ISO or VDI)
#### Option A: Booting from `Micro-OS.iso`
1. Open **Oracle VM VirtualBox** and create a new VM:
   - **Type:** Other
   - **Version:** Other/Unknown (64-bit)
   - **RAM:** 512 MB or 1024 MB
2. In **Settings -> System**:
   - Check **Enable EFI (special OSes only)**
   - Set **Pointing Device:** PS/2 Mouse (or USB Tablet)
3. In **Settings -> Storage**:
   - Attach `Micro-OS.iso` to the Optical Drive (SATA / IDE Controller).
4. Click **Start** to boot Micro-OS directly from the ISO!

#### Option B: Booting from `kernel.vdi`
1. Follow the steps above, but attach `kernel.vdi` to the SATA Hard Disk controller.
2. Click **Start**.

---

## 🤗 Hugging Face Integration
To push this operating system and the bootable ISO to Hugging Face:
1. Generate a free Access Token with **Write** permissions at: [https://huggingface.co/settings/tokens](https://huggingface.co/settings/tokens)
2. Run:
   ```bash
   py push_to_hf.py --token <YOUR_HF_TOKEN>
   ```
   This automatically creates the repository on Hugging Face and uploads `Micro-OS.iso` with full model card metadata.

---

## ⌨️ Desktop Shortcuts
- **`[T]`**: Open Terminal (`vysh-sh`)
- **`[C]`**: Open Calculator
- **`[W]`**: Open Wi-Fi Center
- **`[L]`**: Open Laptop / This PC
- **`[S]`**: Open Settings
- **`[R]`**: Open Research Paper
- **`[Esc]`**: Close active window
- **`[Tab]`**: Toggle Start Menu
- **`[Arrow Keys]`**: Move mouse pointer (backup to touchpad)
- **`[Space]`**: Click under cursor
