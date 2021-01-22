// TOE Boot Protocol
#![no_std]

#[repr(C)]
pub struct BootInfo {
    pub vram_base: u32,
    pub screen_width: u16,
    pub screen_height: u16,
    pub screen_stride: u16,
    pub screen_bpp: u8,
    _reserved: u8,
    pub memsz_lo: u16,
    pub memsz_mi: u16,
    pub memsz_hi: u32,
    pub kernel_end: u32,
    pub arch: BootArch,
    pub bios_boot_drive: u8,
}

#[repr(u8)]
pub enum BootArch {
    Nec98 = 0,
    PcCompatible = 1,
    FmTowns = 2,
}
