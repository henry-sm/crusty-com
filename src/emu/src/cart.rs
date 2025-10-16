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
        Ok(())
    }

    // Allow the bus to read from the ROM
    pub fn read(&self, addr: u32) -> u8 {
        // This is a simplified LoROM mapping.
        // A real implementation would be more complex.
        self.rom.get(addr as usize).cloned().unwrap_or(0)
    }
}