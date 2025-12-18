// SNES ROM Mapper - Handles LoROM/HiROM address translation
//
// SOURCES:
// 1. SNES ROM Cartridge Format Documentation
//    - LoROM header at 0x7FC0: [Title (21 bytes), MapMode, CartType, RomSize, SramSize, Country, (2 bytes), Checksum, Checksum complement]
//    - HiROM header at 0xFFC0: Same structure but in different bank
//    - Reference: Technical SNES documentation, reverse engineering
//
// 2. BSNES Cartridge Implementation (byuu/bsnes)
//    - File: bsnes/cartridge/cartridge.cpp
//    - Sections: ROM type detection logic, address translation formulas
//    - Used for: translate_lorom() and translate_hirom() algorithms
//
// 3. SNES Hardware Documentation (No$SNS by Martin Korth)
//    - Section: ROM cartridge types and address mapping
//    - Used for: Detailed ROM layout specifications
//
// 4. ZSNES Source Code
//    - File: src/lib/gfx.c
//    - Used for: Header validation and ROM type verification

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RomType {
    LoRom,      // 32KB banks, common in early SNES games (SOURCE: ROM format docs)
    HiRom,      // 64KB banks, less common but faster (SOURCE: BSNES cartridge.cpp)
    ExLoRom,    // Extended LoROM (rare, special addressing) (SOURCE: SA-1 cartridge specs)
    Unknown,    // Could not detect type
}

#[derive(Clone, Copy, Debug)]
pub struct RomHeader {
    pub rom_type: RomType,
    pub title: [u8; 21],    // Game title
    pub map_mode: u8,       // Cartridge ROM type/mapper info
    pub cartridge_type: u8, // 0=ROM only, 1=ROM+RAM, 2=ROM+RAM+Battery, etc.
    pub rom_size: u8,       // In 256KB units (usually)
    pub ram_size: u8,
    pub country: u8,
    pub checksum: u16,
    pub has_sram: bool,
    pub has_battery: bool,
}

pub struct RomMapper {
    pub header: RomHeader,
    pub rom_size: usize,
    pub ram_size: usize,
}

impl RomMapper {
    /// Create a new mapper by analyzing the ROM data
    pub fn new(rom_data: &[u8]) -> Self {
        let (header, rom_type) = Self::parse_header(rom_data);
        let rom_size = rom_data.len();
        let ram_size = Self::calculate_ram_size(header.ram_size);

        RomMapper {
            header: RomHeader {
                rom_type,
                ..header
            },
            rom_size,
            ram_size,
        }
    }

    /// Detect ROM type from header information
    fn detect_rom_type(map_mode: u8, _rom_data: &[u8]) -> RomType {
        // Map mode byte structure:
        // High nibble: various features
        // Low nibble: determines bank layout
        let bank_layout = map_mode & 0x0F;

        match bank_layout {
            0x00..=0x04 | 0x08 | 0x09 => {
                // LoROM variants (0x00-0x04, and also 0x08-0x09 for some games)
                RomType::LoRom
            }
            0x05 => {
                // ExLoROM (extended LoROM)
                RomType::ExLoRom
            }
            0x20..=0x27 => {
                // HiROM variants
                RomType::HiRom
            }
            0x30..=0x37 => {
                // HiROM variants with different chip select
                RomType::HiRom
            }
            _ => RomType::Unknown,
        }
    }

    /// Parse SNES ROM header
    fn parse_header(rom_data: &[u8]) -> (RomHeader, RomType) {
        // Try LoROM header first (0x7FC0)
        let mut title = [0u8; 21];
        let mut map_mode = 0u8;
        let mut cartridge_type = 0u8;
        let mut rom_size = 0u8;
        let mut ram_size = 0u8;
        let mut country = 0u8;
        let mut checksum = 0u16;

        // Try LoROM header at 0x7FC0
        if rom_data.len() >= 0x7FC0 + 32 {
            let offset = 0x7FC0;
            title = Self::read_title(&rom_data[offset..offset + 21]);
            map_mode = rom_data[offset + 21];
            cartridge_type = rom_data[offset + 22];
            rom_size = rom_data[offset + 23];
            ram_size = rom_data[offset + 24];
            country = rom_data[offset + 25];
            checksum = u16::from_le_bytes([
                rom_data[offset + 28],
                rom_data[offset + 29],
            ]);
        } else if rom_data.len() >= 0xFFC0 + 32 {
            // Try HiROM header at 0xFFC0
            let offset = 0xFFC0;
            title = Self::read_title(&rom_data[offset..offset + 21]);
            map_mode = rom_data[offset + 21];
            cartridge_type = rom_data[offset + 22];
            rom_size = rom_data[offset + 23];
            ram_size = rom_data[offset + 24];
            country = rom_data[offset + 25];
            checksum = u16::from_le_bytes([
                rom_data[offset + 28],
                rom_data[offset + 29],
            ]);
        }

        let rom_type = Self::detect_rom_type(map_mode, rom_data);

        let header = RomHeader {
            rom_type,
            title,
            map_mode,
            cartridge_type,
            rom_size,
            ram_size,
            country,
            checksum,
            has_sram: (cartridge_type & 0x01) != 0,
            has_battery: (cartridge_type & 0x02) != 0,
        };

        (header, rom_type)
    }

    /// Read title from ROM header (21 bytes, trim spaces)
    fn read_title(data: &[u8]) -> [u8; 21] {
        let mut title = [0u8; 21];
        if data.len() >= 21 {
            title.copy_from_slice(&data[0..21]);
        }
        title
    }

    /// Calculate RAM size from ROM header value
    fn calculate_ram_size(size_byte: u8) -> usize {
        match size_byte {
            0x00 => 0,        // No RAM
            0x01 => 2 * 1024, // 2KB
            0x02 => 4 * 1024, // 4KB
            0x03 => 8 * 1024, // 8KB
            0x04 => 32 * 1024,
            0x05 => 64 * 1024,
            0x06 => 128 * 1024,
            0x07 => 256 * 1024,
            0x08 => 512 * 1024,
            _ => 0,
        }
    }

    /// Translate CPU address to ROM offset (LoROM format)
    /// SOURCE: SNES ROM Format Documentation + BSNES cartridge.cpp
    /// LoROM memory map breakdown:
    /// - Banks 00-3F, 80-BF, addresses 0x8000-0xFFFF: 32KB ROM per bank
    /// - Banks 40-7D, C0-FF, addresses 0x0000-0xFFFF: Extended 64KB ROM per bank
    /// Formula from BSNES: rom_offset = ((bank & 0x3F) << 15) | (addr & 0x7FFF)
    fn translate_lorom(&self, addr: u32) -> Option<usize> {
        let bank = (addr >> 16) as u8;
        let offset = (addr & 0xFFFF) as u16;

        // LoROM memory map:
        // Banks 00-3F, 80-BF, addresses 0x8000-0xFFFF: ROM
        // Banks 40-7D, C0-FF, addresses 0x0000-0xFFFF: ROM (extended)
        // Banks 00-3F, addresses 0x0000-0x7FFF: Can contain ROM (headers, code in some games)

        match (bank, offset) {
            // Standard ROM region (Banks 00-3F and 80-BF)
            (0x00..=0x3F, 0x8000..=0xFFFF) | (0x80..=0xBF, 0x8000..=0xFFFF) => {
                // Each bank has 32KB of ROM at 0x8000
                let rom_offset = ((bank & 0x3F) as usize * 0x8000) + (offset as usize - 0x8000);
                if rom_offset < self.rom_size {
                    Some(rom_offset)
                } else {
                    None
                }
            }

            // Extended ROM region (Banks 40-7D and C0-FF)
            (0x40..=0x7D, _) => {
                // Banks 40-7D have full 64KB ROM access
                let rom_offset = ((bank as usize - 0x40) * 0x10000) + 0x200000 + (offset as usize);
                if rom_offset < self.rom_size {
                    Some(rom_offset)
                } else {
                    None
                }
            }

            (0xC0..=0xFF, _) => {
                // Banks C0-FF are mirror of 40-7D for some mappers
                let rom_offset = ((bank as usize - 0xC0) * 0x10000) + 0x200000 + (offset as usize);
                if rom_offset < self.rom_size {
                    Some(rom_offset)
                } else {
                    None
                }
            }

            // LoROM low address space (Banks 00-3F and 80-BF, addresses 0x0000-0x7FFF)
            // This is system RAM region, NOT ROM
            (0x00..=0x3F, 0x0000..=0x7FFF) | (0x80..=0xBF, 0x0000..=0x7FFF) => {
                // System RAM region - not directly ROM mapped
                None
            }

            _ => None,
        }
    }

    /// Translate CPU address to ROM offset (HiROM format)
    /// SOURCE: SNES ROM Format Documentation + BSNES cartridge.cpp
    /// HiROM memory map breakdown:
    /// - Banks 40-7F, C0-FF, addresses 0x0000-0xFFFF: Full 64KB ROM per bank
    /// - No 0x8000 offset like LoROM, direct linear addressing
    /// Formula from BSNES: rom_offset = ((bank & 0x3F) << 16) | addr
    fn translate_hirom(&self, addr: u32) -> Option<usize> {
        let bank = (addr >> 16) as u8;
        let offset = (addr & 0xFFFF) as u16;

        // HiROM memory map:
        // Banks 40-7F, C0-FF, addresses 0x0000-0xFFFF: ROM (64KB each)

        match bank {
            // Primary HiROM region
            (0x40..=0x7D) => {
                let rom_offset = ((bank as usize - 0x40) * 0x10000) + (offset as usize);
                if rom_offset < self.rom_size {
                    Some(rom_offset)
                } else {
                    None
                }
            }

            // Mirrored HiROM region
            (0xC0..=0xFF) => {
                let rom_offset = ((bank as usize - 0xC0) * 0x10000) + (offset as usize);
                if rom_offset < self.rom_size {
                    Some(rom_offset)
                } else {
                    None
                }
            }

            _ => None,
        }
    }

    /// Translate CPU address to ROM offset (ExLoROM format - rare)
    fn translate_exlorom(&self, addr: u32) -> Option<usize> {
        // ExLoROM is a variant of LoROM with different bank switching
        // For now, use LoROM translation
        // Proper ExLoROM would need chip select emulation
        self.translate_lorom(addr)
    }

    /// Main translation function - convert CPU address to ROM offset
    pub fn translate_address(&self, addr: u32) -> Option<usize> {
        match self.header.rom_type {
            RomType::LoRom => self.translate_lorom(addr),
            RomType::HiRom => self.translate_hirom(addr),
            RomType::ExLoRom => self.translate_exlorom(addr),
            RomType::Unknown => {
                // Try to guess based on address pattern
                // If address is in HiROM region, use HiROM translation
                let bank = (addr >> 16) as u8;
                match bank {
                    0x40..=0x7D | 0xC0..=0xFF => self.translate_hirom(addr),
                    _ => self.translate_lorom(addr),
                }
            }
        }
    }

    /// Translate CPU address to SRAM offset (if applicable)
    /// SRAM mapping varies by mapper type and game
    /// Common LoROM SRAM locations: 0x70:0000-0x7D:FFFF (SuperFX), 0xA0-0xBF (standard)
    /// For simplicity, we support standard LoROM SRAM at 0x70-0x7F (fixed 64KB window)
    pub fn translate_sram_address(&self, addr: u32) -> Option<usize> {
        if !self.header.has_sram {
            return None;
        }

        let bank = (addr >> 16) as u8;
        let offset = (addr & 0xFFFF) as u16;

        match self.header.rom_type {
            RomType::LoRom => {
                // Standard LoROM SRAM at banks 0x70-0x7F (fixed mapping)
                // This is the most common location for battery-backed SRAM
                match bank {
                    0x70..=0x7D => {
                        // Each bank is 64KB, but SRAM is typically much smaller
                        // Map all accesses to the same SRAM space
                        let sram_offset = (offset as usize) % self.ram_size;
                        if sram_offset < self.ram_size {
                            Some(sram_offset)
                        } else {
                            None
                        }
                    }
                    // Some games use 0x20-0x3F for SRAM (depends on header flags)
                    0x20..=0x3F => {
                        // Alternative SRAM location for some games
                        let sram_offset = (offset as usize) % self.ram_size;
                        if sram_offset < self.ram_size {
                            Some(sram_offset)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            RomType::HiRom => {
                // HiROM typically uses 0x30-0x3F for SRAM
                match bank {
                    0x30..=0x3F => {
                        let sram_offset = (offset as usize) % self.ram_size;
                        if sram_offset < self.ram_size {
                            Some(sram_offset)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Check if an address points to ROM
    pub fn is_rom_address(&self, addr: u32) -> bool {
        self.translate_address(addr).is_some()
    }

    /// Print ROM info (for debugging)
    pub fn print_info(&self) {
        let title_str = String::from_utf8_lossy(&self.header.title);
        println!("=== SNES ROM Information ===");
        println!("Title: {}", title_str.trim());
        println!("ROM Type: {:?}", self.header.rom_type);
        println!("ROM Size: {} KB", self.rom_size / 1024);
        println!("RAM Size: {} KB", self.ram_size / 1024);
        println!("Has SRAM: {}", self.header.has_sram);
        println!("Has Battery: {}", self.header.has_battery);
        println!("Country: {}", self.get_country_name(self.header.country));
        println!("Checksum: 0x{:04X}", self.header.checksum);
    }

    /// Get country/region name from country code
    fn get_country_name(&self, code: u8) -> &'static str {
        match code {
            0x00 => "Japan",
            0x01 => "USA",
            0x02 => "PAL (Europe)",
            0x03 => "Sweden",
            0x04 => "Finland",
            0x05 => "Denmark",
            0x06 => "France",
            0x07 => "Holland",
            0x08 => "Spain",
            0x09 => "Germany",
            0x0A => "Italy",
            0x0B => "Hong Kong",
            0x0C => "Indonesia",
            0x0D => "India",
            0x0E => "Unknown",
            _ => "Invalid",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lorom_address_translation() {
        // Create dummy ROM data
        let mut rom = vec![0u8; 0x100000]; // 1MB ROM

        // Set up fake LoROM header at 0x7FC0
        let header_offset = 0x7FC0;
        rom[header_offset] = b'T'; // Title start
        rom[header_offset + 21] = 0x20; // Map mode (LoROM)

        let mapper = RomMapper::new(&rom);

        // Test standard ROM region
        // Bank 00, Address 0x8000 should map to ROM offset 0x0000
        assert_eq!(mapper.translate_lorom(0x008000), Some(0x0000));

        // Bank 01, Address 0x8000 should map to ROM offset 0x8000
        assert_eq!(mapper.translate_lorom(0x018000), Some(0x8000));

        // Bank 00, Address 0x0000 should not be ROM (system RAM region)
        assert_eq!(mapper.translate_lorom(0x000000), None);
    }

    #[test]
    fn test_hirom_address_translation() {
        let mut rom = vec![0u8; 0x200000]; // 2MB ROM

        // Set up fake HiROM header
        let header_offset = 0xFFC0;
        rom[header_offset] = b'T';
        rom[header_offset + 21] = 0x21; // Map mode (HiROM)

        let mapper = RomMapper::new(&rom);

        // Bank 40, Address 0x0000 should map to ROM offset 0x0000
        assert_eq!(mapper.translate_hirom(0x400000), Some(0x0000));

        // Bank C0, Address 0x0000 should also map to ROM offset 0x0000 (mirror)
        assert_eq!(mapper.translate_hirom(0xC00000), Some(0x0000));
    }
}
