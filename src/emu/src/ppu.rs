// PPU Register Control
#[derive(Clone, Copy)]
pub struct PPUControl {
    pub forced_blank: bool,         // Forced blank (bit 7)
    pub brightness: u8,             // Brightness (bits 0-3)
    pub obj_size: u8,              // Object size (bits 0-2)
    pub obj_addr: u16,             // Object address (bits 0-1, 3-4)
}

impl PPUControl {
    fn from_byte(val: u8) -> Self {
        PPUControl {
            forced_blank: (val & 0x80) != 0,
            brightness: val & 0x0F,
            obj_size: (val & 0x70) >> 4,
            obj_addr: (((val as u16) & 0x03) << 8),
        }
    }
}

#[derive(Clone, Copy)]
pub struct BGControl {
    pub tile_size: bool,            // 0 = 8x8, 1 = 16x16
    pub map_addr: u16,              // Map address (2K unit)
    pub tile_addr: u16,             // Tile address (4K unit)
    pub mosaic: bool,               // Mosaic enable
}

#[derive(Clone, Copy)]
pub struct BGOffset {
    pub x: u16,
    pub y: u16,
}

pub struct PPU {
    // Video Memory
    pub vram: [u8; 65536],          // Video RAM (64KB)
    pub oam: [u8; 544],              // Object Attribute Memory (544 bytes)
    pub cgram: [u8; 512],            // Color Generator RAM (512 bytes, 256 colors)
    
    // PPU Control Registers
    pub control: PPUControl,
    pub bg_control: [BGControl; 4],  // Background 0-3 control
    pub bg_offset: [BGOffset; 4],    // Background 0-3 offset
    
    // PPU Status & Timing
    pub scanline: u16,               // Current scanline (0-261 for NTSC)
    pub cycle: u16,                  // PPU cycle within scanline (0-1363)
    pub field: bool,                 // Field (interlace)
    pub vblank: bool,                // V-blank flag
    pub hblank: bool,                // H-blank flag
    
    // Framebuffer (256x224 NTSC) - heap-allocated to avoid stack overflow
    pub framebuffer: Box<[u32; 256 * 224]>, // 32-bit ARGB pixels
    
    // VRAM Address Pointer
    pub vram_addr: u16,              // Current VRAM address
    pub vram_addr_high: bool,        // Address increment select
    
    // Temporary registers
    pub temp_x: u16,
    pub temp_y: u16,
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            vram: [0; 65536],
            oam: [0; 544],
            cgram: [0; 512],
            
            control: PPUControl {
                forced_blank: true,
                brightness: 0,
                obj_size: 0,
                obj_addr: 0,
            },
            
            bg_control: [
                BGControl { tile_size: false, map_addr: 0, tile_addr: 0, mosaic: false };
                4
            ],
            
            bg_offset: [
                BGOffset { x: 0, y: 0 };
                4
            ],
            
            scanline: 0,
            cycle: 0,
            field: false,
            vblank: false,
            hblank: false,
            
            framebuffer: Box::new([0; 256 * 224]),
            
            vram_addr: 0,
            vram_addr_high: false,
            
            temp_x: 0,
            temp_y: 0,
        }
    }
    
    // --- VRAM Access ---
    pub fn vram_read(&self) -> u8 {
        let addr = (self.vram_addr as usize) & 0xFFFF;
        self.vram[addr]
    }
    
    pub fn vram_write(&mut self, data: u8) {
        let addr = (self.vram_addr as usize) & 0xFFFF;
        self.vram[addr] = data;
    }
    
    pub fn vram_addr_inc(&mut self) {
        let increment = if self.vram_addr_high { 256 } else { 1 };
        self.vram_addr = self.vram_addr.wrapping_add(increment);
    }
    
    // --- CGRAM (Palette) Access ---
    pub fn cgram_read(&self, addr: u16) -> u8 {
        let addr = (addr as usize) & 0x1FF;
        self.cgram[addr]
    }
    
    pub fn cgram_write(&mut self, addr: u16, data: u8) {
        let addr = (addr as usize) & 0x1FF;
        self.cgram[addr] = data;
    }
    
    // --- Color Conversion (555 RGB → 32-bit ARGB) ---
    pub fn get_color(&self, palette_idx: u16) -> u32 {
        // Each palette entry is 2 bytes (555 RGB)
        let addr = ((palette_idx & 0xFF) * 2) as usize;
        let low = self.cgram[addr] as u16;
        let high = self.cgram[addr + 1] as u16;
        
        let color_555 = ((high as u16) << 8) | (low as u16);
        
        // Convert 555 RGB to 32-bit ARGB
        let r = ((color_555 & 0x7C00) >> 10) as u32;
        let g = ((color_555 & 0x03E0) >> 5) as u32;
        let b = (color_555 & 0x001F) as u32;
        
        // Scale from 5-bit to 8-bit
        let r = (r << 3) | (r >> 2);
        let g = (g << 3) | (g >> 2);
        let b = (b << 3) | (b >> 2);
        
        // Return as ARGB
        0xFF000000 | (r << 16) | (g << 8) | b
    }
    
    // --- OAM (Sprite) Access ---
    pub fn oam_read(&self, addr: u16) -> u8 {
        self.oam[(addr as usize) & 0x1FF]
    }
    
    pub fn oam_write(&mut self, addr: u16, data: u8) {
        self.oam[(addr as usize) & 0x1FF] = data;
    }
    
    // --- Background Layer Reading ---
    fn get_bg_tile(&self, bg: u8, map_x: u16, map_y: u16) -> (u8, bool, bool, u8) {
        // Get background control
        let bg_ctrl = self.bg_control[bg as usize];
        
        // Map size is 32x32 tiles
        let map_x = map_x & 0x1F;
        let map_y = map_y & 0x1F;
        
        // Tile map address in VRAM
        let map_addr = bg_ctrl.map_addr + (map_y << 5) + map_x;
        
        // Read tile entry from VRAM
        let tile_entry_low = self.vram[(map_addr as usize) & 0xFFFF];
        let tile_entry_high = self.vram[((map_addr + 1) as usize) & 0xFFFF];
        
        let tile_num = ((tile_entry_high as u16 & 0x03) << 8) | (tile_entry_low as u16);
        let palette = (tile_entry_high >> 2) & 0x07;
        let _priority = (tile_entry_high >> 5) & 0x01;
        let flip_h = (tile_entry_high & 0x40) != 0;
        let flip_v = (tile_entry_high & 0x80) != 0;
        
        (tile_num as u8, flip_h, flip_v, palette)
    }
    
    fn get_bg_pixel(&self, bg: u8, pix_x: u16, pix_y: u16) -> (u8, bool) {
        let bg_ctrl = self.bg_control[bg as usize];
        
        // 8x8 tiles
        let tile_x = (pix_x >> 3) & 0x1F;
        let tile_y = (pix_y >> 3) & 0x1F;
        let in_tile_x = pix_x & 0x07;
        let in_tile_y = pix_y & 0x07;
        
        let (tile_num, flip_h, flip_v, palette) = self.get_bg_tile(bg, tile_x, tile_y);
        
        // Tile data is in VRAM (2bpp for mode 0)
        let tile_addr = bg_ctrl.tile_addr + (tile_num as u16 * 16);
        
        let mut pix_x_in_tile = in_tile_x;
        let mut pix_y_in_tile = in_tile_y;
        
        if flip_h { pix_x_in_tile = 7 - pix_x_in_tile; }
        if flip_v { pix_y_in_tile = 7 - pix_y_in_tile; }
        
        // For 2bpp, 2 pixels per byte
        let byte_offset = pix_y_in_tile * 2 + (pix_x_in_tile >> 2);
        let bit_offset = (pix_x_in_tile & 0x03) << 1;
        
        let tile_data = self.vram[((tile_addr + byte_offset) as usize) & 0xFFFF];
        let pixel = (tile_data >> bit_offset) & 0x03;
        
        // Palette index (palette * 4 + pixel)
        let palette_idx = ((palette as u16) << 2) | (pixel as u16);
        (palette_idx as u8, pixel != 0) // Return (palette index, is_opaque)
    }
    
    // --- Main PPU Tick (Per Cycle) ---
    pub fn tick(&mut self) {
        // Handle H-blank and V-blank timing
        if self.cycle == 0 {
            // Start of scanline
            self.hblank = false;
        } else if self.cycle == 1096 {
            // H-blank starts
            self.hblank = true;
        }
        
        // Advance cycle
        self.cycle += 1;
        if self.cycle >= 1364 {
            self.cycle = 0;
            self.scanline += 1;
            
            // V-blank period: scanlines 225-261
            if self.scanline == 225 {
                self.vblank = true;
                // NMI would be triggered here
            } else if self.scanline == 262 {
                self.scanline = 0;
                self.vblank = false;
                self.field = !self.field;
            }
        }
    }
    
    // --- Render a Scanline (Called during H-blank) ---
    pub fn render_scanline(&mut self) {
        if self.control.forced_blank || self.scanline >= 224 {
            // Blank scanline
            for x in 0..256 {
                self.framebuffer[(self.scanline as usize * 256) + x] = 0xFF000000; // Black
            }
            return;
        }
        
        let y = self.scanline;
        
        // Render Mode 0: 4 background layers, 2bpp each
        for x in 0..256 {
            let mut color = 0xFF000000; // Default: black
            
            // Layer priority (simplified - just draw top layer)
            for bg in 0..4 {
                let bg_y = (y + self.bg_offset[bg as usize].y) & 0xFFFF;
                let bg_x = (x as u16 + self.bg_offset[bg as usize].x) & 0xFFFF;
                
                let (palette_idx, is_opaque) = self.get_bg_pixel(bg as u8, bg_x, bg_y);
                
                if is_opaque {
                    color = self.get_color(palette_idx as u16);
                    break; // Use first opaque layer
                }
            }
            
            self.framebuffer[(y as usize * 256) + x] = color;
        }
    }
    
    // --- Register Read/Write (from CPU) ---
    pub fn read_register(&self, addr: u8) -> u8 {
        match addr {
            0x34 => {
                // PPU status
                let mut status = 0u8;
                if self.vblank { status |= 0x80; }
                if self.hblank { status |= 0x40; }
                status
            }
            0x39 => self.vram_read(), // VMDATAL
            0x3A => self.vram_read(), // VMDATAH
            _ => 0,
        }
    }
    
    pub fn write_register(&mut self, addr: u8, data: u8) {
        match addr {
            0x00 => {
                // INIDISP - Display control
                self.control = PPUControl::from_byte(data);
            }
            0x05 => {
                // BGXOFS - BG offset X
                // Which BG is determined by register sequence
                self.temp_x = ((data as u16) << 8) | (self.temp_x & 0xFF);
            }
            0x06 => {
                // BGXOFS - BG offset Y
                self.temp_y = ((data as u16) << 8) | (self.temp_y & 0xFF);
            }
            0x15 => {
                // VMADD - VRAM address low byte
                self.vram_addr = (self.vram_addr & 0xFF00) | (data as u16);
            }
            0x16 => {
                // VMADD - VRAM address high byte
                self.vram_addr = (self.vram_addr & 0x00FF) | (((data as u16) & 0x7F) << 8);
                self.vram_addr_high = (data & 0x80) != 0;
            }
            0x18 => {
                // VMDATAL - VRAM data write
                self.vram_write(data);
                self.vram_addr_inc();
            }
            0x19 => {
                // VMDATAH - VRAM data write high
                self.vram_write(data);
                self.vram_addr_inc();
            }
            0x21 => {
                // OBJSEL - OAM size/address
                self.control.obj_size = (data >> 5) & 0x07;
                self.control.obj_addr = ((data as u16) & 0x03) << 8;
            }
            0x22 => {
                // OAMADDL - OAM address low
                // OAM write would happen here
            }
            0x23 => {
                // OAMADDH - OAM address high
                // OAM write would happen here
            }
            _ => {}
        }
    }
}