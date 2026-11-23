use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::process::Command;

fn main() {
    println!("==================================================");
    println!("🛠️ BUILDING VIRTUALBOX HARD DISK (kernel.vdi)...");
    println!("==================================================");

    let efi_path = "target/x86_64-unknown-uefi/debug/uefi_kernel.efi";
    let efi_bytes = match fs::read(efi_path) {
        Ok(b) => b,
        Err(_) => {
            println!("❌ Error: Could not find '{}'. Run 'cargo build --target x86_64-unknown-uefi' first!", efi_path);
            return;
        }
    };
    println!("📦 Read kernel binary: {} bytes", efi_bytes.len());

    let img_path = "disk.img";
    let vdi_path = "kernel.vdi";

    let _ = fs::remove_file(img_path);
    let _ = fs::remove_file(vdi_path);

    // Create a 32 MB disk image
    let total_sectors: u32 = 64000;
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(img_path)
        .unwrap();

    file.set_len(total_sectors as u64 * 512).unwrap();

    // 1. Write MBR Partition Table (Sector 0)
    let mut mbr = [0u8; 512];
    // Partition 1 entry at offset 446
    mbr[446] = 0x80; // Active/Bootable
    mbr[450] = 0xEF; // EFI System Partition type
    // Start LBA = 2048
    mbr[454..458].copy_from_slice(&2048u32.to_le_bytes());
    // Sector Count = 60000
    mbr[458..462].copy_from_slice(&60000u32.to_le_bytes());
    mbr[510] = 0x55;
    mbr[511] = 0xAA;
    file.write_all(&mbr).unwrap();

    // 2. Write FAT16 Boot Sector (at LBA 2048)
    file.seek(SeekFrom::Start(2048 * 512)).unwrap();
    let mut bpb = [0u8; 512];
    bpb[0..3].copy_from_slice(&[0xEB, 0x3C, 0x90]);
    bpb[3..11].copy_from_slice(b"MSWIN4.1");
    bpb[11..13].copy_from_slice(&512u16.to_le_bytes()); // Bytes per sector
    bpb[13] = 4; // Sectors per cluster (2048 bytes)
    bpb[14..16].copy_from_slice(&4u16.to_le_bytes()); // Reserved sectors
    bpb[16] = 2; // Number of FATs
    bpb[17..19].copy_from_slice(&512u16.to_le_bytes()); // Root entries
    bpb[19..21].copy_from_slice(&60000u16.to_le_bytes()); // Total sectors
    bpb[21] = 0xF8; // Media: Hard disk
    bpb[22..24].copy_from_slice(&64u16.to_le_bytes()); // Sectors per FAT
    bpb[24..26].copy_from_slice(&63u16.to_le_bytes());
    bpb[26..28].copy_from_slice(&255u16.to_le_bytes());
    bpb[28..32].copy_from_slice(&2048u32.to_le_bytes()); // Hidden sectors
    bpb[36] = 0x80;
    bpb[38] = 0x29;
    bpb[39..43].copy_from_slice(&[0x12, 0x34, 0x56, 0x78]);
    bpb[43..54].copy_from_slice(b"EFI SYSTEM ");
    bpb[54..62].copy_from_slice(b"FAT16   ");
    bpb[510] = 0x55;
    bpb[511] = 0xAA;
    file.write_all(&bpb).unwrap();

    // 3. Write FAT1 and FAT2
    let cluster_count = (efi_bytes.len() + 2047) / 2048;
    let mut fat = vec![0u8; 64 * 512];
    // FAT[0] = Media descriptor, FAT[1] = EOF
    fat[0..4].copy_from_slice(&[0xF8, 0xFF, 0xFF, 0xFF]);
    // Cluster 2: EFI Directory (EOF)
    fat[4..6].copy_from_slice(&0xFFFFu16.to_le_bytes());
    // Cluster 3: BOOT Directory (EOF)
    fat[6..8].copy_from_slice(&0xFFFFu16.to_le_bytes());

    // Cluster 4..N: BOOTX64.EFI file chain
    for i in 0..cluster_count {
        let cluster = 4 + i;
        let next_val: u16 = if i + 1 < cluster_count {
            (cluster + 1) as u16
        } else {
            0xFFFF // End of File
        };
        let offset = cluster * 2;
        fat[offset..offset + 2].copy_from_slice(&next_val.to_le_bytes());
    }

    // Write FAT1 (LBA 2052) and FAT2 (LBA 2116)
    file.seek(SeekFrom::Start((2048 + 4) * 512)).unwrap();
    file.write_all(&fat).unwrap();
    file.seek(SeekFrom::Start((2048 + 4 + 64) * 512)).unwrap();
    file.write_all(&fat).unwrap();

    // 4. Root Directory (LBA 2180): Entry for "EFI" directory
    file.seek(SeekFrom::Start((2048 + 4 + 128) * 512)).unwrap();
    let mut root_dir = [0u8; 512 * 32];
    root_dir[0..11].copy_from_slice(b"EFI        ");
    root_dir[11] = 0x10; // Directory
    root_dir[26..28].copy_from_slice(&2u16.to_le_bytes()); // Cluster 2
    file.write_all(&root_dir).unwrap();

    // 5. Cluster 2 (LBA 2212): "BOOT" Directory inside EFI
    let data_lba = 2048 + 4 + 128 + 32;
    file.seek(SeekFrom::Start(data_lba * 512)).unwrap();
    let mut efi_dir = [0u8; 2048];
    efi_dir[0..11].copy_from_slice(b"BOOT       ");
    efi_dir[11] = 0x10; // Directory
    efi_dir[26..28].copy_from_slice(&3u16.to_le_bytes()); // Cluster 3
    file.write_all(&efi_dir).unwrap();

    // 6. Cluster 3 (LBA 2216): "BOOTX64.EFI" entry inside BOOT
    file.seek(SeekFrom::Start((data_lba + 4) * 512)).unwrap();
    let mut boot_dir = [0u8; 2048];
    boot_dir[0..11].copy_from_slice(b"BOOTX64 EFI");
    boot_dir[11] = 0x20; // Archive / Normal File
    boot_dir[26..28].copy_from_slice(&4u16.to_le_bytes()); // Starts at Cluster 4
    boot_dir[28..32].copy_from_slice(&(efi_bytes.len() as u32).to_le_bytes());
    file.write_all(&boot_dir).unwrap();

    // 7. Cluster 4+ (LBA 2220): The actual BOOTX64.EFI bytes!
    file.seek(SeekFrom::Start((data_lba + 8) * 512)).unwrap();
    file.write_all(&efi_bytes).unwrap();
    file.flush().unwrap();
    drop(file);

    println!("💾 Created standard EFI FAT disk image ('disk.img')");

    // 8. Call VirtualBox VBoxManage to convert disk.img into kernel.vdi!
    let vbox_manage = "C:\\Program Files\\Oracle\\VirtualBox\\VBoxManage.exe";
    println!("🔄 Converting to VirtualBox VDI format...");
    let status = Command::new(vbox_manage)
        .args(["convertfromraw", img_path, vdi_path, "--format", "VDI"])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("==================================================");
            println!("🎉 SUCCESS! 'kernel.vdi' IS READY FOR VIRTUALBOX!");
            println!("==================================================");
        }
        _ => {
            println!("⚠️ Warning: VBoxManage conversion had an issue. Check if VirtualBox is installed.");
        }
    }
}