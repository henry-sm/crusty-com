use crate::ppu::PPU;
use crate::apu::APU;
use crate::cartridge::Cartridge;

pub struct Bus {
    ram: [u8; 131072], // 128KB WRAM
    pub ppu: PPU,
    apu: APU,
    pub cart: Cartridge,
}

impl Bus {
    pub fn new() -> Self {
        Bus {
            ram: [0; 131072],
            ppu: PPU::new(),
            apu: APU::new(),
            cart: Cartridge::new(),
        }
    }

    // Read data from the bus using bank and address
    pub fn read(&self, bank: u8, addr: u16) -> u8 {
        let full_addr = ((bank as u32) << 16) | (addr as u32);

        match (bank, addr) {
            // System Lo-RAM & Hi-RAM (Banks 00-3F and 80-BF, addresses 0000-1FFF)
            (0x00..=0x3F, 0x0000..=0x1FFF) | (0x80..=0xBF, 0x0000..=0x1FFF) => {
                // This is a mirror of the first 8KB of WRAM
                self.ram[addr as usize % 0x2000]
            }
            // PPU Registers (Banks 00-3F and 80-BF, addresses 2100-21FF)
            (0x00..=0x3F, 0x2100..=0x21FF) | (0x80..=0xBF, 0x2100..=0x21FF) => {
                // In a real implementation, you would have a ppu.read() method
                // For now, this is a placeholder
                0
            }
            // WRAM (Work RAM) Access
            (0x7E..=0x7F, _) => self.ram[((full_addr - 0x7E0000) as usize)],
            // Cartridge ROM (LoROM Memory Map)
            (0x00..=0x3F, 0x8000..=0xFFFF) | (0x80..=0xBF, 0x8000..=0xFFFF) => {
                // This will read from the loaded game cartridge
                // self.cart.read(full_addr) // This would be the actual call
                0 // Placeholder
            }
            _ => {
                // Return 0 for any unmapped memory regions
                0
            }
        }
    }

    // Write data to the bus using bank and address
    pub fn write(&mut self, bank: u8, addr: u16, data: u8) {
        let full_addr = ((bank as u32) << 16) | (addr as u32);

        match (bank, addr) {
            // System Lo-RAM & Hi-RAM (Banks 00-3F and 80-BF, addresses 0000-1FFF)
            (0x00..=0x3F, 0x0000..=0x1FFF) | (0x80..=0xBF, 0x0000..=0x1FFF) => {
                self.ram[addr as usize % 0x2000] = data;
            }
            // PPU Registers (Banks 00-3F and 80-BF, addresses 2100-21FF)
            (0x00..=0x3F, 0x2100..=0x21FF) | (0x80..=0xBF, 0x2100..=0x21FF) => {
                // self.ppu.write(addr, data); // Future implementation
            }
             // WRAM (Work RAM) Access
            (0x7E..=0x7F, _) => {
                self.ram[((full_addr - 0x7E0000) as usize)] = data;
            }
            // Writing to ROM is ignored
            _ => {}
        }
    }
}