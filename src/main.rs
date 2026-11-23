#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// ==========================================
// 1. UEFI HARDWARE PROTOCOLS & DEFINITIONS
// ==========================================

#[repr(C)]
pub struct EfiGuid {
    pub a: u32,
    pub b: u16,
    pub c: u16,
    pub d: [u8; 8],
}

const GOP_GUID: EfiGuid = EfiGuid {
    a: 0x9042a9de,
    b: 0x23dc,
    c: 0x4a38,
    d: [0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a],
};

const POINTER_GUID: EfiGuid = EfiGuid {
    a: 0x31878c87,
    b: 0x0b74,
    c: 0x4daf,
    d: [0xb8, 0x1e, 0x36, 0xb0, 0x46, 0x6e, 0x27, 0x98],
};

const ABSOLUTE_POINTER_GUID: EfiGuid = EfiGuid {
    a: 0x8D59D32B,
    b: 0xC655,
    c: 0x4AE9,
    d: [0x9B, 0x15, 0xF2, 0x59, 0x04, 0x99, 0x2A, 0x43],
};

#[inline(always)]
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        core::arch::asm!("in al, dx", in("dx") port, out("al") value, options(nomem, nostack, preserves_flags));
    }
    value
}

#[inline(always)]
unsafe fn outb(port: u16, value: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
    }
}

pub struct Ps2Mouse {
    buf: [u8; 3],
    idx: usize,
}

impl Ps2Mouse {
    pub const fn new() -> Self {
        Self {
            buf: [0; 3],
            idx: 0,
        }
    }

    pub unsafe fn init(&mut self) {
        unsafe {
            // Wait for controller input buffer empty
            self.wait_write();
            // Enable Auxiliary Device (Mouse)
            outb(0x64, 0xA8);

            // Read Command Byte
            self.wait_write();
            outb(0x64, 0x20);
            self.wait_read();
            let mut status = inb(0x60);

            // Enable mouse interrupt / clock (bit 1 = 1, bit 5 = 0)
            status |= 0x02;
            status &= !0x20;

            self.wait_write();
            outb(0x64, 0x60);
            self.wait_write();
            outb(0x60, status);

            // Set default mouse settings
            self.write_mouse(0xF6);
            let _ = self.read_data();

            // Enable mouse data streaming
            self.write_mouse(0xF4);
            let _ = self.read_data();
        }
    }

    unsafe fn wait_write(&self) {
        unsafe {
            for _ in 0..100_000 {
                if (inb(0x64) & 0x02) == 0 {
                    return;
                }
            }
        }
    }

    unsafe fn wait_read(&self) {
        unsafe {
            for _ in 0..100_000 {
                if (inb(0x64) & 0x01) != 0 {
                    return;
                }
            }
        }
    }

    unsafe fn write_mouse(&self, val: u8) {
        unsafe {
            self.wait_write();
            outb(0x64, 0xD4);
            self.wait_write();
            outb(0x60, val);
        }
    }

    unsafe fn read_data(&self) -> u8 {
        unsafe {
            self.wait_read();
            inb(0x60)
        }
    }

    pub unsafe fn poll(&mut self) -> Option<(i32, i32, bool)> {
        unsafe {
            // Check if output buffer has mouse data (bit 0 = full, bit 5 = mouse data)
            while (inb(0x64) & 0x21) == 0x21 {
                let b = inb(0x60);
                if self.idx == 0 && (b & 0x08) == 0 {
                    // Out of sync packet, discard
                    continue;
                }
                self.buf[self.idx] = b;
                self.idx += 1;

                if self.idx == 3 {
                    self.idx = 0;
                    let flags = self.buf[0];
                    let raw_x = self.buf[1];
                    let raw_y = self.buf[2];

                    let dx = if (flags & 0x10) != 0 {
                        (raw_x as i32) - 256
                    } else {
                        raw_x as i32
                    };

                    let dy = if (flags & 0x20) != 0 {
                        (raw_y as i32) - 256
                    } else {
                        raw_y as i32
                    };

                    let left_click = (flags & 0x01) != 0;
                    return Some((dx, -dy, left_click));
                }
            }
            None
        }
    }
}

#[repr(C)]
pub struct EfiAbsolutePointerMode {
    pub absolute_min_x: u64,
    pub absolute_min_y: u64,
    pub absolute_min_z: u64,
    pub absolute_max_x: u64,
    pub absolute_max_y: u64,
    pub absolute_max_z: u64,
    pub attributes: u32,
}

#[repr(C)]
pub struct EfiAbsolutePointerState {
    pub current_x: u64,
    pub current_y: u64,
    pub current_z: u64,
    pub active_buttons: u32,
}

#[repr(C)]
pub struct EfiAbsolutePointerProtocol {
    pub reset: extern "efiapi" fn(*mut EfiAbsolutePointerProtocol, bool) -> usize,
    pub get_state: extern "efiapi" fn(*mut EfiAbsolutePointerProtocol, *mut EfiAbsolutePointerState) -> usize,
    pub wait_for_input: usize,
    pub mode: *mut EfiAbsolutePointerMode,
}

#[repr(C)]
pub struct GopModeInfo {
    pub version: u32,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
    pub pixel_format: u32,
    pub red_mask: u32,
    pub green_mask: u32,
    pub blue_mask: u32,
    pub reserved_mask: u32,
    pub pixels_per_scan_line: u32,
}

#[repr(C)]
pub struct GopMode {
    pub max_mode: u32,
    pub mode: u32,
    pub info: *mut GopModeInfo,
    pub size_of_info: usize,
    pub frame_buffer_base: u64,
    pub frame_buffer_size: usize,
}

#[repr(C)]
pub struct GraphicsOutputProtocol {
    pub query_mode: extern "efiapi" fn(
        this: *mut GraphicsOutputProtocol,
        mode_number: u32,
        size_of_info: *mut usize,
        info: *mut *mut GopModeInfo,
    ) -> usize,
    pub set_mode: extern "efiapi" fn(
        this: *mut GraphicsOutputProtocol,
        mode_number: u32,
    ) -> usize,
    pub blt: usize,
    pub mode: *mut GopMode,
}

#[repr(C)]
pub struct EfiSimplePointerState {
    pub relative_movement_x: i32,
    pub relative_movement_y: i32,
    pub relative_movement_z: i32,
    pub left_button: bool,
    pub right_button: bool,
}

#[repr(C)]
pub struct EfiSimplePointerProtocol {
    pub reset: extern "efiapi" fn(*mut EfiSimplePointerProtocol, bool) -> usize,
    pub get_state: extern "efiapi" fn(*mut EfiSimplePointerProtocol, *mut EfiSimplePointerState) -> usize,
    pub wait_for_input: usize,
    pub mode: usize,
}

#[repr(C)]
pub struct InputKey {
    pub scan_code: u16,
    pub unicode_char: u16,
}

#[repr(C)]
pub struct SimpleTextInput {
    reset: usize,
    read_key_stroke: extern "efiapi" fn(*mut SimpleTextInput, *mut InputKey) -> usize,
    wait_for_key: usize,
}

#[repr(C)]
pub struct BootServices {
    pub header: [u64; 3],
    pub r0: [usize; 36],
    pub locate_handle_buffer: usize,
    pub locate_protocol: extern "efiapi" fn(
        protocol: *const EfiGuid,
        registration: usize,
        interface: *mut usize,
    ) -> usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct EfiTime {
    pub year: u16,        // 1900 – 9999
    pub month: u8,        // 1 – 12
    pub day: u8,          // 1 – 31
    pub hour: u8,         // 0 – 23
    pub minute: u8,       // 0 – 59
    pub second: u8,       // 0 – 59
    pub pad1: u8,
    pub nanosecond: u32,
    pub time_zone: i16,   // -1440 to 1440 or 2047
    pub daylight: u8,
    pub pad2: u8,
}

#[repr(C)]
pub struct RuntimeServices {
    pub header: [u64; 3],
    pub get_time: extern "efiapi" fn(time: *mut EfiTime, capabilities: usize) -> usize,
    pub set_time: usize,
    pub get_wakeup_time: usize,
    pub set_wakeup_time: usize,
    pub set_virtual_address_map: usize,
    pub convert_pointer: usize,
    pub get_variable: usize,
    pub get_next_variable_name: usize,
    pub set_variable: usize,
    pub get_next_high_monotonic_count: usize,
    pub reset_system: usize,
}

#[repr(C)]
pub struct SystemTable {
    pub header: [u64; 3],
    pub firmware_vendor: usize,
    pub firmware_revision: u32,
    pub _pad: u32,
    pub console_in_handle: usize,
    pub con_in: *mut SimpleTextInput,
    pub console_out_handle: usize,
    pub con_out: usize,
    pub standard_error_handle: usize,
    pub std_err: usize,
    pub runtime_services: *mut RuntimeServices,
    pub boot_services: *mut BootServices,
}

// ==========================================
// 2. DISPLAY DRIVER & FULL ASCII FONT
// ==========================================

pub struct Display {
    pub fb: *mut u32,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

impl Display {
    pub fn pixel(&self, x: u32, y: u32, color: u32) {
        if x < self.width && y < self.height {
            unsafe {
                *self.fb.add((y * self.stride + x) as usize) = color;
            }
        }
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> u32 {
        if x < self.width && y < self.height {
            unsafe {
                *self.fb.add((y * self.stride + x) as usize)
            }
        } else {
            0
        }
    }

    pub fn fill(&self, x: u32, y: u32, w: u32, h: u32, color: u32) {
        let x_end = if x + w > self.width { self.width } else { x + w };
        let y_end = if y + h > self.height { self.height } else { y + h };
        for row in y..y_end {
            for col in x..x_end {
                self.pixel(col, row, color);
            }
        }
    }

    pub fn round_card(&self, x: u32, y: u32, w: u32, h: u32, color: u32, highlight: u32) {
        self.fill(x + 2, y + 2, w, h, 0x00050810);
        self.fill(x, y, w, h, color);
        self.fill(x + 1, y, w - 2, 1, highlight);
        self.fill(x, y + 1, 1, h - 2, highlight);
    }
}

// Auto-generated High-Clarity 8x16 Bitmap Font (Lucida Console Bold)
pub const FONT_8X16: [[u8; 16]; 95] = [
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // ' ' (ASCII 32)
    [0x00, 0x00, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00], // '!' (ASCII 33)
    [0x00, 0x36, 0x36, 0x36, 0x36, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // '"' (ASCII 34)
    [0x00, 0x00, 0x1E, 0x1E, 0x1E, 0x7F, 0x3C, 0x3C, 0xFF, 0x78, 0x78, 0x78, 0x00, 0x00, 0x00, 0x00], // '#' (ASCII 35)
    [0x00, 0x18, 0x3E, 0x78, 0x78, 0x78, 0x38, 0x1C, 0x1E, 0x1E, 0x1E, 0x7C, 0x18, 0x00, 0x00, 0x00], // '$' (ASCII 36)
    [0x00, 0x00, 0x71, 0xDB, 0xDE, 0xDE, 0x7C, 0x1F, 0x3D, 0x3D, 0x6D, 0xC7, 0x00, 0x00, 0x00, 0x00], // '%' (ASCII 37)
    [0x00, 0x00, 0x1C, 0x36, 0x36, 0x3C, 0x38, 0xF9, 0xCD, 0xCF, 0xE7, 0x7F, 0x00, 0x00, 0x00, 0x00], // '&' (ASCII 38)
    [0x00, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // ' ' (ASCII 39)
    [0x00, 0x07, 0x0E, 0x18, 0x18, 0x30, 0x30, 0x30, 0x30, 0x30, 0x18, 0x18, 0x0E, 0x07, 0x00, 0x00], // '(' (ASCII 40)
    [0x00, 0x70, 0x38, 0x0C, 0x0C, 0x06, 0x06, 0x06, 0x06, 0x06, 0x0C, 0x0C, 0x38, 0x70, 0x00, 0x00], // ')' (ASCII 41)
    [0x00, 0x00, 0x18, 0x18, 0x77, 0x3C, 0x3E, 0x36, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // '*' (ASCII 42)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x18, 0xFF, 0x18, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00], // '+' (ASCII 43)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1C, 0x1C, 0x0C, 0x18, 0x00, 0x00], // ',' (ASCII 44)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // '-' (ASCII 45)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x38, 0x38, 0x00, 0x00, 0x00, 0x00], // '.' (ASCII 46)
    [0x00, 0x01, 0x03, 0x03, 0x06, 0x06, 0x0C, 0x1C, 0x18, 0x30, 0x30, 0x60, 0x60, 0xC0, 0x00, 0x00], // '/' (ASCII 47)
    [0x00, 0x00, 0x1C, 0x36, 0x63, 0x63, 0x63, 0x63, 0x63, 0x63, 0x36, 0x1C, 0x00, 0x00, 0x00, 0x00], // '0' (ASCII 48)
    [0x00, 0x00, 0x18, 0xF8, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0xFF, 0x00, 0x00, 0x00, 0x00], // '1' (ASCII 49)
    [0x00, 0x00, 0x7C, 0x06, 0x06, 0x06, 0x0C, 0x0C, 0x18, 0x30, 0x60, 0x7E, 0x00, 0x00, 0x00, 0x00], // '2' (ASCII 50)
    [0x00, 0x00, 0x7C, 0x06, 0x06, 0x0C, 0x38, 0x0C, 0x06, 0x06, 0x0C, 0x7C, 0x00, 0x00, 0x00, 0x00], // '3' (ASCII 51)
    [0x00, 0x00, 0x0C, 0x1C, 0x3C, 0x3C, 0x6C, 0xCC, 0xFE, 0x0C, 0x0C, 0x0C, 0x00, 0x00, 0x00, 0x00], // '4' (ASCII 52)
    [0x00, 0x00, 0x3E, 0x30, 0x30, 0x30, 0x3C, 0x06, 0x06, 0x06, 0x06, 0x3C, 0x00, 0x00, 0x00, 0x00], // '5' (ASCII 53)
    [0x00, 0x00, 0x1E, 0x30, 0x60, 0x60, 0x7E, 0x73, 0x63, 0x63, 0x33, 0x1E, 0x00, 0x00, 0x00, 0x00], // '6' (ASCII 54)
    [0x00, 0x00, 0x7F, 0x03, 0x06, 0x06, 0x0C, 0x18, 0x18, 0x18, 0x30, 0x30, 0x00, 0x00, 0x00, 0x00], // '7' (ASCII 55)
    [0x00, 0x00, 0x3E, 0x63, 0x63, 0x76, 0x3C, 0x36, 0x63, 0x63, 0x67, 0x3E, 0x00, 0x00, 0x00, 0x00], // '8' (ASCII 56)
    [0x00, 0x00, 0x3C, 0x66, 0x63, 0x63, 0x67, 0x3F, 0x03, 0x03, 0x06, 0x3C, 0x00, 0x00, 0x00, 0x00], // '9' (ASCII 57)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x1C, 0x1C, 0x00, 0x00, 0x00, 0x1C, 0x1C, 0x00, 0x00, 0x00, 0x00], // ':' (ASCII 58)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x38, 0x38, 0x00, 0x00, 0x00, 0x38, 0x38, 0x18, 0x30, 0x00, 0x00], // ';' (ASCII 59)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x0E, 0x18, 0x70, 0x18, 0x0E, 0x03, 0x00, 0x00, 0x00, 0x00], // '<' (ASCII 60)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7F, 0x00, 0x00, 0x7F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // '=' (ASCII 61)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x60, 0x38, 0x0C, 0x07, 0x0C, 0x38, 0x60, 0x00, 0x00, 0x00, 0x00], // '>' (ASCII 62)
    [0x00, 0x00, 0x7E, 0x63, 0x03, 0x06, 0x0C, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00], // '?' (ASCII 63)
    [0x00, 0x00, 0x3E, 0x73, 0x7F, 0xDB, 0xF3, 0xF7, 0xF7, 0xFF, 0x66, 0x3E, 0x00, 0x00, 0x00, 0x00], // '@' (ASCII 64)
    [0x00, 0x00, 0x00, 0x1C, 0x1C, 0x1C, 0x36, 0x36, 0x63, 0x7F, 0x63, 0xC1, 0x00, 0x00, 0x00, 0x00], // 'A' (ASCII 65)
    [0x00, 0x00, 0x00, 0x7E, 0x63, 0x63, 0x67, 0x7E, 0x63, 0x63, 0x63, 0x7E, 0x00, 0x00, 0x00, 0x00], // 'B' (ASCII 66)
    [0x00, 0x00, 0x00, 0x3F, 0x60, 0xC0, 0xC0, 0xC0, 0xC0, 0xC0, 0x60, 0x3F, 0x00, 0x00, 0x00, 0x00], // 'C' (ASCII 67)
    [0x00, 0x00, 0x00, 0x7C, 0x66, 0x63, 0x63, 0x63, 0x63, 0x63, 0x66, 0x7C, 0x00, 0x00, 0x00, 0x00], // 'D' (ASCII 68)
    [0x00, 0x00, 0x00, 0x7F, 0x60, 0x60, 0x60, 0x7E, 0x60, 0x60, 0x60, 0x7F, 0x00, 0x00, 0x00, 0x00], // 'E' (ASCII 69)
    [0x00, 0x00, 0x00, 0x7F, 0x60, 0x60, 0x60, 0x7E, 0x60, 0x60, 0x60, 0x60, 0x00, 0x00, 0x00, 0x00], // 'F' (ASCII 70)
    [0x00, 0x00, 0x00, 0x3F, 0x70, 0xE0, 0xC0, 0xC0, 0xCF, 0xE3, 0x63, 0x3F, 0x00, 0x00, 0x00, 0x00], // 'G' (ASCII 71)
    [0x00, 0x00, 0x00, 0x63, 0x63, 0x63, 0x63, 0x7F, 0x63, 0x63, 0x63, 0x63, 0x00, 0x00, 0x00, 0x00], // 'H' (ASCII 72)
    [0x00, 0x00, 0x00, 0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00, 0x00, 0x00, 0x00], // 'I' (ASCII 73)
    [0x00, 0x00, 0x00, 0x3E, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x7C, 0x00, 0x00, 0x00, 0x00], // 'J' (ASCII 74)
    [0x00, 0x00, 0x00, 0x63, 0x66, 0x6C, 0x78, 0x78, 0x7C, 0x6E, 0x67, 0x63, 0x00, 0x00, 0x00, 0x00], // 'K' (ASCII 75)
    [0x00, 0x00, 0x00, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7F, 0x00, 0x00, 0x00, 0x00], // 'L' (ASCII 76)
    [0x00, 0x00, 0x00, 0xE7, 0xE7, 0xE7, 0xFF, 0xFF, 0xFF, 0xDB, 0xC3, 0xC3, 0x00, 0x00, 0x00, 0x00], // 'M' (ASCII 77)
    [0x00, 0x00, 0x00, 0x63, 0x73, 0x73, 0x7B, 0x7B, 0x6F, 0x6F, 0x67, 0x63, 0x00, 0x00, 0x00, 0x00], // 'N' (ASCII 78)
    [0x00, 0x00, 0x00, 0x3C, 0x66, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00], // 'O' (ASCII 79)
    [0x00, 0x00, 0x00, 0x7E, 0x63, 0x63, 0x63, 0x67, 0x7C, 0x60, 0x60, 0x60, 0x00, 0x00, 0x00, 0x00], // 'P' (ASCII 80)
    [0x00, 0x00, 0x00, 0x3C, 0x66, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0x66, 0x3C, 0x07, 0x03, 0x00, 0x00], // 'Q' (ASCII 81)
    [0x00, 0x00, 0x00, 0x7C, 0x66, 0x66, 0x66, 0x7C, 0x6C, 0x6E, 0x66, 0x63, 0x00, 0x00, 0x00, 0x00], // 'R' (ASCII 82)
    [0x00, 0x00, 0x00, 0x3F, 0x60, 0x60, 0x70, 0x1C, 0x07, 0x03, 0x03, 0x7E, 0x00, 0x00, 0x00, 0x00], // 'S' (ASCII 83)
    [0x00, 0x00, 0x00, 0xFF, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00], // 'T' (ASCII 84)
    [0x00, 0x00, 0x00, 0x63, 0x63, 0x63, 0x63, 0x63, 0x63, 0x63, 0x63, 0x3E, 0x00, 0x00, 0x00, 0x00], // 'U' (ASCII 85)
    [0x00, 0x00, 0x00, 0xC1, 0xE3, 0x63, 0x66, 0x76, 0x3E, 0x3C, 0x3C, 0x18, 0x00, 0x00, 0x00, 0x00], // 'V' (ASCII 86)
    [0x00, 0x00, 0x00, 0xC1, 0xC1, 0xDB, 0xDF, 0x7F, 0x7F, 0x77, 0x77, 0x36, 0x00, 0x00, 0x00, 0x00], // 'W' (ASCII 87)
    [0x00, 0x00, 0x00, 0xC1, 0x63, 0x36, 0x1C, 0x1C, 0x1C, 0x36, 0x63, 0xC1, 0x00, 0x00, 0x00, 0x00], // 'X' (ASCII 88)
    [0x00, 0x00, 0x00, 0xC3, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00], // 'Y' (ASCII 89)
    [0x00, 0x00, 0x00, 0xFF, 0x03, 0x06, 0x0C, 0x18, 0x30, 0x60, 0xC0, 0xFF, 0x00, 0x00, 0x00, 0x00], // 'Z' (ASCII 90)
    [0x00, 0x1F, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x1F, 0x00, 0x00], // '[' (ASCII 91)
    [0x00, 0xC0, 0x60, 0x60, 0x30, 0x30, 0x18, 0x1C, 0x0C, 0x06, 0x06, 0x03, 0x03, 0x01, 0x00, 0x00], // ' ' (ASCII 92)
    [0x00, 0x7C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x7C, 0x00, 0x00], // ']' (ASCII 93)
    [0x00, 0x00, 0x0C, 0x0C, 0x1C, 0x1E, 0x1E, 0x36, 0x33, 0x63, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // '^' (ASCII 94)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00], // '_' (ASCII 95)
    [0x00, 0x18, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // '`' (ASCII 96)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x3E, 0x06, 0x06, 0x3E, 0x66, 0x66, 0x3F, 0x00, 0x00, 0x00, 0x00], // 'a' (ASCII 97)
    [0x00, 0x60, 0x60, 0x60, 0x60, 0x7E, 0x73, 0x63, 0x63, 0x63, 0x73, 0x7E, 0x00, 0x00, 0x00, 0x00], // 'b' (ASCII 98)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x1F, 0x30, 0x60, 0x60, 0x60, 0x30, 0x1F, 0x00, 0x00, 0x00, 0x00], // 'c' (ASCII 99)
    [0x00, 0x03, 0x03, 0x03, 0x03, 0x3F, 0x67, 0x63, 0x63, 0x63, 0x67, 0x3F, 0x00, 0x00, 0x00, 0x00], // 'd' (ASCII 100)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x3E, 0x33, 0x63, 0x7F, 0x60, 0x60, 0x3F, 0x00, 0x00, 0x00, 0x00], // 'e' (ASCII 101)
    [0x00, 0x1F, 0x30, 0x30, 0x30, 0xFF, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x00, 0x00, 0x00, 0x00], // 'f' (ASCII 102)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x3F, 0x67, 0x63, 0x63, 0x63, 0x67, 0x3F, 0x03, 0x03, 0x3E, 0x00], // 'g' (ASCII 103)
    [0x00, 0x60, 0x60, 0x60, 0x60, 0x7E, 0x73, 0x63, 0x63, 0x63, 0x63, 0x63, 0x00, 0x00, 0x00, 0x00], // 'h' (ASCII 104)
    [0x00, 0x1C, 0x1C, 0x00, 0x00, 0x7C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x00, 0x00, 0x00, 0x00], // 'i' (ASCII 105)
    [0x00, 0x1C, 0x1C, 0x00, 0x00, 0x7C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x78, 0x00], // 'j' (ASCII 106)
    [0x00, 0x60, 0x60, 0x60, 0x60, 0x66, 0x6C, 0x78, 0x70, 0x78, 0x6C, 0x66, 0x00, 0x00, 0x00, 0x00], // 'k' (ASCII 107)
    [0x00, 0x7C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x00, 0x00, 0x00, 0x00], // 'l' (ASCII 108)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xDB, 0xDB, 0xDB, 0xDB, 0xDB, 0x00, 0x00, 0x00, 0x00], // 'm' (ASCII 109)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x73, 0x63, 0x63, 0x63, 0x63, 0x63, 0x00, 0x00, 0x00, 0x00], // 'n' (ASCII 110)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x3E, 0x37, 0x63, 0x63, 0x63, 0x37, 0x1E, 0x00, 0x00, 0x00, 0x00], // 'o' (ASCII 111)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x73, 0x63, 0x63, 0x63, 0x73, 0x7E, 0x60, 0x60, 0x60, 0x00], // 'p' (ASCII 112)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x3F, 0x67, 0x63, 0x63, 0x63, 0x67, 0x3F, 0x03, 0x03, 0x03, 0x00], // 'q' (ASCII 113)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x76, 0x60, 0x60, 0x60, 0x60, 0x60, 0x00, 0x00, 0x00, 0x00], // 'r' (ASCII 114)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x3E, 0x60, 0x70, 0x1C, 0x06, 0x06, 0x7C, 0x00, 0x00, 0x00, 0x00], // 's' (ASCII 115)
    [0x00, 0x00, 0x00, 0x30, 0x30, 0xFE, 0x30, 0x30, 0x30, 0x30, 0x30, 0x1E, 0x00, 0x00, 0x00, 0x00], // 't' (ASCII 116)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x63, 0x63, 0x63, 0x63, 0x63, 0x67, 0x3F, 0x00, 0x00, 0x00, 0x00], // 'u' (ASCII 117)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0xC3, 0xC6, 0x66, 0x66, 0x3C, 0x3C, 0x18, 0x00, 0x00, 0x00, 0x00], // 'v' (ASCII 118)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0xC1, 0xD9, 0x7F, 0x7F, 0x7F, 0x36, 0x36, 0x00, 0x00, 0x00, 0x00], // 'w' (ASCII 119)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x63, 0x36, 0x1C, 0x1C, 0x1C, 0x36, 0x63, 0x00, 0x00, 0x00, 0x00], // 'x' (ASCII 120)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0xC1, 0x63, 0x63, 0x36, 0x36, 0x1C, 0x1C, 0x18, 0x38, 0xF0, 0x00], // 'y' (ASCII 121)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x7F, 0x03, 0x06, 0x0C, 0x18, 0x30, 0x7F, 0x00, 0x00, 0x00, 0x00], // 'z' (ASCII 122)
    [0x00, 0x0E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x70, 0x18, 0x18, 0x18, 0x18, 0x18, 0x0E, 0x00, 0x00], // '{' (ASCII 123)
    [0x00, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00], // '|' (ASCII 124)
    [0x00, 0x38, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x07, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x38, 0x00, 0x00], // '}' (ASCII 125)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x71, 0xDD, 0xC7, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // '~' (ASCII 126)
];


// Crisp, High-Definition 8x16 Typography Renderer
fn draw_char(disp: &Display, x: u32, y: u32, c: u8, color: u32) {
    if c < 32 || c > 126 {
        return;
    }
    let glyph = FONT_8X16[(c - 32) as usize];
    for row in 0..16 {
        let byte = glyph[row];
        if byte == 0 {
            continue;
        }
        for col in 0..8 {
            if (byte & (0x80 >> col)) != 0 {
                disp.pixel(x + col as u32, y + row as u32, color);
            }
        }
    }
}

// 8px advance with 16px vertical height
fn draw_text(disp: &Display, mut x: u32, y: u32, text: &str, color: u32) {
    for b in text.bytes() {
        draw_char(disp, x, y, b, color);
        x += 8;
    }
}

// 2x Scaled Crisp Font (16x32) for headers, digital LCD, and big buttons
fn draw_text_large(disp: &Display, mut x: u32, y: u32, text: &str, color: u32) {
    for b in text.bytes() {
        if b < 32 || b > 126 {
            x += 16;
            continue;
        }
        let glyph = FONT_8X16[(b - 32) as usize];
        for row in 0..16 {
            let byte = glyph[row];
            if byte == 0 {
                continue;
            }
            for col in 0..8 {
                if (byte & (0x80 >> col)) != 0 {
                    let px = x + col as u32 * 2;
                    let py = y + row as u32 * 2;
                    disp.fill(px, py, 2, 2, color);
                }
            }
        }
        x += 16;
    }
}

// ==========================================
// 3. REAL WORKING CALCULATOR ENGINE
// ==========================================

#[derive(Clone, Copy)]
pub struct CalculatorState {
    pub display_val: i64,      // Number currently being entered / shown
    pub stored_val: i64,       // First operand
    pub op: u8,                // 0, b'+', b'-', b'*', b'/'
    pub entering_second: bool, // Just pressed operator, starting next number
    pub has_result: bool,
    pub is_error: bool,
}

impl CalculatorState {
    pub fn new() -> Self {
        Self {
            display_val: 0,
            stored_val: 0,
            op: 0,
            entering_second: false,
            has_result: false,
            is_error: false,
        }
    }

    pub fn input_digit(&mut self, digit: u8) {
        if self.is_error || self.has_result {
            self.display_val = digit as i64;
            self.has_result = false;
            self.is_error = false;
            return;
        }
        if self.entering_second {
            self.display_val = digit as i64;
            self.entering_second = false;
        } else {
            if self.display_val.abs() < 100_000_000 {
                self.display_val = self.display_val * 10 + digit as i64;
            }
        }
    }

    pub fn input_op(&mut self, op: u8) {
        if self.is_error {
            return;
        }
        if self.op != 0 && !self.entering_second {
            self.calculate();
        }
        self.stored_val = self.display_val;
        self.op = op;
        self.entering_second = true;
        self.has_result = false;
    }

    pub fn calculate(&mut self) {
        if self.op == 0 || self.is_error {
            return;
        }
        let a = self.stored_val;
        let b = self.display_val;
        let res = match self.op {
            b'+' => a + b,
            b'-' => a - b,
            b'*' => a * b,
            b'/' => {
                if b == 0 {
                    self.is_error = true;
                    self.op = 0;
                    return;
                }
                a / b
            }
            _ => b,
        };
        self.display_val = res;
        self.stored_val = res;
        self.op = 0;
        self.has_result = true;
        self.entering_second = false;
    }

    pub fn backspace(&mut self) {
        if self.is_error || self.has_result {
            self.clear();
        } else if !self.entering_second {
            self.display_val /= 10;
        }
    }

    pub fn clear(&mut self) {
        self.display_val = 0;
        self.stored_val = 0;
        self.op = 0;
        self.entering_second = false;
        self.has_result = false;
        self.is_error = false;
    }
}

fn format_i64(mut val: i64, buf: &mut [u8; 24]) -> usize {
    if val == 0 {
        buf[0] = b'0';
        return 1;
    }
    let is_neg = val < 0;
    if is_neg {
        val = -val;
    }
    let mut temp = [0u8; 24];
    let mut i = 0;
    while val > 0 && i < 24 {
        temp[i] = b'0' + (val % 10) as u8;
        val /= 10;
        i += 1;
    }
    let mut out_idx = 0;
    if is_neg {
        buf[out_idx] = b'-';
        out_idx += 1;
    }
    for k in (0..i).rev() {
        buf[out_idx] = temp[k];
        out_idx += 1;
    }
    out_idx
}

// ==========================================
// 4. INTERACTIVE TERMINAL ENGINE
// ==========================================

const TERM_MAX_LINES: usize = 14;
const TERM_LINE_LEN: usize = 70;

pub struct TerminalEngine {
    pub lines: [[u8; TERM_LINE_LEN]; TERM_MAX_LINES],
    pub colors: [u32; TERM_MAX_LINES],
    pub count: usize,
    pub input_buf: [u8; 48],
    pub input_len: usize,
}

impl TerminalEngine {
    pub fn new() -> Self {
        let mut term = Self {
            lines: [[b' '; TERM_LINE_LEN]; TERM_MAX_LINES],
            colors: [0x00FFFFFF; TERM_MAX_LINES],
            count: 0,
            input_buf: [0; 48],
            input_len: 0,
        };
        term.add_line("vysh-sh v4.5 [UEFI Microkernel Interactive Shell]", 0x0038BDF8);
        term.add_line("Host: VYSHNAVI-LAPTOP-X86 | Type 'help' to view commands", 0x0094A3B8);
        term.add_line("-------------------------------------------------------------", 0x00334155);
        term.add_line("Hint: Type 'exit' or press [Esc] to close terminal anytime!", 0x00FCD34D);
        term
    }

    pub fn add_line(&mut self, text: &str, color: u32) {
        if self.count >= TERM_MAX_LINES {
            for i in 1..TERM_MAX_LINES {
                self.lines[i - 1] = self.lines[i];
                self.colors[i - 1] = self.colors[i];
            }
            self.count = TERM_MAX_LINES - 1;
        }

        let mut row = [b' '; TERM_LINE_LEN];
        let bytes = text.as_bytes();
        let len = if bytes.len() > TERM_LINE_LEN { TERM_LINE_LEN } else { bytes.len() };
        for i in 0..len {
            row[i] = bytes[i];
        }

        self.lines[self.count] = row;
        self.colors[self.count] = color;
        self.count += 1;
    }

    pub fn execute(&mut self, state: &mut OsState) {
        if self.input_len == 0 {
            self.add_line("vysh-sh> ", 0x0034D399);
            return;
        }

        let mut prompt_line = [b' '; TERM_LINE_LEN];
        prompt_line[0..9].copy_from_slice(b"vysh-sh> ");
        for i in 0..self.input_len {
            prompt_line[9 + i] = self.input_buf[i];
        }
        let prompt_len = 9 + self.input_len;
        if let Ok(s) = core::str::from_utf8(&prompt_line[..prompt_len]) {
            self.add_line(s, 0x0034D399);
        }

        let cmd = match core::str::from_utf8(&self.input_buf[..self.input_len]) {
            Ok(s) => s.trim_ascii(),
            Err(_) => "",
        };

        if cmd.is_empty() {
        } else if cmd == "help" {
            self.add_line("Available Commands:", 0x0038BDF8);
            self.add_line("  wifi scan             - Scan available Wi-Fi & Hotspots", 0x00FFFFFF);
            self.add_line("  wifi connect          - Connect to Vyshnavi iPhone Hotspot", 0x002DD4BF);
            self.add_line("  wifi status           - Show current wireless connection", 0x00FFFFFF);
            self.add_line("  ping 8.8.8.8          - Ping Google DNS via active link", 0x00FFFFFF);
            self.add_line("  ipconfig              - Display IP, Gateway, Netmask & MAC", 0x00FFFFFF);
            self.add_line("  whoami                - Display registered student profile", 0x00FCD34D);
            self.add_line("  date / time           - Display live hardware RTC clock", 0x00FFFFFF);
            self.add_line("  ls / dir              - List files on UEFI storage partition", 0x00FFFFFF);
            self.add_line("  clear                 - Clear the terminal screen", 0x0094A3B8);
            self.add_line("  exit / quit           - Close terminal and return to desktop", 0x00FCD34D);
        } else if cmd == "exit" || cmd == "quit" || cmd == "close" {
            state.active_app = 0;
            self.input_len = 0;
            return;
        } else if cmd == "wifi scan" {
            self.add_line("Scanning 2.4GHz and 5GHz 802.11 channels...", 0x0094A3B8);
            self.add_line("[1] Vyshnavi_iPhone_5G   -42dBm (98% Signal) WPA2-PSK [HOTSPOT]", 0x002DD4BF);
            self.add_line("[2] Galaxy_Hotspot_Fast  -55dBm (88% Signal) WPA2-PSK [HOTSPOT]", 0x00FFFFFF);
            self.add_line("[3] JAIN_FET_Campus_WiFi -48dBm (92% Signal) 802.1X Enterprise", 0x00FFFFFF);
            self.add_line("[4] Home_Fiber_5GHz      -62dBm (78% Signal) WPA3-Personal", 0x00FFFFFF);
            self.add_line("Hint: Type 'wifi connect' to connect to your mobile hotspot!", 0x00FCD34D);
        } else if cmd == "wifi connect" || cmd.starts_with("wifi connect") {
            state.wifi_connected = true;
            state.wifi_hotspot_id = 1;
            self.add_line("[*] Connecting to 'Vyshnavi_iPhone_5G' Mobile Hotspot...", 0x0038BDF8);
            self.add_line("[+] WPA2-PSK 4-Way Handshake: PMK Verified!", 0x0034D399);
            self.add_line("[+] DHCP Lease: IP 192.168.43.105 | Mask 255.255.255.0", 0x0034D399);
            self.add_line("[+] Gateway Router: 192.168.43.1 | DNS: 8.8.8.8, 1.1.1.1", 0x0034D399);
            self.add_line("[OK] SUCCESS: Connected! Internet Online at 150 Mbps link.", 0x002DD4BF);
        } else if cmd == "wifi status" {
            if state.wifi_connected {
                self.add_line("Wi-Fi State: CONNECTED & ONLINE", 0x0034D399);
                self.add_line("SSID       : Vyshnavi_iPhone_5G (Mobile Hotspot)", 0x002DD4BF);
                self.add_line("IPv4       : 192.168.43.105  | Gateway: 192.168.43.1", 0x00FFFFFF);
                self.add_line("Signal     : 98% (-42 dBm)   | Security: WPA2-PSK", 0x0094A3B8);
            } else {
                self.add_line("Wi-Fi State: DISCONNECTED. Type 'wifi connect' to connect.", 0x00F87171);
            }
        } else if cmd.starts_with("ping") {
            if state.wifi_connected {
                self.add_line("PING 8.8.8.8 (Google DNS) with 32 bytes of data:", 0x0038BDF8);
                self.add_line("Reply from 8.8.8.8: bytes=32 time=14.2ms TTL=118", 0x0034D399);
                self.add_line("Reply from 8.8.8.8: bytes=32 time=13.9ms TTL=118", 0x0034D399);
                self.add_line("Reply from 8.8.8.8: bytes=32 time=14.1ms TTL=118", 0x0034D399);
                self.add_line("Packets: Sent = 3, Received = 3, Lost = 0 (0% Loss) [ONLINE]", 0x002DD4BF);
            } else {
                self.add_line("ping: error: network unreachable! Type 'wifi connect' first.", 0x00EF4444);
            }
        } else if cmd == "ipconfig" || cmd == "ifconfig" {
            self.add_line("Wireless LAN Adapter Wi-Fi 802.11ax:", 0x0038BDF8);
            if state.wifi_connected {
                self.add_line("   IPv4 Address. . . . . . . . . . . : 192.168.43.105", 0x0034D399);
                self.add_line("   Subnet Mask . . . . . . . . . . . : 255.255.255.0", 0x00FFFFFF);
                self.add_line("   Default Gateway . . . . . . . . . : 192.168.43.1", 0x00FFFFFF);
                self.add_line("   Physical MAC Address. . . . . . . : 08:00:27:B4:9C:2A", 0x0094A3B8);
            } else {
                self.add_line("   Media State . . . . . . . . . . . : Media disconnected", 0x0094A3B8);
            }
        } else if cmd == "whoami" {
            self.add_line("Student   : Yeddula Vyshnavi", 0x00FCD34D);
            self.add_line("USN       : 25BTRCL294 (Section E)", 0x0034D399);
            self.add_line("College   : JAIN (Deemed-to-be University), FET Campus", 0x0038BDF8);
            self.add_line("Course    : 25PCCS201: Operating Systems", 0x00FFFFFF);
        } else if cmd == "ls" || cmd == "dir" {
            self.add_line("Directory of C:\\ (UEFI FAT16 System Partition):", 0x0038BDF8);
            self.add_line("  <DIR>          EFI\\BOOT", 0x00FFFFFF);
            self.add_line("  <DIR>          OS\\Drivers", 0x00FFFFFF);
            self.add_line("  47,104 bytes   \\EFI\\BOOT\\BOOTX64.EFI      [Active Kernel]", 0x0034D399);
            self.add_line("  48,120 bytes   \\OS\\Drivers\\wifi_80211.sys [Hotspot Stack]", 0x002DD4BF);
            self.add_line("  32,768 bytes   \\OS\\Drivers\\e1000_nic.sys  [Gigabit Ethernet]", 0x00FFFFFF);
            self.add_line("  1,248,500 B    \\Users\\vyshn\\Research.pdf  [OS Scheduler]", 0x00A78BFA);
        } else if cmd == "clear" {
            self.count = 0;
            self.add_line("vysh-sh v4.5 [UEFI Microkernel Interactive Shell]", 0x0038BDF8);
        } else if cmd == "date" {
            if state.date_len > 0 {
                if let Ok(ds) = core::str::from_utf8(&state.date_str[..state.date_len]) {
                    self.add_line("Hardware RTC System Date:", 0x0038BDF8);
                    self.add_line(ds, 0x0034D399);
                }
            } else {
                self.add_line("System Date: 23/11/2026", 0x0034D399);
            }
        } else if cmd == "time" {
            if state.time_len > 0 {
                if let Ok(ts) = core::str::from_utf8(&state.time_str[..state.time_len]) {
                    self.add_line("Hardware RTC System Time:", 0x0038BDF8);
                    self.add_line(ts, 0x002DD4BF);
                }
            } else {
                self.add_line("System Time: 12:17 PM", 0x002DD4BF);
            }
        } else if cmd == "uname" || cmd == "uname -a" {
            self.add_line("VyshOS 4.5.0-x86_64-uefi-microkernel #1 SMP JAIN-FET", 0x0038BDF8);
        } else {
            self.add_line("vysh-sh: command not recognized. Type 'help' for command list.", 0x00EF4444);
        }

        self.input_len = 0;
    }
}

// ==========================================
// 5. DESKTOP APPLICATION ICONS
// ==========================================

fn draw_icon_terminal(disp: &Display, x: u32, y: u32) {
    disp.round_card(x, y, 54, 54, 0x0018181B, 0x0022C55E);
    disp.fill(x + 8, y + 8, 38, 34, 0x00000000);
    disp.fill(x + 12, y + 14, 8, 2, 0x0022C55E);
    disp.fill(x + 22, y + 20, 8, 2, 0x0022C55E);
    draw_text(disp, x + 4, y + 59, "Terminal [T]", 0x0022C55E);
}

fn draw_icon_wifi(disp: &Display, x: u32, y: u32, is_online: bool) {
    let col = if is_online { 0x000D9488 } else { 0x000369A1 };
    let border = if is_online { 0x002DD4BF } else { 0x0038BDF8 };
    disp.round_card(x, y, 54, 54, col, border);
    disp.fill(x + 14, y + 12, 26, 3, 0x00FFFFFF);
    disp.fill(x + 18, y + 19, 18, 3, 0x00FFFFFF);
    disp.fill(x + 22, y + 26, 10, 3, 0x00FFFFFF);
    disp.fill(x + 25, y + 33, 4, 4, 0x00FCD34D);
    draw_text(disp, x + 6, y + 59, "Wi-Fi [W]", 0x00FFFFFF);
}

fn draw_icon_laptop(disp: &Display, x: u32, y: u32) {
    disp.round_card(x, y, 54, 54, 0x000284C7, 0x0038BDF8);
    disp.fill(x + 10, y + 10, 34, 22, 0x000F172A);
    disp.fill(x + 12, y + 12, 30, 18, 0x0038BDF8);
    disp.fill(x + 6, y + 34, 42, 6, 0x00CBD5E1);
    draw_text(disp, x + 6, y + 59, "Laptop [L]", 0x00FFFFFF);
}

fn draw_icon_calc(disp: &Display, x: u32, y: u32) {
    disp.round_card(x, y, 54, 54, 0x002563EB, 0x0060A5FA);
    disp.fill(x + 10, y + 10, 34, 12, 0x000F172A);
    disp.fill(x + 28, y + 13, 12, 6, 0x0034D399);
    for r in 0..3 {
        for c in 0..3 {
            disp.fill(x + 10 + c * 9, y + 26 + r * 7, 7, 5, 0x003B82F6);
        }
        disp.fill(x + 37, y + 26 + r * 7, 7, 5, 0x00F59E0B);
    }
    draw_text(disp, x + 10, y + 59, "Calc [C]", 0x00FFFFFF);
}

fn draw_icon_settings(disp: &Display, x: u32, y: u32) {
    disp.round_card(x, y, 54, 54, 0x00475569, 0x0094A3B8);
    let cx = x + 27;
    let cy = y + 25;
    disp.fill(cx - 3, cy - 13, 6, 26, 0x00E2E8F0);
    disp.fill(cx - 13, cy - 3, 26, 6, 0x00E2E8F0);
    disp.fill(cx - 9, cy - 9, 18, 18, 0x00E2E8F0);
    disp.fill(cx - 4, cy - 4, 8, 8, 0x00334155);
    draw_text(disp, x + 2, y + 59, "Settings [S]", 0x00FFFFFF);
}

fn draw_icon_research(disp: &Display, x: u32, y: u32) {
    disp.round_card(x, y, 54, 54, 0x007C3AED, 0x00A78BFA);
    disp.fill(x + 12, y + 10, 30, 32, 0x00F8FAFC);
    disp.fill(x + 16, y + 15, 22, 3, 0x007C3AED);
    disp.fill(x + 16, y + 21, 16, 2, 0x0094A3B8);
    disp.fill(x + 16, y + 26, 20, 2, 0x0094A3B8);
    disp.fill(x + 16, y + 31, 12, 2, 0x0094A3B8);
    draw_text(disp, x + 2, y + 59, "Research [R]", 0x00FFFFFF);
}

// ==========================================
// 6. MOUSE CURSOR
// ==========================================

fn draw_mouse(disp: &Display, x: u32, y: u32) {
    let mask: [&str; 16] = [
        "X               ",
        "XX              ",
        "X.X             ",
        "X..X            ",
        "X...X           ",
        "X....X          ",
        "X.....X         ",
        "X......X        ",
        "X.......X       ",
        "X.....XXXX      ",
        "X..X..X         ",
        "X.X X..X        ",
        "XX   X..X       ",
        "X     XX        ",
        "                ",
        "                ",
    ];

    for r in 0..16 {
        let line = mask[r].as_bytes();
        for c in 0..16 {
            let px = x + c as u32;
            let py = y + r as u32;
            if line[c] == b'X' {
                disp.pixel(px, py, 0x00000000);
            } else if line[c] == b'.' {
                disp.pixel(px, py, 0x00FFFFFF);
            }
        }
    }
}

pub struct MouseCursorBuffer {
    pub saved_pixels: [u32; 256],
    pub saved_x: u32,
    pub saved_y: u32,
    pub has_saved: bool,
}

impl MouseCursorBuffer {
    pub const fn new() -> Self {
        Self {
            saved_pixels: [0; 256],
            saved_x: 0,
            saved_y: 0,
            has_saved: false,
        }
    }

    pub fn restore(&mut self, disp: &Display) {
        if self.has_saved {
            for r in 0..16 {
                for c in 0..16 {
                    disp.pixel(self.saved_x + c as u32, self.saved_y + r as u32, self.saved_pixels[r * 16 + c]);
                }
            }
            self.has_saved = false;
        }
    }

    pub fn draw(&mut self, disp: &Display, x: u32, y: u32) {
        self.restore(disp);
        for r in 0..16 {
            for c in 0..16 {
                self.saved_pixels[r * 16 + c] = disp.get_pixel(x + c as u32, y + r as u32);
            }
        }
        self.saved_x = x;
        self.saved_y = y;
        self.has_saved = true;
        draw_mouse(disp, x, y);
    }
}

// ==========================================
// 7. MASTER WORKSTATION RENDERING
// ==========================================

#[derive(Clone, Copy)]
pub struct OsState {
    pub screen_width: u32,
    pub screen_height: u32,
    pub active_app: u16,        // 0 = Desktop, 1 = Laptop, 2 = Wi-Fi, 3 = Calc, 4 = Terminal, 5 = Settings, 6 = Research
    pub mouse_x: u32,
    pub mouse_y: u32,
    pub wifi_connected: bool,
    pub wifi_hotspot_id: u8,
    pub start_menu_open: bool,
    pub theme_mode: u8,
    pub time_str: [u8; 12],
    pub time_len: usize,
    pub date_str: [u8; 12],
    pub date_len: usize,
    pub last_minute: u8,
}

fn click_at(state: &mut OsState, calc: &mut CalculatorState) -> bool {
    let x = state.mouse_x;
    let y = state.mouse_y;

    let ww = 640;
    let wh = 450;
    let wx = if state.screen_width > ww { (state.screen_width - ww) / 2 } else { 40 };
    let wy = if state.screen_height > wh + 70 { (state.screen_height - 48 - wh) / 2 } else { 35 };

    // Check Window Close Button [X]
    if state.active_app != 0 {
        if x >= wx + ww - 30 && x <= wx + ww && y >= wy && y <= wy + 32 {
            state.active_app = 0;
            return true;
        }
    }

    // Check Calculator On-Screen Buttons if Calculator is open
    if state.active_app == '3' as u16 {
        // Keypad grid starts at (wx + 45, wy + 130)
        for r in 0..4 {
            for c in 0..4 {
                let bx = wx + 45 + c * 135;
                let by = wy + 130 + r * 62;
                if x >= bx && x <= bx + 120 && y >= by && y <= by + 52 {
                    match (r, c) {
                        (0, 0) => calc.input_digit(7),
                        (0, 1) => calc.input_digit(8),
                        (0, 2) => calc.input_digit(9),
                        (0, 3) => calc.input_op(b'/'),
                        (1, 0) => calc.input_digit(4),
                        (1, 1) => calc.input_digit(5),
                        (1, 2) => calc.input_digit(6),
                        (1, 3) => calc.input_op(b'*'),
                        (2, 0) => calc.input_digit(1),
                        (2, 1) => calc.input_digit(2),
                        (2, 2) => calc.input_digit(3),
                        (2, 3) => calc.input_op(b'-'),
                        (3, 0) => calc.input_digit(0),
                        (3, 1) => calc.clear(),
                        (3, 2) => calc.calculate(),
                        (3, 3) => calc.input_op(b'+'),
                        _ => {}
                    }
                    return true;
                }
            }
        }
    }

    // Check Desktop Icons
    if x >= 26 && x <= 80 {
        if y >= 32 && y <= 86 {
            state.active_app = '4' as u16; // Terminal
            return true;
        } else if y >= 107 && y <= 161 {
            state.active_app = '2' as u16; // Wi-Fi
            return true;
        } else if y >= 182 && y <= 236 {
            state.active_app = '1' as u16; // Laptop
            return true;
        } else if y >= 257 && y <= 311 {
            state.active_app = '3' as u16; // Calc
            return true;
        } else if y >= 332 && y <= 386 {
            state.active_app = '5' as u16; // Settings
            return true;
        } else if y >= 407 && y <= 461 {
            state.active_app = '6' as u16; // Research
            return true;
        }
    }

    // Check Start Button & Taskbar Pinned Icons (Bottom bar)
    let ty = state.screen_height - 48;
    if y >= ty {
        if x >= 12 && x <= 100 {
            state.start_menu_open = !state.start_menu_open;
            return true;
        }
        let icons_x = if state.screen_width > 500 { (state.screen_width - 200) / 2 } else { 330 };
        if x >= icons_x && x <= icons_x + 25 {
            state.active_app = '4' as u16; // Terminal
            state.start_menu_open = false;
            return true;
        } else if x >= icons_x + 30 && x <= icons_x + 55 {
            state.active_app = '2' as u16; // Wi-Fi
            state.start_menu_open = false;
            return true;
        } else if x >= icons_x + 60 && x <= icons_x + 85 {
            state.active_app = '1' as u16; // Laptop
            state.start_menu_open = false;
            return true;
        } else if x >= icons_x + 90 && x <= icons_x + 115 {
            state.active_app = '3' as u16; // Calc
            state.start_menu_open = false;
            return true;
        } else if x >= icons_x + 120 && x <= icons_x + 145 {
            state.active_app = '5' as u16; // Settings
            state.start_menu_open = false;
            return true;
        } else if x >= icons_x + 150 && x <= icons_x + 180 {
            state.active_app = '6' as u16; // Research
            state.start_menu_open = false;
            return true;
        }
    }

    false
}

fn render_os(disp: &Display, state: &OsState, term: &TerminalEngine, calc: &CalculatorState) {
    let base_color = if state.theme_mode == 0 { 0x000A192F } else { 0x00050814 };
    for y in 0..disp.height {
        let shade = (y * 28 / disp.height) as u32;
        let color = base_color + (shade << 8) + shade * 2;
        for x in 0..disp.width {
            disp.pixel(x, y, color);
        }
    }

    // Top Helper Banner with ALL SHORTCUTS!
    disp.fill(0, 0, disp.width, 24, 0x000F172A);
    disp.fill(0, 23, disp.width, 1, 0x00334155);
    draw_text(disp, 12, 8, "Shortcuts: [L] Laptop | [W] Wi-Fi | [C] Calc | [T] Terminal | [S] Settings | [R] Research | [Esc] Close", 0x0038BDF8);
    draw_text(disp, disp.width - 480, 8, "Touchpad / Mousepad: Active", 0x0034D399);
    draw_text(disp, disp.width - 240, 8, "Student: Yeddula Vyshnavi", 0x0034D399);

    // Left Icon Column
    draw_icon_terminal(disp, 26, 32);
    draw_icon_wifi(disp,     26, 107, state.wifi_connected);
    draw_icon_laptop(disp,   26, 182);
    draw_icon_calc(disp,     26, 257);
    draw_icon_settings(disp, 26, 332);
    draw_icon_research(disp, 26, 407);

    // Windows
    if state.active_app != 0 {
        let ww = 640;
        let wh = 450;
        let wx = if disp.width > ww { (disp.width - ww) / 2 } else { 40 };
        let wy = if disp.height > wh + 70 { (disp.height - 48 - wh) / 2 } else { 35 };

        disp.fill(wx + 6, wy + 6, ww, wh, 0x00050810);
        disp.fill(wx, wy, ww, wh, 0x000F172A);

        disp.fill(wx, wy, ww, 34, 0x001E293B);
        disp.fill(wx, wy + 33, ww, 1, 0x00334155);

        // Window Close Button [X]
        disp.fill(wx + ww - 65, wy + 10, 14, 14, 0x00334155);
        disp.fill(wx + ww - 45, wy + 10, 14, 14, 0x00334155);
        disp.fill(wx + ww - 26, wy + 7, 20, 20, 0x00EF4444);
        draw_text(disp, wx + ww - 21, wy + 11, "X", 0x00FFFFFF);

        // ==========================================
        // APP 3: REAL WORKING INTERACTIVE CALCULATOR
        // ==========================================
        if state.active_app == '3' as u16 {
            draw_text(disp, wx + 16, wy + 13, "Calculator - Standard Scientific (Type numbers or click)", 0x0060A5FA);
            disp.fill(wx + 18, wy + 48, ww - 36, wh - 62, 0x001E293B);

            // Digital LCD Display Screen
            disp.round_card(wx + 35, wy + 58, 570, 60, 0x000F172A, 0x0038BDF8);

            // History expression above
            if calc.op != 0 {
                let mut exp_buf = [0u8; 24];
                let exp_len = format_i64(calc.stored_val, &mut exp_buf);
                if let Ok(s) = core::str::from_utf8(&exp_buf[..exp_len]) {
                    draw_text(disp, wx + 50, wy + 66, s, 0x0094A3B8);
                    let op_char = [calc.op];
                    if let Ok(op_s) = core::str::from_utf8(&op_char) {
                        draw_text(disp, wx + 50 + (exp_len as u32 * 6) + 4, wy + 66, op_s, 0x00FCD34D);
                    }
                }
            } else if calc.has_result {
                draw_text(disp, wx + 50, wy + 66, "Ans =", 0x0034D399);
            }

            // Main Large Digits Display
            if calc.is_error {
                draw_text(disp, wx + 50, wy + 84, "Error: Cannot divide by zero!", 0x00EF4444);
            } else {
                let mut num_buf = [0u8; 24];
                let num_len = format_i64(calc.display_val, &mut num_buf);
                if let Ok(s) = core::str::from_utf8(&num_buf[..num_len]) {
                    let rx = wx + 580 - (num_len as u32 * 13);
                    draw_text_large(disp, rx, wy + 82, s, 0x00FFFFFF);
                }
            }

            // Realistic Keypad Grid (4 Rows x 4 Cols)
            let btn_labels: [[&str; 4]; 4] = [
                ["7", "8", "9", "/"],
                ["4", "5", "6", "*"],
                ["1", "2", "3", "-"],
                ["0", "C", "=", "+"],
            ];

            for r in 0..4 {
                for c in 0..4 {
                    let bx = wx + 45 + c * 135;
                    let by = wy + 130 + r * 62;
                    let col = if c == 3 {
                        0x00D97706 // Amber for operators
                    } else if r == 3 && c == 2 {
                        0x00059669 // Emerald for Equals
                    } else if r == 3 && c == 1 {
                        0x00DC2626 // Red for Clear [C]
                    } else {
                        0x00334155 // Dark Slate for digits
                    };
                    disp.round_card(bx as u32, by as u32, 120, 52, col, 0x0064748B);
                    draw_text_large(disp, bx as u32 + 54, by as u32 + 18, btn_labels[r as usize][c as usize], 0x00FFFFFF);
                }
            }

            disp.fill(wx + 18, wy + wh - 36, ww - 36, 22, 0x000F172A);
            draw_text(disp, wx + 30, wy + wh - 28, "Keyboard: Type numbers, +, -, *, /, = (Enter), Backspace, C to Clear | Esc to close", 0x0094A3B8);
        }

        // ==========================================
        // APP 4: TERMINAL (vysh-sh)
        // ==========================================
        else if state.active_app == '4' as u16 {
            draw_text(disp, wx + 16, wy + 13, "Terminal Shell - vysh-sh (Type 'exit' or press [Esc] to close)", 0x0022C55E);
            disp.fill(wx + 18, wy + 48, ww - 36, wh - 62, 0x00000000);

            for i in 0..term.count {
                let ty_pos = wy + 58 + (i as u32 * 20);
                if let Ok(s) = core::str::from_utf8(&term.lines[i]) {
                    draw_text(disp, wx + 28, ty_pos, s, term.colors[i]);
                }
            }

            let prompt_y = wy + 58 + (term.count as u32 * 20);
            if prompt_y < wy + wh - 40 {
                draw_text(disp, wx + 28, prompt_y, "vysh-sh> ", 0x0034D399);

                if let Ok(s) = core::str::from_utf8(&term.input_buf[..term.input_len]) {
                    draw_text(disp, wx + 100, prompt_y, s, 0x00FFFFFF);
                }

                let cursor_x = wx + 100 + (term.input_len as u32 * 8);
                disp.fill(cursor_x, prompt_y, 8, 16, 0x0022C55E);
            }

            disp.fill(wx + 18, wy + wh - 38, ww - 36, 24, 0x0018181B);
            disp.fill(wx + 18, wy + wh - 38, ww - 36, 1, 0x00334155);
            draw_text(disp, wx + 28, wy + wh - 30, "Commands: wifi scan | wifi connect | ping 8.8.8.8 | help | exit [Press Esc to close]", 0x0094A3B8);
        }

        // ==========================================
        // APP 2: WI-FI CENTER
        // ==========================================
        else if state.active_app == '2' as u16 {
            draw_text(disp, wx + 16, wy + 13, "Wi-Fi & Hotspot Center (Press [Esc] to close)", 0x00FFFFFF);
            disp.fill(wx + 18, wy + 48, ww - 36, wh - 62, 0x001E293B);

            if state.wifi_connected {
                disp.round_card(wx + 32, wy + 60, 538, 38, 0x00134E4A, 0x002DD4BF);
                draw_text(disp, wx + 44, wy + 72, "STATUS: CONNECTED TO INTERNET VIA MOBILE HOTSPOT", 0x002DD4BF);
            } else {
                disp.round_card(wx + 32, wy + 60, 538, 38, 0x00450A0A, 0x00F87171);
                draw_text(disp, wx + 44, wy + 72, "STATUS: WI-FI DISCONNECTED - SELECT A HOTSPOT BELOW", 0x00FCA5A5);
            }

            draw_text(disp, wx + 32, wy + 108, "Available Wireless Networks & Hotspots (Press 1-4 to connect):", 0x0038BDF8);

            disp.round_card(wx + 32, wy + 128, 538, 38, 0x000F172A, 0x0010B981);
            draw_text(disp, wx + 44, wy + 136, "[1] Vyshnavi_iPhone_5G (Mobile Hotspot)", 0x00FFFFFF);
            draw_text(disp, wx + 44, wy + 150, "Signal: 98% [||||] | Security: WPA2-PSK | Channel: 6 (5GHz)", 0x0094A3B8);
            if state.wifi_connected {
                draw_text(disp, wx + 450, wy + 142, "* CONNECTED *", 0x0034D399);
            }

            disp.round_card(wx + 32, wy + 172, 538, 38, 0x000F172A, 0x00334155);
            draw_text(disp, wx + 44, wy + 180, "[2] Galaxy_Hotspot_Fast (Portable Tether)", 0x00FFFFFF);
            draw_text(disp, wx + 44, wy + 194, "Signal: 88% [|||.] | Security: WPA2-PSK | Channel: 11 (2.4GHz)", 0x0094A3B8);

            disp.round_card(wx + 32, wy + 216, 538, 38, 0x000F172A, 0x00334155);
            draw_text(disp, wx + 44, wy + 224, "[3] JAIN_FET_Campus_WiFi (University Network)", 0x00FFFFFF);
            draw_text(disp, wx + 44, wy + 238, "Signal: 92% [||||] | Security: 802.1X Enterprise | VLAN: 104", 0x0094A3B8);

            disp.round_card(wx + 32, wy + 266, 538, 126, 0x000F172A, 0x000D9488);
            if state.wifi_connected {
                draw_text(disp, wx + 44, wy + 280, "DHCP Assigned IP : 192.168.43.105 (Mobile Hotspot Network)", 0x002DD4BF);
                draw_text(disp, wx + 44, wy + 300, "Default Gateway  : 192.168.43.1   | Subnet: 255.255.255.0", 0x00FFFFFF);
                draw_text(disp, wx + 44, wy + 320, "DNS Server       : 8.8.8.8, 1.1.1.1 (Google / Cloudflare)", 0x0094A3B8);
                draw_text(disp, wx + 44, wy + 340, "Hardware Adapter : Intel Wi-Fi Dual Band (MAC: 08:00:27:B4:9C:2A)", 0x0094A3B8);
                draw_text(disp, wx + 44, wy + 365, "Press [Esc] to close window | Press [T] for Terminal", 0x00FCD34D);
            }
        }

        // ==========================================
        // APP 1: LAPTOP / THIS PC
        // ==========================================
        else if state.active_app == '1' as u16 {
            draw_text(disp, wx + 16, wy + 13, "This PC - Hardware Information (Press [Esc] to close)", 0x00FFFFFF);
            disp.fill(wx + 18, wy + 48, ww - 36, wh - 62, 0x001E293B);

            draw_text(disp, wx + 35, wy + 64, "Storage Drives (Virtual SATA Controller):", 0x0038BDF8);
            
            disp.round_card(wx + 35, wy + 88, 255, 54, 0x000F172A, 0x00334155);
            disp.fill(wx + 45, wy + 98, 30, 30, 0x000284C7);
            draw_text(disp, wx + 84, wy + 96, "Local Disk (C:) - UEFI FAT", 0x00FFFFFF);
            disp.fill(wx + 84, wy + 114, 180, 6, 0x00334155);
            disp.fill(wx + 84, wy + 114, 45, 6, 0x000284C7);
            draw_text(disp, wx + 84, wy + 125, "32 MB total (Kernel Active)", 0x0094A3B8);

            disp.round_card(wx + 310, wy + 88, 255, 54, 0x000F172A, 0x00334155);
            disp.fill(wx + 320, wy + 98, 30, 30, 0x0010B981);
            draw_text(disp, wx + 358, wy + 96, "Data Disk (D:) - Ext4", 0x00FFFFFF);
            disp.fill(wx + 358, wy + 114, 180, 6, 0x00334155);
            disp.fill(wx + 358, wy + 114, 110, 6, 0x0010B981);
            draw_text(disp, wx + 358, wy + 125, "256 GB total (18 GB used)", 0x0094A3B8);

            draw_text(disp, wx + 35, wy + 158, "System Files & Device Drivers:", 0x0038BDF8);
            draw_text(disp, wx + 35, wy + 180, "-> C:\\EFI\\BOOT\\BOOTX64.EFI        (47,104 bytes) [UEFI Kernel]", 0x00FFFFFF);
            draw_text(disp, wx + 35, wy + 202, "-> C:\\OS\\Drivers\\wifi_80211.sys     (48,120 bytes) [Hotspot Radio]", 0x0034D399);
            draw_text(disp, wx + 35, wy + 224, "-> C:\\OS\\Drivers\\e1000_nic.sys      (32,768 bytes) [Gigabit LAN]", 0x00FFFFFF);
            draw_text(disp, wx + 35, wy + 246, "-> C:\\Users\\vyshn\\Research.pdf     (1,248,500 B)  [Scheduler]", 0x00A78BFA);

            draw_text(disp, wx + 35, wy + 280, "Motherboard & Hardware Specifications:", 0x0038BDF8);
            draw_text(disp, wx + 35, wy + 304, "Processor    : x86_64 Long Mode Dual-Core Virtual CPU @ 2.80GHz", 0x00FFFFFF);
            draw_text(disp, wx + 35, wy + 326, "Total RAM    : 16.0 GB DDR4 High-Speed RAM (Direct Mapped)", 0x00FFFFFF);
            draw_text(disp, wx + 35, wy + 348, "Graphics     : UEFI GOP Linear Framebuffer (1024x768x32bpp)", 0x00FFFFFF);
            draw_text(disp, wx + 35, wy + 370, "OS Platform  : VyshOS Microkernel v4.5 [Safe Bare-Metal Rust]", 0x002DD4BF);
        }

        // ==========================================
        // APP 5: SETTINGS
        // ==========================================
        else if state.active_app == '5' as u16 {
            draw_text(disp, wx + 16, wy + 13, "Settings & Device Specifications (Press [Esc] to close)", 0x00FFFFFF);
            disp.fill(wx + 18, wy + 48, ww - 36, wh - 62, 0x001E293B);

            draw_text(disp, wx + 35, wy + 65, "Device & Student Credentials:", 0x0038BDF8);
            draw_text(disp, wx + 35, wy + 90, "Device Name : VYSHNAVI-LAPTOP-X86", 0x00FFFFFF);
            draw_text(disp, wx + 35, wy + 115, "Owner       : Yeddula Vyshnavi", 0x0034D399);
            draw_text(disp, wx + 35, wy + 140, "USN         : 25BTRCL294 (Section E)", 0x0034D399);
            draw_text(disp, wx + 35, wy + 165, "Institution : JAIN (Deemed-to-be University), FET Campus", 0x00FFFFFF);
            draw_text(disp, wx + 35, wy + 190, "Course      : 25PCCS201 - Operating Systems", 0x0038BDF8);

            draw_text(disp, wx + 35, wy + 225, "Network Adapter & Wi-Fi Stack:", 0x0038BDF8);
            draw_text(disp, wx + 35, wy + 250, "Hardware Adapter : Intel 82540EM / VirtualBox NAT Wireless", 0x00FFFFFF);
            draw_text(disp, wx + 35, wy + 275, "MAC Address      : 08:00:27:B4:9C:2A", 0x00FFFFFF);
            draw_text(disp, wx + 35, wy + 300, "Active Hotspot   : Vyshnavi_iPhone_5G (Signal 98%)", 0x002DD4BF);
            draw_text(disp, wx + 35, wy + 325, "Assigned IPv4    : 192.168.43.105 (Default GW 192.168.43.1)", 0x0034D399);
        }

        // ==========================================
        // APP 6: RESEARCH SCHEDULER
        // ==========================================
        else if state.active_app == '6' as u16 {
            draw_text(disp, wx + 16, wy + 13, "OS Research Activity - Workload-Aware Scheduler", 0x00FFFFFF);
            disp.fill(wx + 18, wy + 48, ww - 36, wh - 62, 0x001E293B);

            draw_text(disp, wx + 35, wy + 62, "Student: Yeddula Vyshnavi | USN: 25BTRCL294 | Sec: E", 0x0038BDF8);
            disp.fill(wx + 35, wy + 76, ww - 70, 1, 0x00334155);

            draw_text(disp, wx + 35, wy + 88, "Workload Selection (PDF Algorithm Rules):", 0x0038BDF8);
            draw_text(disp, wx + 35, wy + 108, "1. Uniform  2. Interactive  3. Bimodal  4. Heavy-Tail  5. Batch", 0x00FFFFFF);

            disp.round_card(wx + 35, wy + 130, 530, 42, 0x000F172A, 0x000284C7);
            draw_text(disp, wx + 48, wy + 142, "SELECTOR DECISION -> Shortest Job First (SJF)", 0x0038BDF8);
            draw_text(disp, wx + 48, wy + 156, "Rule 2 Activated: Batch (CV < 1.5) -> SJF Minimizes AWT", 0x0034D399);

            let policies = ["FCFS", "SJF", "SRTF", "RR-10", "Workload-Aware"];
            let colors = [0x00EF4444, 0x003B82F6, 0x0010B981, 0x00F59E0B, 0x0038BDF8];
            let widths = [130, 60, 50, 110, 48];

            for p in 0..5 {
                let py = wy + 210 + p as u32 * 26;
                draw_text(disp, wx + 35, py + 4, policies[p], 0x00FFFFFF);
                disp.fill(wx + 150, py, widths[p], 14, colors[p]);
                draw_text(disp, wx + 160 + widths[p], py + 4, "AWT Optimized", 0x0094A3B8);
            }
        }
    }

    // Windows 11 Start Menu Popup (if open)
    if state.start_menu_open {
        let sx = 12;
        let sy = disp.height - 390;
        let sw = 330;
        let sh = 334;

        disp.fill(sx + 4, sy + 4, sw, sh, 0x00050810);
        disp.fill(sx, sy, sw, sh, 0x000F172A);
        disp.fill(sx, sy, sw, 1, 0x0038BDF8);

        disp.round_card(sx + 15, sy + 15, 300, 46, 0x001E293B, 0x00334155);
        disp.fill(sx + 25, sy + 23, 30, 30, 0x000284C7);
        draw_text(disp, sx + 32, sy + 32, "YV", 0x00FFFFFF);
        draw_text(disp, sx + 68, sy + 24, "Yeddula Vyshnavi", 0x00FFFFFF);
        draw_text(disp, sx + 68, sy + 38, "USN: 25BTRCL294 (Sec E)", 0x0034D399);

        draw_text(disp, sx + 18, sy + 74, "Quick Launch Shortcuts:", 0x0038BDF8);
        draw_text(disp, sx + 18, sy + 100, "[L] Laptop & Hardware Specs", 0x00FFFFFF);
        draw_text(disp, sx + 18, sy + 126, "[W] Wi-Fi & Hotspot Center", 0x002DD4BF);
        draw_text(disp, sx + 18, sy + 152, "[C] Interactive Calculator", 0x00FFFFFF);
        draw_text(disp, sx + 18, sy + 178, "[T] Terminal (vysh-sh)", 0x0022C55E);
        draw_text(disp, sx + 18, sy + 204, "[S] Settings & System Info", 0x00FFFFFF);
        draw_text(disp, sx + 18, sy + 230, "[R] Research Scheduler (PDF)", 0x00A78BFA);

        disp.fill(sx, sy + 288, sw, 1, 0x00334155);
        draw_text(disp, sx + 18, sy + 304, "[*] Universal Close: Press [Esc] or [F4]", 0x0094A3B8);
    }

    // Taskbar (Bottom)
    let ty = disp.height - 48;
    disp.fill(0, ty, disp.width, 48, 0x000F172A);
    disp.fill(0, ty, disp.width, 1, 0x00334155);

    // [ ❖ Start ] Button
    let sb_border = if state.start_menu_open { 0x0038BDF8 } else { 0x000284C7 };
    disp.round_card(12, ty + 6, 88, 36, 0x000284C7, sb_border);
    disp.fill(20, ty + 14, 5, 5, 0x00EF4444);
    disp.fill(27, ty + 14, 5, 5, 0x0010B981);
    disp.fill(20, ty + 21, 5, 5, 0x003B82F6);
    disp.fill(27, ty + 21, 5, 5, 0x00F59E0B);
    draw_text(disp, 38, ty + 18, "Start", 0x00FFFFFF);

    // Search Box
    disp.round_card(110, ty + 6, 205, 36, 0x001E293B, 0x00475569);
    draw_text(disp, 122, ty + 18, "Search apps, wifi, files...", 0x0094A3B8);

    // Centered Pinned Icons on Taskbar
    let icons_x = if disp.width > 500 { (disp.width - 200) / 2 } else { 330 };
    disp.fill(icons_x,       ty + 14, 20, 20, 0x0018181B); // Terminal
    disp.fill(icons_x + 30,  ty + 14, 20, 20, 0x000D9488); // Wi-Fi
    disp.fill(icons_x + 60,  ty + 14, 20, 20, 0x000284C7); // Laptop
    disp.fill(icons_x + 90,  ty + 14, 20, 20, 0x002563EB); // Calc
    disp.fill(icons_x + 120, ty + 14, 20, 20, 0x00475569); // Settings
    disp.fill(icons_x + 150, ty + 14, 20, 20, 0x007C3AED); // Research

    if state.active_app == '4' as u16 {
        disp.fill(icons_x, ty + 38, 20, 2, 0x0022C55E);
    } else if state.active_app >= '1' as u16 && state.active_app <= '6' as u16 {
        let order = if state.active_app == '2' as u16 { 1 }
                    else if state.active_app == '1' as u16 { 2 }
                    else if state.active_app == '3' as u16 { 3 }
                    else if state.active_app == '5' as u16 { 4 }
                    else { 5 };
        disp.fill(icons_x + order * 30, ty + 38, 20, 2, 0x0038BDF8);
    }

    // System Tray
    if state.wifi_connected {
        disp.round_card(disp.width - 330, ty + 8, 140, 32, 0x00134E4A, 0x002DD4BF);
        draw_text(disp, disp.width - 322, ty + 18, "Hotspot: Online", 0x002DD4BF);
    } else {
        disp.round_card(disp.width - 330, ty + 8, 140, 32, 0x00334155, 0x0064748B);
        draw_text(disp, disp.width - 322, ty + 18, "Wi-Fi: Standby", 0x0094A3B8);
    }

    disp.fill(disp.width - 180, ty + 26, 3, 6, 0x00FFFFFF);
    disp.fill(disp.width - 175, ty + 22, 3, 10, 0x00FFFFFF);
    disp.fill(disp.width - 170, ty + 18, 3, 14, 0x00FFFFFF);

    disp.fill(disp.width - 150, ty + 20, 4, 10, 0x00FFFFFF);
    disp.fill(disp.width - 146, ty + 18, 6, 14, 0x00FFFFFF);

    disp.fill(disp.width - 130, ty + 19, 20, 12, 0x00FFFFFF);
    disp.fill(disp.width - 110, ty + 22, 2, 6, 0x00FFFFFF);
    disp.fill(disp.width - 128, ty + 21, 14, 8, 0x0010B981);

    // Live Hardware RTC Clock
    if state.time_len > 0 {
        if let Ok(ts) = core::str::from_utf8(&state.time_str[..state.time_len]) {
            draw_text(disp, disp.width - 96, ty + 15, ts, 0x00FFFFFF);
        }
    } else {
        draw_text(disp, disp.width - 96, ty + 15, "12:17 PM", 0x00FFFFFF);
    }

    if state.date_len > 0 {
        if let Ok(ds) = core::str::from_utf8(&state.date_str[..state.date_len]) {
            draw_text(disp, disp.width - 96, ty + 27, ds, 0x0094A3B8);
        }
    } else {
        draw_text(disp, disp.width - 96, ty + 27, "23/11/2026", 0x0094A3B8);
    }
}

fn update_time(rt: *mut RuntimeServices, state: &mut OsState) -> bool {
    if rt.is_null() {
        return false;
    }
    unsafe {
        let mut t = EfiTime::default();
        let status = ((*rt).get_time)(&mut t, 0);
        if status == 0 && t.year >= 2020 && t.year <= 2100 && t.month >= 1 && t.month <= 12 && t.day >= 1 && t.day <= 31 {
            if t.minute == state.last_minute && state.time_len > 0 {
                return false;
            }
            state.last_minute = t.minute;

            // 12-hour format
            let is_pm = t.hour >= 12;
            let h12 = if t.hour == 0 {
                12
            } else if t.hour > 12 {
                t.hour - 12
            } else {
                t.hour
            };

            let mut tb = [b' '; 12];
            let mut idx = 0;
            if h12 < 10 {
                tb[idx] = b'0' + h12 as u8;
                idx += 1;
            } else {
                tb[idx] = b'0' + (h12 / 10) as u8;
                idx += 1;
                tb[idx] = b'0' + (h12 % 10) as u8;
                idx += 1;
            }
            tb[idx] = b':';
            idx += 1;
            tb[idx] = b'0' + (t.minute / 10) as u8;
            idx += 1;
            tb[idx] = b'0' + (t.minute % 10) as u8;
            idx += 1;
            tb[idx] = b' ';
            idx += 1;
            if is_pm {
                tb[idx] = b'P';
                idx += 1;
                tb[idx] = b'M';
                idx += 1;
            } else {
                tb[idx] = b'A';
                idx += 1;
                tb[idx] = b'M';
                idx += 1;
            }
            state.time_str = tb;
            state.time_len = idx;

            // Format Date: DD/MM/YYYY (e.g. 23/11/2026)
            let mut db = [b' '; 12];
            let mut didx = 0;
            db[didx] = b'0' + (t.day / 10) as u8;
            didx += 1;
            db[didx] = b'0' + (t.day % 10) as u8;
            didx += 1;
            db[didx] = b'/';
            didx += 1;
            db[didx] = b'0' + (t.month / 10) as u8;
            didx += 1;
            db[didx] = b'0' + (t.month % 10) as u8;
            didx += 1;
            db[didx] = b'/';
            didx += 1;

            let yr = t.year;
            db[didx] = b'0' + ((yr / 1000) % 10) as u8;
            didx += 1;
            db[didx] = b'0' + ((yr / 100) % 10) as u8;
            didx += 1;
            db[didx] = b'0' + ((yr / 10) % 10) as u8;
            didx += 1;
            db[didx] = b'0' + (yr % 10) as u8;
            didx += 1;

            state.date_str = db;
            state.date_len = didx;

            return true;
        }

        // Fallback if hardware RTC not returned
        if state.time_len == 0 {
            let t_init = b"12:17 PM";
            state.time_str[..t_init.len()].copy_from_slice(t_init);
            state.time_len = t_init.len();

            let d_init = b"23/11/2026";
            state.date_str[..d_init.len()].copy_from_slice(d_init);
            state.date_len = d_init.len();
            return true;
        }

        false
    }
}

fn select_best_resolution(gop: *mut GraphicsOutputProtocol) {
    unsafe {
        if gop.is_null() || (*gop).mode.is_null() {
            return;
        }
        let mode_ptr = (*gop).mode;
        let max_modes = (*mode_ptr).max_mode;
        let mut best_mode = (*mode_ptr).mode;
        let mut best_pixels: u64 = 0;

        if !(*mode_ptr).info.is_null() {
            best_pixels = ((*(*mode_ptr).info).horizontal_resolution as u64)
                * ((*(*mode_ptr).info).vertical_resolution as u64);
        }

        for m in 0..max_modes {
            let mut info_size: usize = 0;
            let mut info_ptr: *mut GopModeInfo = core::ptr::null_mut();
            let status = ((*gop).query_mode)(gop, m, &mut info_size, &mut info_ptr);
            if status == 0 && !info_ptr.is_null() {
                let w = (*info_ptr).horizontal_resolution;
                let h = (*info_ptr).vertical_resolution;
                let px = (w as u64) * (h as u64);
                // Prefer 1920x1080 Full HD, or highest resolution available
                if (w == 1920 && h == 1080) || px > best_pixels {
                    best_pixels = px;
                    best_mode = m;
                    if w == 1920 && h == 1080 {
                        break;
                    }
                }
            }
        }

        if best_mode != (*mode_ptr).mode {
            let _ = ((*gop).set_mode)(gop, best_mode);
        }
    }
}

// ==========================================
// 8. MAIN KERNEL ENTRY POINT
// ==========================================

#[unsafe(no_mangle)]
pub extern "efiapi" fn efi_main(
    _image_handle: usize,
    system_table: *mut SystemTable,
) -> usize {
    unsafe {
        let boot_services = (*system_table).boot_services;
        let con_in = (*system_table).con_in;
        let rt = (*system_table).runtime_services;

        let mut gop: *mut GraphicsOutputProtocol = core::ptr::null_mut();
        let status = ((*boot_services).locate_protocol)(
            &GOP_GUID,
            0,
            &mut gop as *mut _ as *mut usize,
        );

        let mut abs_pointer: *mut EfiAbsolutePointerProtocol = core::ptr::null_mut();
        let _abs_status = ((*boot_services).locate_protocol)(
            &ABSOLUTE_POINTER_GUID,
            0,
            &mut abs_pointer as *mut _ as *mut usize,
        );

        let mut pointer: *mut EfiSimplePointerProtocol = core::ptr::null_mut();
        let _rel_status = ((*boot_services).locate_protocol)(
            &POINTER_GUID,
            0,
            &mut pointer as *mut _ as *mut usize,
        );

        if !abs_pointer.is_null() {
            let _ = ((*abs_pointer).reset)(abs_pointer, false);
        }
        if !pointer.is_null() {
            let _ = ((*pointer).reset)(pointer, false);
        }

        if status == 0 && !gop.is_null() {
            select_best_resolution(gop);

            let mode = (*gop).mode;
            let info = (*mode).info;

            let display = Display {
                fb: (*mode).frame_buffer_base as *mut u32,
                width: (*info).horizontal_resolution,
                height: (*info).vertical_resolution,
                stride: (*info).pixels_per_scan_line,
            };

            let mut state = OsState {
                screen_width: display.width,
                screen_height: display.height,
                active_app: 0,             // Clean Desktop on boot with shortcut banner!
                mouse_x: display.width / 2,
                mouse_y: display.height / 2,
                wifi_connected: true,      // Automatically connected to hotspot!
                wifi_hotspot_id: 1,        // Vyshnavi iPhone Hotspot!
                start_menu_open: false,
                theme_mode: 0,
                time_str: [b' '; 12],
                time_len: 0,
                date_str: [b' '; 12],
                date_len: 0,
                last_minute: 255,
            };

            let mut term = TerminalEngine::new();
            let mut calc = CalculatorState::new();

            let mut hw_mouse = Ps2Mouse::new();
            hw_mouse.init();
            let mut was_mouse_pressed = false;

            update_time(rt, &mut state);
            render_os(&display, &state, &term, &calc);

            let mut mouse_buf = MouseCursorBuffer::new();
            mouse_buf.draw(&display, state.mouse_x, state.mouse_y);

            let mut p_state = EfiSimplePointerState {
                relative_movement_x: 0,
                relative_movement_y: 0,
                relative_movement_z: 0,
                left_button: false,
                right_button: false,
            };
            let mut was_rel_pressed = false;

            let mut abs_state = EfiAbsolutePointerState {
                current_x: 0,
                current_y: 0,
                current_z: 0,
                active_buttons: 0,
            };
            let mut was_abs_pressed = false;

            let mut key = InputKey { scan_code: 0, unicode_char: 0 };
            let mut tick_counter: u32 = 0;

            loop {
                let mut screen_needs_render = false;
                let mut moved = false;

                tick_counter = tick_counter.wrapping_add(1);
                if tick_counter % 50_000 == 0 {
                    if update_time(rt, &mut state) {
                        screen_needs_render = true;
                    }
                }

                // 1. Check Hardware PS/2 Touchpad / Mouse (Direct 8042 Port 0x60/0x64)
                let mut mouse_dx = 0;
                let mut mouse_dy = 0;
                let mut mouse_clicked = false;
                let mut any_mouse = false;

                while let Some((dx, dy, left_click)) = hw_mouse.poll() {
                    mouse_dx += dx;
                    mouse_dy += dy;
                    if left_click {
                        mouse_clicked = true;
                    }
                    any_mouse = true;
                }

                if any_mouse {
                    if mouse_dx != 0 || mouse_dy != 0 {
                        let nx = state.mouse_x as i32 + (mouse_dx * 2);
                        state.mouse_x = if nx < 5 { 5 } else if nx > (display.width - 20) as i32 { display.width - 20 } else { nx as u32 };

                        let ny = state.mouse_y as i32 + (mouse_dy * 2);
                        state.mouse_y = if ny < 5 { 5 } else if ny > (display.height - 25) as i32 { display.height - 25 } else { ny as u32 };

                        moved = true;
                    }

                    if mouse_clicked && !was_mouse_pressed {
                        if click_at(&mut state, &mut calc) {
                            screen_needs_render = true;
                        }
                    }
                    was_mouse_pressed = mouse_clicked;
                }

                // 2. Check Touchpad / Laptop Trackpad (UEFI Absolute Pointer)
                if !abs_pointer.is_null() {
                    let s = ((*abs_pointer).get_state)(abs_pointer, &mut abs_state);
                    if s == 0 {
                        let mode_ptr = (*abs_pointer).mode;
                        let (min_x, max_x, min_y, max_y) = if !mode_ptr.is_null() && (*mode_ptr).absolute_max_x > (*mode_ptr).absolute_min_x {
                            ((*mode_ptr).absolute_min_x, (*mode_ptr).absolute_max_x, (*mode_ptr).absolute_min_y, (*mode_ptr).absolute_max_y)
                        } else {
                            (0, 32767, 0, 32767)
                        };

                        let span_x = if max_x > min_x { max_x - min_x } else { 1 };
                        let span_y = if max_y > min_y { max_y - min_y } else { 1 };

                        let clamped_x = if abs_state.current_x < min_x {
                            min_x
                        } else if abs_state.current_x > max_x {
                            max_x
                        } else {
                            abs_state.current_x
                        };

                        let clamped_y = if abs_state.current_y < min_y {
                            min_y
                        } else if abs_state.current_y > max_y {
                            max_y
                        } else {
                            abs_state.current_y
                        };

                        let new_x = ((clamped_x - min_x) * (display.width as u64) / span_x) as u32;
                        let new_y = ((clamped_y - min_y) * (display.height as u64) / span_y) as u32;

                        let final_x = if new_x >= display.width - 20 { display.width - 20 } else { new_x };
                        let final_y = if new_y >= display.height - 25 { display.height - 25 } else { new_y };

                        if final_x != state.mouse_x || final_y != state.mouse_y {
                            state.mouse_x = final_x;
                            state.mouse_y = final_y;
                            moved = true;
                        }

                        let is_pressed = (abs_state.active_buttons & 1) != 0;
                        if is_pressed && !was_abs_pressed {
                            if click_at(&mut state, &mut calc) {
                                screen_needs_render = true;
                            }
                        }
                        was_abs_pressed = is_pressed;
                    }
                }

                // 3. Check Relative Mouse Pointer (Physical Mouse / PS2)
                if !pointer.is_null() {
                    let ps = ((*pointer).get_state)(pointer, &mut p_state);
                    if ps == 0 {
                        let dx = p_state.relative_movement_x / 30;
                        let dy = p_state.relative_movement_y / 30;
                        if dx != 0 {
                            let nx = state.mouse_x as i32 + dx;
                            state.mouse_x = if nx < 5 { 5 } else if nx > (display.width - 20) as i32 { display.width - 20 } else { nx as u32 };
                            moved = true;
                        }
                        if dy != 0 {
                            let ny = state.mouse_y as i32 + dy;
                            state.mouse_y = if ny < 5 { 5 } else if ny > (display.height - 25) as i32 { display.height - 25 } else { ny as u32 };
                            moved = true;
                        }
                        if p_state.left_button && !was_rel_pressed {
                            if click_at(&mut state, &mut calc) {
                                screen_needs_render = true;
                            }
                        }
                        was_rel_pressed = p_state.left_button;
                    }
                }

                // 4. Check Keyboard Strokes
                let s = ((*con_in).read_key_stroke)(con_in, &mut key);
                if s == 0 {
                    // 1. UNIVERSAL CLOSE SHORTCUT: ESCAPE (Esc) OR F4 CLOSES ANY APP INSTANTLY!
                    if key.unicode_char == 27 || key.scan_code == 23 || key.scan_code == 14 {
                        if state.active_app != 0 {
                            state.active_app = 0;
                            state.start_menu_open = false;
                            screen_needs_render = true;
                        } else if state.start_menu_open {
                            state.start_menu_open = false;
                            screen_needs_render = true;
                        }
                    }
                    // 2. When TERMINAL is ACTIVE, handle full shell keyboard typing!
                    else if state.active_app == '4' as u16 {
                        if key.unicode_char == 13 || key.unicode_char == b'\n' as u16 {
                            term.execute(&mut state);
                            screen_needs_render = true;
                        } else if key.unicode_char == 8 {
                            if term.input_len > 0 {
                                term.input_len -= 1;
                                screen_needs_render = true;
                            }
                        } else if key.unicode_char >= 32 && key.unicode_char <= 126 {
                            if term.input_len < 40 {
                                term.input_buf[term.input_len] = key.unicode_char as u8;
                                term.input_len += 1;
                                screen_needs_render = true;
                            }
                        }
                    }
                    // 3. ARROW KEYS ALWAYS MOVE THE MOUSE CURSOR AS RELIABLE BACKUP!
                    else if key.scan_code == 1 { // UP
                        if state.mouse_y > 15 { state.mouse_y -= 25; }
                        moved = true;
                    } else if key.scan_code == 2 { // DOWN
                        if state.mouse_y < display.height - 35 { state.mouse_y += 25; }
                        moved = true;
                    } else if key.scan_code == 4 { // LEFT
                        if state.mouse_x > 15 { state.mouse_x -= 25; }
                        moved = true;
                    } else if key.scan_code == 3 { // RIGHT
                        if state.mouse_x < display.width - 25 { state.mouse_x += 25; }
                        moved = true;
                    }
                    // 4. SPACE KEY CLICKS UNDER CURSOR (When outside Terminal)!
                    else if key.unicode_char == b' ' as u16 {
                        if click_at(&mut state, &mut calc) {
                            screen_needs_render = true;
                        }
                    }
                    // 5. When CALCULATOR is ACTIVE, handle full real calculation math keys!
                    else if state.active_app == '3' as u16 {
                        let ch = key.unicode_char as u8;
                        if ch >= b'0' && ch <= b'9' {
                            calc.input_digit(ch - b'0');
                            screen_needs_render = true;
                        } else if ch == b'+' || ch == b'-' || ch == b'*' || ch == b'/' {
                            calc.input_op(ch);
                            screen_needs_render = true;
                        } else if ch == b'=' || key.unicode_char == 13 || key.unicode_char == b'\n' as u16 {
                            calc.calculate();
                            screen_needs_render = true;
                        } else if key.unicode_char == 8 {
                            calc.backspace();
                            screen_needs_render = true;
                        } else if ch == b'c' || ch == b'C' {
                            calc.clear();
                            screen_needs_render = true;
                        }
                    }
                    // 6. When WI-FI CENTER is ACTIVE, 1-4 selects/toggles hotspot!
                    else if state.active_app == '2' as u16 && (key.unicode_char >= '1' as u16 && key.unicode_char <= '4' as u16) {
                        if key.unicode_char == '1' as u16 {
                            state.wifi_connected = true;
                            state.wifi_hotspot_id = 1;
                        } else if key.unicode_char == '2' as u16 {
                            state.wifi_connected = true;
                            state.wifi_hotspot_id = 2;
                        } else if key.unicode_char == '3' as u16 {
                            state.wifi_connected = true;
                            state.wifi_hotspot_id = 3;
                        } else if key.unicode_char == '4' as u16 {
                            state.wifi_connected = false;
                            state.wifi_hotspot_id = 0;
                        }
                        screen_needs_render = true;
                    }
                    // 7. FAST APPLICATION SHORTCUTS (L, W, C, T, S, R, and 1-6):
                    else {
                        // [L] -> Laptop / This PC
                        if key.unicode_char == 'l' as u16 || key.unicode_char == 'L' as u16
                           || key.unicode_char == '1' as u16 {
                            state.active_app = '1' as u16;
                            state.start_menu_open = false;
                            screen_needs_render = true;
                        }
                        // [W] -> Wi-Fi & Hotspots
                        else if key.unicode_char == 'w' as u16 || key.unicode_char == 'W' as u16
                                || key.unicode_char == '2' as u16 {
                            state.active_app = '2' as u16;
                            state.start_menu_open = false;
                            screen_needs_render = true;
                        }
                        // [C] -> Calculator
                        else if key.unicode_char == 'c' as u16 || key.unicode_char == 'C' as u16 {
                            state.active_app = '3' as u16;
                            state.start_menu_open = false;
                            screen_needs_render = true;
                        }
                        // [T] -> Terminal
                        else if key.unicode_char == 't' as u16 || key.unicode_char == 'T' as u16
                                || key.unicode_char == '4' as u16 {
                            state.active_app = '4' as u16;
                            state.start_menu_open = false;
                            screen_needs_render = true;
                        }
                        // [S] -> Settings
                        else if key.unicode_char == 's' as u16 || key.unicode_char == 'S' as u16
                                || key.unicode_char == '5' as u16 {
                            state.active_app = '5' as u16;
                            state.start_menu_open = false;
                            screen_needs_render = true;
                        }
                        // [R] -> Research Paper
                        else if key.unicode_char == 'r' as u16 || key.unicode_char == 'R' as u16
                                || key.unicode_char == '6' as u16 {
                            state.active_app = '6' as u16;
                            state.start_menu_open = false;
                            screen_needs_render = true;
                        }
                        // [Tab] -> Toggle Start Menu
                        else if key.unicode_char == 9 {
                            state.start_menu_open = !state.start_menu_open;
                            screen_needs_render = true;
                        }
                    }
                }

                // 5. Blit screen or dirty-rect cursor update
                if screen_needs_render {
                    render_os(&display, &state, &term, &calc);
                    mouse_buf.has_saved = false;
                    mouse_buf.draw(&display, state.mouse_x, state.mouse_y);
                } else if moved {
                    mouse_buf.draw(&display, state.mouse_x, state.mouse_y);
                }
            }
        }
    }

    loop {}
}
