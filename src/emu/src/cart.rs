use std::fs::File;
use std::io::{self, Read};
use crate::mapper::RomMapper;

pub struct Cartridge {
    rom: Vec<u8>,
    mapper: Option<RomMapper>,
}

impl Cartridge {
    pub fn new() -> Self {
        Cartridge {
            rom: Vec::new(),
            mapper: None,
        }
    }

    pub fn load_rom(&mut self, path: &str) -> io::Result<()> {
        let mut file = File::open(path)?;
        self.rom.clear();
        file.read_to_end(&mut self.rom)?;
        
        // Create mapper for this ROM
        self.mapper = Some(RomMapper::new(&self.rom));
        
        // Print ROM info
        if let Some(ref mapper) = self.mapper {
            mapper.print_info();
        }
        
        Ok(())
    }

    /// Read from cartridge ROM using CPU address
    /// The mapper handles the address translation
    pub fn read(&self, addr: u32) -> u8 {
        match &self.mapper {
            Some(mapper) => {
                match mapper.translate_address(addr) {
                    Some(rom_offset) => self.rom.get(rom_offset).cloned().unwrap_or(0),
                    None => 0, // Address not in ROM
                }
            }
            None => 0, // No ROM loaded
        }
    }

    /// Get the mapper reference for detailed queries
    pub fn get_mapper(&self) -> Option<&RomMapper> {
        self.mapper.as_ref()
    }
}