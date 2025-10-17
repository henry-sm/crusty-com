use std::fs::File;
use std::io::{self, Read};

pub struct Cartridge {
    rom: Vec<u8>,
    // In the future, you'll add header info here
}

impl Cartridge {
    pub fn new() -> Self {
        Cartridge { rom: Vec::new() }
    }

    pub fn load_rom(&mut self, path: &str) -> io::Result<()> {
        let mut file = File::open(path)?;
        file.read_to_end(&mut self.rom)?;
        self.parse_header(); 
        Ok(())
    }

    // Allow the bus to read from the ROM
    pub fn read(&self, addr: u32) -> u8 {
        // This is a simplified LoROM mapping.
        // A real implementation would be more complex.
        self.rom.get(addr as usize).cloned().unwrap_or(0)
    }

    pub fn parse_header(&self) {
        // TODO
        // snes headers at 0x7FC0 (LoROM) or 0xFFC0 (HiROM)
                let header_offset = 0x7FC0;
        if self.rom.len() >= header_offset + 21 {
            let title_bytes = &self.rom[header_offset..header_offset + 21];
            if let Ok(title) = std::str::from_utf8(title_bytes) {
                println!("ROM Title: {}", title.trim());
            } else {
                println!("ROM Title: <invalid UTF-8>");
            }
        } else {
            println!("ROM too small to contain a standard SNES header.");
        }
    }
}