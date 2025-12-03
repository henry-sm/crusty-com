// DMA/HDMA System - Direct Memory Access for SNES
//
// SOURCES:
// 1. No$SNS Documentation
//    - Sections: DMA register layout at 0x4300-0x437F
//    - HDMA register layout at 0x4300-0x437F (mode-specific)
//    - Transfer mode specifications
// 2. BSNES Implementation
//    - File: bsnes/cpu/dma.cpp
//    - Algorithm: Multi-channel DMA execution, mode handling
//    - Used for: Channel structure, transfer logic
// 3. SNES Technical Reference
//    - DMA channel specifications
//    - HDMA timing and per-scanline behavior
//    - Transfer cycle accuracy

/// DMA Channel Structure
/// SOURCE: No$SNS DMA register documentation (0x4300-0x430F per channel)
/// Each DMA channel has 16 bytes of registers:
/// 0x00: Control register
/// 0x01: Destination register
/// 0x02-0x03: Source address (low/mid)
/// 0x04: Source bank
/// 0x05-0x06: Transfer size
/// 0x07: Indirect addressing parameter
/// 0x08-0x0F: HDMA-specific registers
#[derive(Clone, Copy, Debug)]
pub struct DMAChannel {
    // Control register (0x4300, 0x4310, 0x4320, etc.)
    pub mode: u8,               // Bits 0-2: Transfer mode (0-7)
    pub direction: bool,        // Bit 3: 0=CPU->PPU, 1=PPU->CPU
    pub indirect: bool,         // Bit 4: Indirect addressing mode
    pub reverse: bool,          // Bit 5: Address reverse (HDMA only)
    pub fixed: bool,            // Bit 6: Fixed address (no increment)
    pub enabled: bool,          // Bit 7: Transfer enabled

    // Destination register (0x4301, 0x4311, etc.)
    pub destination: u8,        // PPU register target

    // Source address (0x4302-0x4304)
    pub source_addr: u32,       // 24-bit CPU address

    // Transfer size (0x4305-0x4306)
    pub transfer_size: u16,     // Byte count to transfer

    // HDMA specific (0x4307-0x430F)
    pub hdma_line_counter: u8,  // Lines remaining in HDMA block
    pub hdma_table_addr: u16,   // HDMA table address in WRAM bank 0x7E
    pub hdma_finished: bool,    // HDMA transfer complete
}

impl DMAChannel {
    pub fn new() -> Self {
        DMAChannel {
            mode: 0,
            direction: false,
            indirect: false,
            reverse: false,
            fixed: false,
            enabled: false,
            destination: 0,
            source_addr: 0,
            transfer_size: 0,
            hdma_line_counter: 0,
            hdma_table_addr: 0,
            hdma_finished: false,
        }
    }

    /// Parse control register byte into channel config
    /// SOURCE: No$SNS register 0x4300/0x4310/etc. specification
    pub fn set_control(&mut self, value: u8) {
        self.mode = value & 0x07;           // Bits 0-2
        self.direction = (value & 0x08) != 0; // Bit 3
        self.indirect = (value & 0x10) != 0;  // Bit 4
        self.reverse = (value & 0x20) != 0;   // Bit 5
        self.fixed = (value & 0x40) != 0;     // Bit 6
        self.enabled = (value & 0x80) != 0;   // Bit 7
    }

    /// Get control register byte from channel config
    pub fn get_control(&self) -> u8 {
        let mut value = self.mode & 0x07;
        if self.direction { value |= 0x08; }
        if self.indirect { value |= 0x10; }
        if self.reverse { value |= 0x20; }
        if self.fixed { value |= 0x40; }
        if self.enabled { value |= 0x80; }
        value
    }
}

pub struct DMAController {
    channels: [DMAChannel; 8],
    
    // DMA Master Enable (0x420C)
    pub dma_enabled: bool,
    
    // HDMA Enable (0x420D)
    pub hdma_enabled: bool,
    
    // DMA Channel Priority (0x420E) - not used in Mode 7
    pub channel_priority: u8,
    
    // CPU IRQ vectors
    pub nmi_vector: u16,
    pub irq_vector: u16,
}

impl DMAController {
    pub fn new() -> Self {
        DMAController {
            channels: [DMAChannel::new(); 8],
            dma_enabled: false,
            hdma_enabled: false,
            channel_priority: 0,
            nmi_vector: 0xFFFA,
            irq_vector: 0xFFFE,
        }
    }

    /// Get DMA channel register byte
    /// Registers: 0x4300-0x437F (8 channels × 16 bytes)
    pub fn read_register(&self, addr: u16) -> u8 {
        if addr < 0x4300 || addr >= 0x4380 {
            return 0; // Out of range
        }

        let channel = ((addr - 0x4300) / 16) as usize;
        let offset = ((addr - 0x4300) % 16) as usize;

        if channel >= 8 {
            return 0;
        }

        let ch = &self.channels[channel];

        match offset {
            0x00 => ch.get_control(),
            0x01 => ch.destination,
            0x02 => (ch.source_addr & 0xFF) as u8,
            0x03 => ((ch.source_addr >> 8) & 0xFF) as u8,
            0x04 => ((ch.source_addr >> 16) & 0xFF) as u8,
            0x05 => (ch.transfer_size & 0xFF) as u8,
            0x06 => ((ch.transfer_size >> 8) & 0xFF) as u8,
            0x07 => ch.hdma_table_addr as u8,
            0x08 => (ch.hdma_table_addr >> 8) as u8,
            0x09 => ch.hdma_line_counter,
            0x0A => 0, // Reserved
            0x0B => 0, // Reserved
            0x0C => 0, // Reserved
            0x0D => 0, // Reserved
            0x0E => 0, // Reserved
            0x0F => 0, // Reserved
            _ => 0,
        }
    }

    /// Set DMA channel register byte
    pub fn write_register(&mut self, addr: u16, value: u8) {
        if addr < 0x4300 || addr >= 0x4380 {
            return;
        }

        let channel = ((addr - 0x4300) / 16) as usize;
        let offset = ((addr - 0x4300) % 16) as usize;

        if channel >= 8 {
            return;
        }

        let ch = &mut self.channels[channel];

        match offset {
            0x00 => ch.set_control(value),
            0x01 => ch.destination = value,
            0x02 => ch.source_addr = (ch.source_addr & 0xFFFF00) | (value as u32),
            0x03 => ch.source_addr = (ch.source_addr & 0xFF00FF) | ((value as u32) << 8),
            0x04 => ch.source_addr = (ch.source_addr & 0x00FFFF) | ((value as u32) << 16),
            0x05 => ch.transfer_size = (ch.transfer_size & 0xFF00) | (value as u16),
            0x06 => ch.transfer_size = (ch.transfer_size & 0x00FF) | ((value as u16) << 8),
            0x07 => ch.hdma_table_addr = (ch.hdma_table_addr & 0xFF00) | (value as u16),
            0x08 => ch.hdma_table_addr = (ch.hdma_table_addr & 0x00FF) | ((value as u16) << 8),
            0x09 => ch.hdma_line_counter = value,
            _ => {} // Reserved/read-only
        }
    }

    /// Get DMA master enable status
    pub fn get_dma_enabled(&self) -> bool {
        self.dma_enabled
    }

    /// Set DMA master enable (0x420C)
    pub fn set_dma_enabled(&mut self, value: bool) {
        self.dma_enabled = value;
    }

    /// Get HDMA enable status (0x420D)
    pub fn get_hdma_enabled(&self) -> u8 {
        let mut value = 0u8;
        for i in 0..8 {
            if self.channels[i].enabled {
                value |= 1 << i;
            }
        }
        value
    }

    /// Set HDMA enable (0x420D)
    pub fn set_hdma_enabled(&mut self, value: u8) {
        for i in 0..8 {
            self.channels[i].enabled = (value & (1 << i)) != 0;
        }
    }

    /// Execute DMA transfer for specified channels
    /// SOURCE: BSNES dma.cpp - execute_dma() algorithm
    /// This transfers data from CPU memory to PPU registers
    pub fn execute_dma(&mut self, wram: &[u8], ppu_write: impl Fn(u8, u8)) {
        if !self.dma_enabled {
            return;
        }

        // Process each enabled DMA channel
        for channel_idx in 0..8 {
            if !self.channels[channel_idx].enabled {
                continue;
            }

            let ch = &mut self.channels[channel_idx];
            let bytes_to_transfer = if ch.transfer_size == 0 { 
                0x10000u32  // 64KB when size register is 0
            } else { 
                ch.transfer_size as u32
            };

            // MODE 0: 1 register write per transfer
            // MODE 1: 2 register writes per transfer (alternating)
            // MODE 2: 2 register writes per transfer (both same)
            // MODE 3: 4 register writes per transfer
            // MODE 4: 4 register writes per transfer (alternating pairs)
            // MODE 5: 4 register writes per transfer
            // MODE 6: 2 register writes per transfer
            // MODE 7: 1 register write per transfer

            // For now, implement simplified MODE 0 (most common)
            if ch.mode == 0 {
                for _ in 0..bytes_to_transfer {
                    if ch.source_addr >= 0x1000000 {
                        break; // Out of addressable range
                    }

                    // Read from WRAM (assuming source is always WRAM for now)
                    let data = if ch.source_addr < wram.len() as u32 {
                        wram[ch.source_addr as usize]
                    } else {
                        0
                    };

                    // Call PPU write callback
                    ppu_write(ch.destination, data);

                    // Increment source address if not fixed
                    if !ch.fixed {
                        ch.source_addr = ch.source_addr.wrapping_add(1);
                    }
                }
                ch.transfer_size = 0; // Transfer complete
                ch.enabled = false;
            }
        }

        self.dma_enabled = false;
    }

    /// Execute HDMA (Horizontal DMA) - per-scanline transfers
    /// SOURCE: No$SNS HDMA documentation
    /// HDMA transfers occur during H-blank for each scanline
    pub fn execute_hdma_scanline(&mut self, _bus: &mut crate::bus::Bus, _scanline: u16) {
        // HDMA is complex and requires per-scanline handling
        // For now, stub implementation
        // Future: Read HDMA table from WRAM, transfer data per-scanline
    }

    /// Reset all DMA channels
    pub fn reset(&mut self) {
        for i in 0..8 {
            self.channels[i] = DMAChannel::new();
        }
        self.dma_enabled = false;
        self.hdma_enabled = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dma_channel_control_register() {
        let mut channel = DMAChannel::new();
        
        // Set control register with all flags set
        channel.set_control(0xFF);
        assert_eq!(channel.mode, 0x07);
        assert!(channel.direction);
        assert!(channel.indirect);
        assert!(channel.reverse);
        assert!(channel.fixed);
        assert!(channel.enabled);
        
        // Verify round-trip
        assert_eq!(channel.get_control(), 0xFF);
    }

    #[test]
    fn test_dma_address_registers() {
        let mut controller = DMAController::new();
        
        // Write 24-bit address across 3 registers
        controller.write_register(0x4302, 0x34); // Low byte
        controller.write_register(0x4303, 0x12); // Mid byte
        controller.write_register(0x4304, 0x80); // High byte
        
        assert_eq!(controller.channels[0].source_addr, 0x801234);
    }

    #[test]
    fn test_dma_transfer_size() {
        let mut controller = DMAController::new();
        
        // Write 16-bit transfer size
        controller.write_register(0x4305, 0x00);
        controller.write_register(0x4306, 0x10);
        
        assert_eq!(controller.channels[0].transfer_size, 0x1000);
    }
}
