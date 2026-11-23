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

## 📁 Repository Structure

```
uefi_kernel/
├── .gitignore               # Ignores build artifacts and disk images
├── Cargo.toml               # Rust package manifest
├── Cargo.lock               # Dependency lockfile
├── README.md                # Project documentation
└── src/
    ├── main.rs              # Complete microkernel, GOP graphics, drivers, & apps
    └── bin/
        └── builder.rs       # Disk builder: creates FAT16 EFI partition & VDI image
```

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

### Step 3: Run in Oracle VirtualBox
1. Open **Oracle VM VirtualBox** and create a new VM:
   - **Type:** Other
   - **Version:** Other/Unknown (64-bit)
   - **RAM:** 512 MB or 1024 MB
2. In **Settings -> System**:
   - Check **Enable EFI (special OSes only)**
   - Set **Pointing Device:** PS/2 Mouse
3. In **Settings -> Storage**:
   - Attach `kernel.vdi` to the SATA controller.
4. Click **Start** to launch VyshOS!

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
