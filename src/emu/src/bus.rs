use crate::ppu::PPU;
use crate::apu::APU;
use crate::cart::Cartridge;
use crate::input::Input;
use crate::dma::DMAController;

pub struct Bus {
    ram: Box<[u8; 131072]>, // 128KB WRAM - heap-allocated
    pub ppu: PPU,
    pub apu: APU,
    pub cart: Cartridge,
    pub input: Input,
    pub dma: DMAController,
    pub cpu_io_registers: [u8; 0x10],  // CPU I/O registers 0x4200-0x420F
}

impl Bus {
    pub fn new() -> Self {
        let mut ram = Box::new([0; 131072]);
        
        // Initialize RAM with values expected by SNES bootloader
        // RAM[0x31] = 0x03 signals that hardware initialization is complete (IPL ROM sets this)
        // This allows bootloader code that waits for this flag to proceed
        ram[0x31] = 0x03;
        
        Bus {
            ram,
            ppu: PPU::new(),
            apu: APU::new(),
            cart: Cartridge::new(),
            input: Input::new(),
            dma: DMAController::new(),
            cpu_io_registers: [0; 0x10],
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
            // Note: APU registers are at 2140-217F but usually accessed through PPU region
            (0x00..=0x3F, 0x2100..=0x21FF) | (0x80..=0xBF, 0x2100..=0x21FF) => {
                self.ppu.read_register((addr - 0x2100) as u8)
            }
            
            // CPU I/O Registers (Banks 00-3F and 80-BF, addresses 4200-42FF)
            (0x00..=0x3F, 0x4200..=0x42FF) | (0x80..=0xBF, 0x4200..=0x42FF) => {
                // CPU I/O registers: interrupt control, NMI, DMA, etc.
                // 4200 = NMITIMEN (NMI/Timer enable)
                // 4201 = WRIO (Joypad/programmable I/O port)
                // 4202-4203 = WRMPYA/B (Multiplication operands)
                // 4204-4206 = WRDIVL/H, WRDIVB (Division operands)
                // 4207-4209 = HTIMEL/H, VTIMEL (H/V timer settings)
                // 420A-420B = MDMAEN, HDMAEN (DMA/HDMA enable)
                // 4212 = HVBJOY (H/V-Blank flag and Joypad Busy flag) - READ ONLY
                // 4216-4219 = Joypad data (serial or latched)
                match addr {
                    0x4200..=0x420F => {
                        // Read from CPU I/O registers
                        self.cpu_io_registers[(addr - 0x4200) as usize]
                    }
                    0x4212 => {
                        // HVBJOY - H/V-Blank flag and Joypad Busy flag (READ ONLY)
                        // Bit 7: V-Blank Period Flag (0=No, 1=VBlank)
                        // Bit 6: H-Blank Period Flag (0=No, 1=HBlank)
                        // Bits 5-1: Not used
                        // Bit 0: Auto-Joypad-Read Busy Flag (usually 0)
                        let mut status = 0u8;
                        if self.ppu.vblank { status |= 0x80; }
                        if self.ppu.hblank { status |= 0x40; }
                        status
                    }
                    0x4217 => self.input.read_serial(), // Joypad data (serial)
                    0x4218..=0x4219 => self.input.read_buttons(), // Joypad data (latched)
                    _ => 0 // Other I/O registers
                }
            }
            
            // DMA/HDMA Registers (Banks 00-3F and 80-BF, addresses 4300-437F)
            (0x00..=0x3F, 0x4300..=0x437F) | (0x80..=0xBF, 0x4300..=0x437F) => {
                // 8 DMA channels, each has 16 bytes of registers
                // 4300-430F = DMA0 registers
                // 4310-431F = DMA1 registers
                // ... through ...
                // 4370-437F = DMA7 registers
                self.dma.read_register(addr)
            }
            
            // WRAM (Work RAM) Access (Banks 7E-7F, all addresses)
            (0x7E..=0x7F, _) => self.ram[(full_addr - 0x7E0000) as usize],
            
            // Cartridge ROM (LoROM extended banks 40-7D, addresses 0000-FFFF)
            // ALSO LoROM banks 00-3F, addresses 0000-7FFF (for header/code in some games)
            (0x00..=0x3F, 0x0000..=0x7FFF) | (0x40..=0x7D, 0x0000..=0xFFFF) => {
                self.cart.read(full_addr)
            }
            
            // Cartridge ROM (LoROM Memory Map - Banks 00-7D, addresses 8000-FFFF)
            (0x00..=0x3F, 0x8000..=0xFFFF) | (0x80..=0xBF, 0x8000..=0xFFFF) => {
                self.cart.read(full_addr)
            }
            
            // Cartridge ROM (HiROM Memory Map - Banks C0-FF)
            (0xC0..=0xFF, _) => {
                self.cart.read(full_addr)
            }
            
            // Unmapped memory - return 0
            _ => 0
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
                self.ppu.write_register((addr - 0x2100) as u8, data);
            }
            
            // CPU I/O Registers (Banks 00-3F and 80-BF, addresses 4200-42FF)
            (0x00..=0x3F, 0x4200..=0x42FF) | (0x80..=0xBF, 0x4200..=0x42FF) => {
                // CPU I/O registers write handling
                match addr {
                    0x420A => {
                        // MDMAEN (Master DMA Enable)
                        // Each bit enables a DMA channel
                        self.cpu_io_registers[(addr - 0x4200) as usize] = data;
                        
                        // Execute DMA immediately when any bit is set
                        if data != 0 {
                            self.dma.dma_enabled = true;
                            
                            // Enable individual channels based on bits
                            for channel_idx in 0..8 {
                                if (data & (1 << channel_idx)) != 0 {
                                    self.dma.write_register(0x4300 + (channel_idx as u16) * 16, 
                                        self.dma.read_register(0x4300 + (channel_idx as u16) * 16) | 0x80);
                                }
                            }
                            
                            // Execute DMA - we'll handle the PPU write through register writes
                            self.execute_dma_transfer();
                        }
                    }
                    0x4200..=0x420F => {
                        // Store in cpu_io_registers array
                        self.cpu_io_registers[(addr - 0x4200) as usize] = data;
                    }
                    0x4016 => {
                        // JOYSER0 - Joypad strobe
                        self.input.write_strobe(data);
                    }
                    _ => {
                        // Other I/O registers (not implemented)
                    }
                }
            }
            
            // DMA/HDMA Registers (Banks 00-3F and 80-BF, addresses 4300-437F)
            (0x00..=0x3F, 0x4300..=0x437F) | (0x80..=0xBF, 0x4300..=0x437F) => {
                // DMA/HDMA channel register writes
                // SOURCE: No$SNS DMA register specification
                self.dma.write_register(addr, data);
            }
            
            // WRAM (Work RAM) Access (Banks 7E-7F, all addresses)
            (0x7E..=0x7F, _) => {
                self.ram[(full_addr - 0x7E0000) as usize] = data;
            }
            
            // Writing to Cartridge ROM is ignored
            (0x00..=0x3F, 0x8000..=0xFFFF) | (0x80..=0xBF, 0x8000..=0xFFFF) => {
                // ROM write ignored
            }
            
            // HiROM cartridge
            (0xC0..=0xFF, _) => {
                // ROM write ignored
            }
            
            // Unmapped memory writes are ignored
            _ => {}
        }
    }
    
    // Execute DMA transfer for all enabled channels
    fn execute_dma_transfer(&mut self) {
        // We need to manually iterate through channels and transfer data
        // since we can't easily pass closure with mutable PPU reference
        
        // For now, implement a simplified DMA that writes to PPU registers
        // We'll handle this by directly manipulating the PPU state
        
        for channel_idx in 0..8 {
            // Read channel configuration
            let channel_addr = 0x4300 + (channel_idx as u16) * 16;
            
            // Check if channel is enabled
            let control = self.dma.read_register(channel_addr);
            if (control & 0x80) == 0 {
                continue; // Channel not enabled
            }
            
            let dest = self.dma.read_register(channel_addr + 1);
            let src_low = self.dma.read_register(channel_addr + 2) as u32;
            let src_mid = self.dma.read_register(channel_addr + 3) as u32;
            let src_bank = self.dma.read_register(channel_addr + 4) as u32;
            let size_low = self.dma.read_register(channel_addr + 5) as u16;
            let size_high = self.dma.read_register(channel_addr + 6) as u16;
            
            let source_addr = (src_bank << 16) | (src_mid << 8) | src_low;
            let transfer_size = if size_low == 0 && size_high == 0 { 
                0x10000u32 
            } else { 
                ((size_high as u32) << 8) | (size_low as u32)
            };
            
            // For MODE 0 (most common), write to single PPU register
            for i in 0..transfer_size {
                let addr = (source_addr + i) as usize;
                let data = if addr < self.ram.len() {
                    self.ram[addr]
                } else {
                    0
                };
                
                // Write to PPU register via proper channel
                self.ppu.write_register(dest, data);
            }
            
            // Clear the enable bit for this channel
            self.dma.write_register(channel_addr, control & 0x7F);
        }
        
        self.dma.dma_enabled = false;
    }
    
    // Trigger NMI interrupt (from PPU V-blank)
    pub fn trigger_nmi(&mut self, cpu: &mut crate::cpu::_65816) {
        cpu.nmi_pending = true;
    }
    
    // Trigger IRQ interrupt
    pub fn trigger_irq(&mut self, cpu: &mut crate::cpu::_65816) {
        cpu.irq_pending = true;
    }
}