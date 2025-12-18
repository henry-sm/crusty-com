use std::fs::{File};
use std::io::{self, Read, Write};
use std::path::Path;
use crate::mapper::RomMapper;

pub struct Cartridge {
    rom: Vec<u8>,
    sram: Vec<u8>,         // Battery-backed SRAM
    mapper: Option<RomMapper>,
    rom_path: Option<String>, // Path to current ROM for SRAM file operations
}

impl Cartridge {
    pub fn new() -> Self {
        Cartridge {
            rom: Vec::new(),
            sram: Vec::new(),
            mapper: None,
            rom_path: None,
        }
    }

    pub fn load_rom(&mut self, path: &str) -> io::Result<()> {
        let mut file = File::open(path)?;
        self.rom.clear();
        file.read_to_end(&mut self.rom)?;
        
        // Create mapper for this ROM
        self.mapper = Some(RomMapper::new(&self.rom));
        
        // Store path for SRAM file operations
        self.rom_path = Some(path.to_string());
        
        // Get SRAM size from mapper and load SRAM if available
        let sram_size = self.mapper.as_ref().unwrap().ram_size;
        let has_sram = self.mapper.as_ref().unwrap().header.has_sram;
        
        // Initialize SRAM storage
        self.sram.clear();
        self.sram.resize(sram_size, 0xFF); // Initialize to 0xFF (unprogrammed state)
        
        // If this cartridge has SRAM and a battery, try to load existing SRAM file
        if has_sram && sram_size > 0 {
            self.load_sram_file(path).ok(); // Ignore errors if no save file exists
        }
        
        // Print ROM info
        if let Some(ref mapper) = self.mapper {
            mapper.print_info();
        }
        
        Ok(())
    }

    /// Load SRAM from disk (.sram file next to ROM)
    fn load_sram_file(&mut self, rom_path: &str) -> io::Result<()> {
        // Generate SRAM filename by replacing extension with .sram
        let sram_path = Path::new(rom_path)
            .with_extension("sram")
            .to_string_lossy()
            .into_owned();
        
        if Path::new(&sram_path).exists() {
            let mut sram_file = File::open(&sram_path)?;
            let mut contents = Vec::new();
            sram_file.read_to_end(&mut contents)?;
            
            // Only load if file size matches expected SRAM size
            if contents.len() <= self.sram.len() {
                self.sram[..contents.len()].copy_from_slice(&contents);
                eprintln!("Loaded SRAM from: {}", sram_path);
            } else {
                eprintln!("SRAM file size mismatch, ignoring: {}", sram_path);
            }
        }
        
        Ok(())
    }

    /// Save SRAM to disk (.sram file next to ROM)
    pub fn save_sram(&self) -> io::Result<()> {
        if let Some(ref rom_path) = self.rom_path {
            if let Some(ref mapper) = self.mapper {
                if mapper.header.has_sram && !self.sram.is_empty() {
                    let sram_path = Path::new(rom_path)
                        .with_extension("sram")
                        .to_string_lossy()
                        .into_owned();
                    
                    let mut sram_file = File::create(&sram_path)?;
                    sram_file.write_all(&self.sram)?;
                    eprintln!("Saved SRAM to: {}", sram_path);
                }
            }
        }
        Ok(())
    }

    /// Read from cartridge ROM or SRAM using CPU address
    /// The mapper handles the address translation
    pub fn read(&self, addr: u32) -> u8 {
        match &self.mapper {
            Some(mapper) => {
                // Check if this is an SRAM access
                if let Some(sram_offset) = mapper.translate_sram_address(addr) {
                    return self.sram.get(sram_offset).cloned().unwrap_or(0xFF);
                }
                
                // Otherwise, read from ROM
                match mapper.translate_address(addr) {
                    Some(rom_offset) => self.rom.get(rom_offset).cloned().unwrap_or(0),
                    None => 0, // Address not in ROM
                }
            }
            None => 0, // No ROM loaded
        }
    }

    /// Write to SRAM using CPU address
    /// Returns true if write was to SRAM, false otherwise
    pub fn write(&mut self, addr: u32, data: u8) -> bool {
        match &self.mapper {
            Some(mapper) => {
                // Check if this is an SRAM write
                if let Some(sram_offset) = mapper.translate_sram_address(addr) {
                    if sram_offset < self.sram.len() {
                        self.sram[sram_offset] = data;
                        return true;
                    }
                }
            }
            None => {}
        }
        false // Not an SRAM write
    }

    /// Get the mapper reference for detailed queries
    pub fn get_mapper(&self) -> Option<&RomMapper> {
        self.mapper.as_ref()
    }
    
    /// Get the ROM data (for debugging)
    pub fn get_rom_data(&self) -> &[u8] {
        &self.rom
    }
    
    /// Get the SRAM data (for debugging)
    pub fn get_sram_data(&self) -> &[u8] {
        &self.sram
    }
}