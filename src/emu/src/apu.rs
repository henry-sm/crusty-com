// Voice Channel State
#[derive(Clone, Copy, Debug)]
pub struct Voice {
    pub left_vol: i8,           // Left volume (-128..127)
    pub right_vol: i8,          // Right volume (-128..127)
    pub pitch: u16,             // Pitch (0x0000-0xFFFF)
    pub srcn: u8,               // Sample number
    pub adsr1: u8,              // ADSR1 register
    pub adsr2: u8,              // ADSR2 register
    pub gain: u8,               // Gain register
    pub envx: u16,              // Envelope value
    pub outx: i8,               // Last output value
    pub enabled: bool,          // Voice enabled
    pub current_sample_idx: u16, // Position in current sample
    pub fraction: u16,          // Fractional sample position
    pub brr_header: u8,         // Current BRR block header
    pub brr_buffer: [i16; 16],  // Decoded BRR samples
    pub prev_sample: i16,       // Previous sample for interpolation
}

impl Voice {
    pub fn new() -> Self {
        Voice {
            left_vol: 0,
            right_vol: 0,
            pitch: 0,
            srcn: 0,
            adsr1: 0,
            adsr2: 0,
            gain: 0,
            envx: 0,
            outx: 0,
            enabled: false,
            current_sample_idx: 0,
            fraction: 0,
            brr_header: 0,
            brr_buffer: [0; 16],
            prev_sample: 0,
        }
    }

    /// Decode a BRR block (16 bytes) into 16 PCM samples
    /// BRR (Bit Rate Reduction) is the SNES sample compression format
    pub fn decode_brr_block(&mut self, data: &[u8]) {
        if data.len() < 9 {
            return;
        }

        self.brr_header = data[0];
        let filter = (self.brr_header >> 2) & 0x03;
        let shift = (self.brr_header >> 4) & 0x0F;

        // Decode 16 4-bit nibbles into 16 samples
        for i in 0..16 {
            let byte_idx = 1 + i / 2;
            let nibble = if i % 2 == 0 {
                (data[byte_idx] >> 4) & 0x0F
            } else {
                data[byte_idx] & 0x0F
            };

            // Sign-extend 4-bit nibble to i16
            let signed: i16 = if (nibble & 0x08) != 0 {
                (((nibble as i16) | (-16i16)) << 1)
            } else {
                (nibble as i16) << 1
            };

            // Apply shift
            let shifted = signed << shift;

            // Apply filter (prediction)
            let sample = match filter {
                0 => shifted,
                1 => {
                    // Simple delta-PCM
                    shifted + (self.brr_buffer[(i as i32 - 1).max(0) as usize] >> 1)
                }
                2 => {
                    // Two-tap filter
                    let s1 = self.brr_buffer[(i as i32 - 1).max(0) as usize];
                    let s2 = self.brr_buffer[(i as i32 - 2).max(0) as usize];
                    shifted + (s1 * 61 >> 6) - (s2 * 15 >> 4)
                }
                3 => {
                    // Three-tap filter
                    let s1 = self.brr_buffer[(i as i32 - 1).max(0) as usize];
                    let s2 = self.brr_buffer[(i as i32 - 2).max(0) as usize];
                    shifted + (s1 * 115 >> 7) - (s2 * 13 >> 4)
                }
                _ => shifted,
            };

            // Clamp to i16 range
            let clamped = sample.max(-32768).min(32767);
            self.brr_buffer[i] = clamped;
        }
    }

    // Update ADSR envelope
    pub fn update_envelope(&mut self) {
        if self.adsr1 & 0x80 != 0 {
            // ADSR mode
            let ar = (self.adsr1 >> 4) & 0x0F;  // Attack rate
            let dr = (self.adsr1 >> 0) & 0x0F;  // Decay rate
            let sl = (self.adsr2 >> 5) & 0x07;  // Sustain level
            let sr = (self.adsr2 >> 0) & 0x1F;  // Sustain rate
            
            // Simple envelope: ramp up on attack, down on decay, hold at sustain
            if self.envx < 0x7FFF {
                let rate = if self.envx < ((sl as u16) << 11) {
                    ar + 1
                } else if self.envx > ((sl as u16) << 11) {
                    dr + 1
                } else {
                    sr + 1
                };
                self.envx = self.envx.saturating_add(rate as u16 * 8);
            }
        } else {
            // Gain mode - direct envelope value
            self.envx = (self.gain & 0x7F) as u16 * 0x80;
        }
    }
}


// SPC700 CPU State
#[derive(Clone)]
pub struct SPC700 {
    // Registers
    pub a: u8,          // Accumulator
    pub x: u8,          // X register
    pub y: u8,          // Y register
    pub sp: u8,         // Stack pointer
    pub pc: u16,        // Program counter
    
    // Flags: NV-B DIZE
    pub n: bool,        // Negative flag
    pub v: bool,        // Overflow flag
    pub p: bool,        // Direct page flag
    pub b: bool,        // Break flag
    pub d: bool,        // Decimal mode flag
    pub i: bool,        // IRQ disable flag
    pub z: bool,        // Zero flag
    pub c: bool,        // Carry flag
}

impl SPC700 {
    pub fn new() -> Self {
        SPC700 {
            a: 0, x: 0, y: 0, sp: 0xEF, pc: 0xFFC0,
            n: false, v: false, p: false, b: true, 
            d: false, i: true, z: false, c: false,
        }
    }
    
    // Get processor status register
    pub fn get_psw(&self) -> u8 {
        ((self.n as u8) << 7)
            | ((self.v as u8) << 6)
            | ((self.p as u8) << 5)
            | ((self.b as u8) << 4)
            | ((self.d as u8) << 3)
            | ((self.i as u8) << 2)
            | ((self.z as u8) << 1)
            | (self.c as u8)
    }
    
    // Set processor status register
    pub fn set_psw(&mut self, val: u8) {
        self.n = (val & 0x80) != 0;
        self.v = (val & 0x40) != 0;
        self.p = (val & 0x20) != 0;
        self.b = (val & 0x10) != 0;
        self.d = (val & 0x08) != 0;
        self.i = (val & 0x04) != 0;
        self.z = (val & 0x02) != 0;
        self.c = (val & 0x01) != 0;
    }
}

// DSP State
#[derive(Clone)]
pub struct DSP {
    pub registers: [u8; 128],       // DSP registers (0x00-0x7F)
    pub voices: [Voice; 8],         // 8 voice channels
    pub master_vol_left: i8,        // Main volume left
    pub master_vol_right: i8,       // Main volume right
    pub echo_vol_left: i8,          // Echo volume left
    pub echo_vol_right: i8,         // Echo volume right
    pub echo_buffer: Vec<i16>,      // Echo/reverb buffer (up to 15.5 KB)
    pub echo_buffer_pos: usize,     // Current position in echo buffer
    pub echo_enabled: bool,         // Echo enable flag
    pub noise_counter: u16,         // Noise generator counter
    pub output_buffer: Vec<i16>,    // Audio output buffer (stereo: L, R, L, R, ...)
    pub sample_rate: u32,           // Sample rate (typically 32000 Hz for SPC700 at 2.048 MHz)
}

impl DSP {
    pub fn new() -> Self {
        DSP {
            registers: [0; 128],
            voices: [Voice::new(); 8],
            master_vol_left: 0,
            master_vol_right: 0,
            echo_vol_left: 0,
            echo_vol_right: 0,
            echo_buffer: vec![0; 8192],  // 15.5 KB echo buffer
            echo_buffer_pos: 0,
            echo_enabled: false,
            noise_counter: 0,
            output_buffer: Vec::with_capacity(32000),  // ~1 second at 32kHz
            sample_rate: 32000,
        }
    }
    
    pub fn read_register(&self, addr: u8) -> u8 {
        match addr & 0x7F {
            // Voice registers: 00-6F (8 voices, 16 registers each)
            0x00..=0x6F => {
                let voice_idx = addr as usize / 16;
                let reg = addr as usize % 16;
                match reg {
                    0x0 => self.voices[voice_idx].left_vol as u8,
                    0x1 => self.voices[voice_idx].right_vol as u8,
                    0x2 => (self.voices[voice_idx].pitch & 0xFF) as u8,
                    0x3 => ((self.voices[voice_idx].pitch >> 8) & 0x3F) as u8,
                    0x4 => self.voices[voice_idx].srcn,
                    0x5 => self.voices[voice_idx].adsr1,
                    0x6 => self.voices[voice_idx].adsr2,
                    0x7 => self.voices[voice_idx].gain,
                    0x8 => self.voices[voice_idx].envx as u8,
                    0x9 => self.voices[voice_idx].outx as u8,
                    _ => self.registers[(addr & 0x7F) as usize],
                }
            }
            // Master/misc registers: 70-7F
            0x70 => 0,  // Read-only test register
            0x71 => 0,  // Control register (read-only)
            0x72 => 0,  // Echo feedback
            0x73 => 0,  // Pitch modulation on
            0x74 => 0,  // Noise on
            0x75 => 0,  // Echo on
            0x76 => 0,  // Mute and echo disabled
            0x77 => 0,  // Noise clock
            0x78 => 0,  // Echo delay
            0x79 => 0,  // Echo filter coefficient
            _ => self.registers[(addr & 0x7F) as usize],
        }
    }
    
    pub fn write_register(&mut self, addr: u8, val: u8) {
        match addr & 0x7F {
            // Voice registers: 00-6F (8 voices, 16 registers each)
            0x00..=0x6F => {
                let voice_idx = addr as usize / 16;
                let reg = addr as usize % 16;
                match reg {
                    0x0 => self.voices[voice_idx].left_vol = val as i8,
                    0x1 => self.voices[voice_idx].right_vol = val as i8,
                    0x2 => self.voices[voice_idx].pitch = (self.voices[voice_idx].pitch & 0xFF00) | (val as u16),
                    0x3 => self.voices[voice_idx].pitch = (self.voices[voice_idx].pitch & 0x00FF) | (((val & 0x3F) as u16) << 8),
                    0x4 => self.voices[voice_idx].srcn = val,
                    0x5 => self.voices[voice_idx].adsr1 = val,
                    0x6 => self.voices[voice_idx].adsr2 = val,
                    0x7 => self.voices[voice_idx].gain = val,
                    0x8 => self.voices[voice_idx].envx = (val as u16) << 8,
                    0x9 => self.voices[voice_idx].outx = val as i8,
                    _ => self.registers[(addr & 0x7F) as usize] = val,
                }
            }
            // Master/misc registers: 70-7F
            0x70 => {
                // Test register - can reset various things
                // Not typically written by games
            }
            0x71 => {
                // Control register
                // Bit 7: Init (1 = reset DSP)
                // Bit 6-4: ROM disable
                // Bit 3: Clear echo buffer
                // Bit 2: Mute
                // Bit 1: Echo disabled
                // Bit 0: Sound disabled
            }
            0x72 => {
                // Echo feedback (unused in basic implementation)
            }
            0x73 => {
                // Pitch modulation enable (8 bits, one per voice)
            }
            0x74 => {
                // Noise on (8 bits, one per voice)
            }
            0x75 => {
                // Echo on (8 bits, one per voice)
                self.echo_enabled = (val & 0xFF) != 0;
            }
            0x76 => {
                // Mute and echo disabled
            }
            0x77 => {
                // Noise clock
            }
            0x78 => {
                // Echo delay (in 2048-sample units)
            }
            0x79 => {
                // Echo filter coefficient
            }
            0x7A..=0x7F => {
                // Echo filter coefficients
                self.registers[(addr & 0x7F) as usize] = val;
            }
            _ => {}
        }
    }
}

pub struct APU {
    pub spc700: SPC700,
    pub dsp: DSP,
    pub ram: [u8; 65536],           // 64KB APU RAM
    pub cycles: u32,                // Cycle counter for sync
    pub sample_counter: u32,        // Counter for sample generation
    pub samples_per_update: u32,    // How many CPU cycles per audio sample
}

impl APU {
    pub fn new() -> Self {
        APU {
            spc700: SPC700::new(),
            dsp: DSP::new(),
            ram: [0; 65536],
            cycles: 0,
            sample_counter: 0,
            samples_per_update: 64,  // SPC700 runs at ~2.048 MHz, output at ~32 kHz = 64 cycles per sample
        }
    }
    
    pub fn tick(&mut self) {
        // Execute one SPC700 instruction
        self.execute_instruction();
        
        // Accumulate cycles
        self.cycles += 1;
        self.sample_counter += 1;
        
        // Generate audio sample every N cycles
        if self.sample_counter >= self.samples_per_update {
            self.sample_counter = 0;
            self.dsp_update();
        }
    }
    
    /// Generate audio samples from current voice states
    fn dsp_update(&mut self) {
        let mut left_out: i32 = 0;
        let mut right_out: i32 = 0;
        
        // Process all 8 voices
        for voice_idx in 0..8 {
            let voice = &mut self.dsp.voices[voice_idx];
            
            if !voice.enabled || voice.envx == 0 {
                continue;
            }
            
            // Update envelope
            voice.update_envelope();
            
            // Get sample from voice's current position
            let sample = voice.brr_buffer[0];  // Simplified - would interpolate normally
            
            // Apply envelope and volume
            let envelope_level = (voice.envx >> 8) as i32;  // 0-255
            let left_level = (voice.left_vol as i32) * envelope_level / 255;
            let right_level = (voice.right_vol as i32) * envelope_level / 255;
            
            left_out += (sample as i32) * left_level / 128;
            right_out += (sample as i32) * right_level / 128;
        }
        
        // Apply master volume
        left_out = left_out * (self.dsp.master_vol_left as i32) / 128;
        right_out = right_out * (self.dsp.master_vol_right as i32) / 128;
        
        // Clamp to i16 range
        left_out = left_out.max(-32768).min(32767);
        right_out = right_out.max(-32768).min(32767);
        
        // Add to output buffer
        self.dsp.output_buffer.push(left_out as i16);
        self.dsp.output_buffer.push(right_out as i16);
        
        // Keep buffer reasonably sized (about 1 second of audio)
        if self.dsp.output_buffer.len() > self.dsp.sample_rate as usize * 2 {
            self.dsp.output_buffer.drain(0..std::cmp::min(1024, self.dsp.output_buffer.len() / 2));
        }
    }
    
    /// Get audio samples from buffer (and remove them)
    pub fn get_audio_samples(&mut self) -> Vec<i16> {
        let samples = self.dsp.output_buffer.clone();
        self.dsp.output_buffer.clear();
        samples
    }
    
    /// Return audio buffer size
    pub fn audio_buffer_len(&self) -> usize {
        self.dsp.output_buffer.len()
    }
    
    fn execute_instruction(&mut self) {
        // Fetch opcode from APU RAM
        let opcode = self.ram[self.spc700.pc as usize];
        
        // Decode and execute - SPC700 instruction set
        match opcode {
            // NOP - 0xEA
            0xEA => { 
                self.spc700.pc = self.spc700.pc.wrapping_add(1); 
            }
            
            // MOV A, #imm - 0xE8
            0xE8 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let imm = self.ram[self.spc700.pc as usize];
                self.spc700.a = imm;
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // MOV X, #imm - 0xCD
            0xCD => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let imm = self.ram[self.spc700.pc as usize];
                self.spc700.x = imm;
                self.update_nz(self.spc700.x);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // MOV Y, #imm - 0xCD (fixed to avoid conflicts)
            0xCF => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let imm = self.ram[self.spc700.pc as usize];
                self.spc700.y = imm;
                self.update_nz(self.spc700.y);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // MOV A, X - 0x7D
            0x7D => {
                self.spc700.a = self.spc700.x;
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // MOV A, Y - 0xDD
            0xDD => {
                self.spc700.a = self.spc700.y;
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // MOV X, A - 0x5D
            0x5D => {
                self.spc700.x = self.spc700.a;
                self.update_nz(self.spc700.x);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // MOV Y, A - 0xFD
            0xFD => {
                self.spc700.y = self.spc700.a;
                self.update_nz(self.spc700.y);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // MOV A, (X) - indirect - 0xC6
            0xC6 => {
                let addr = self.spc700.x as u16;
                self.spc700.a = self.ram[addr as usize];
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // MOV (X), A - indirect - 0xC5
            0xC5 => {
                let addr = self.spc700.x as u16;
                self.ram[addr as usize] = self.spc700.a;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // MOV A, $addr - direct - 0xE4
            0xE4 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let addr = self.ram[self.spc700.pc as usize] as u16;
                self.spc700.a = self.ram[addr as usize];
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // MOV $addr, A - direct - 0xC4
            0xC4 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let addr = self.ram[self.spc700.pc as usize] as u16;
                self.ram[addr as usize] = self.spc700.a;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // MOV $addr, X - direct - 0xD4
            0xD4 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let addr = self.ram[self.spc700.pc as usize] as u16;
                self.ram[addr as usize] = self.spc700.x;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // MOV $addr, Y - direct - 0xD8
            0xD8 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let addr = self.ram[self.spc700.pc as usize] as u16;
                self.ram[addr as usize] = self.spc700.y;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // INC Y - 0xDB (fixed opcode)
            0xDB => {
                self.spc700.y = self.spc700.y.wrapping_add(1);
                self.update_nz(self.spc700.y);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            0xF4 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let addr = self.ram[self.spc700.pc as usize] as u16;
                self.spc700.x = self.ram[addr as usize];
                self.update_nz(self.spc700.x);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // MOV Y, $addr - direct - 0xEB
            0xEB => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let addr = self.ram[self.spc700.pc as usize] as u16;
                self.spc700.y = self.ram[addr as usize];
                self.update_nz(self.spc700.y);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // INC A - 0xBC
            0xBC => {
                self.spc700.a = self.spc700.a.wrapping_add(1);
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // DEC A - 0x9C
            0x9C => {
                self.spc700.a = self.spc700.a.wrapping_sub(1);
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // INC X - 0x3D
            0x3D => {
                self.spc700.x = self.spc700.x.wrapping_add(1);
                self.update_nz(self.spc700.x);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // DEC X - 0x1D
            0x1D => {
                self.spc700.x = self.spc700.x.wrapping_sub(1);
                self.update_nz(self.spc700.x);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // ADD A, #imm - 0x84
            0x84 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let imm = self.ram[self.spc700.pc as usize];
                let result = (self.spc700.a as u16).wrapping_add(imm as u16);
                self.spc700.c = result > 0xFF;
                self.spc700.v = ((self.spc700.a ^ imm) & 0x80) == 0 && 
                                 ((self.spc700.a ^ result as u8) & 0x80) != 0;
                self.spc700.a = result as u8;
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // ADC A, #imm (add with carry) - 0x94
            0x94 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let imm = self.ram[self.spc700.pc as usize];
                let carry = if self.spc700.c { 1 } else { 0 };
                let result = (self.spc700.a as u16).wrapping_add(imm as u16).wrapping_add(carry);
                self.spc700.c = result > 0xFF;
                self.spc700.v = ((self.spc700.a ^ imm) & 0x80) == 0 && 
                                 ((self.spc700.a ^ result as u8) & 0x80) != 0;
                self.spc700.a = result as u8;
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // SUB A, #imm - 0x64
            0x64 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let imm = self.ram[self.spc700.pc as usize];
                let result = (self.spc700.a as i16) - (imm as i16);
                self.spc700.c = result >= 0;
                self.spc700.v = ((self.spc700.a ^ imm) & 0x80) != 0 && 
                                 ((self.spc700.a ^ (result as u8)) & 0x80) != 0;
                self.spc700.a = result as u8;
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // SBC A, #imm (subtract with carry/borrow) - 0xA4
            0xA4 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let imm = self.ram[self.spc700.pc as usize];
                let borrow = if !self.spc700.c { 1 } else { 0 };
                let result = (self.spc700.a as i16) - (imm as i16) - borrow as i16;
                self.spc700.c = result >= 0;
                self.spc700.v = ((self.spc700.a ^ imm) & 0x80) != 0 && 
                                 ((self.spc700.a ^ (result as u8)) & 0x80) != 0;
                self.spc700.a = result as u8;
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // AND A, #imm - 0x24
            0x24 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let imm = self.ram[self.spc700.pc as usize];
                self.spc700.a &= imm;
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // AND A, $addr - 0x24 (direct mode)
            0x34 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let addr = self.ram[self.spc700.pc as usize] as u16;
                let val = self.ram[addr as usize];
                self.spc700.a &= val;
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // OR A, #imm - 0x04
            0x04 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let imm = self.ram[self.spc700.pc as usize];
                self.spc700.a |= imm;
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // OR A, $addr - 0x14 (direct mode)
            0x14 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let addr = self.ram[self.spc700.pc as usize] as u16;
                let val = self.ram[addr as usize];
                self.spc700.a |= val;
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // EOR A, #imm (XOR) - 0x44
            0x44 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let imm = self.ram[self.spc700.pc as usize];
                self.spc700.a ^= imm;
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // EOR A, $addr - 0x54 (direct mode XOR)
            0x54 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let addr = self.ram[self.spc700.pc as usize] as u16;
                let val = self.ram[addr as usize];
                self.spc700.a ^= val;
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // CMP A, #imm - 0xC8
            0xC8 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let imm = self.ram[self.spc700.pc as usize];
                let result = (self.spc700.a as i16) - (imm as i16);
                self.spc700.c = result >= 0;
                self.update_nz(result as u8);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // CMP X, #imm - 0xAD
            0xAD => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let imm = self.ram[self.spc700.pc as usize];
                let result = (self.spc700.x as i16) - (imm as i16);
                self.spc700.c = result >= 0;
                self.update_nz(result as u8);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // CMP Y, #imm - 0x8D (conflicts - using 0xCD for CMP Y)
            0xEC => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let imm = self.ram[self.spc700.pc as usize];
                let result = (self.spc700.y as i16) - (imm as i16);
                self.spc700.c = result >= 0;
                self.update_nz(result as u8);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // ASL A (arithmetic shift left) - 0x1C
            0x1C => {
                self.spc700.c = (self.spc700.a & 0x80) != 0;
                self.spc700.a = self.spc700.a.wrapping_shl(1);
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // LSR A (logical shift right) - 0x5C
            0x5C => {
                self.spc700.c = (self.spc700.a & 0x01) != 0;
                self.spc700.a = self.spc700.a >> 1;
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // ROL A (rotate left through carry) - 0x3C
            0x3C => {
                let new_c = (self.spc700.a & 0x80) != 0;
                self.spc700.a = self.spc700.a.wrapping_shl(1);
                if self.spc700.c {
                    self.spc700.a |= 0x01;
                }
                self.spc700.c = new_c;
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // ROR A (rotate right through carry) - 0x7C
            0x7C => {
                let new_c = (self.spc700.a & 0x01) != 0;
                self.spc700.a = self.spc700.a >> 1;
                if self.spc700.c {
                    self.spc700.a |= 0x80;
                }
                self.spc700.c = new_c;
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // JMP addr - 0x5F
            0x5F => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let lo = self.ram[self.spc700.pc as usize] as u16;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let hi = self.ram[self.spc700.pc as usize] as u16;
                self.spc700.pc = (hi << 8) | lo;
            }
            
            // JMP (addr) - indirect - 0x6F (conflicts with RET, using 0x7F)
            0x7F => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let addr_lo = self.ram[self.spc700.pc as usize] as u16;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let addr_hi = self.ram[self.spc700.pc as usize] as u16;
                let addr = (addr_hi << 8) | addr_lo;
                let lo = self.ram[addr as usize] as u16;
                let hi = self.ram[(addr + 1) as usize] as u16;
                self.spc700.pc = (hi << 8) | lo;
            }
            
            // CALL addr - 0x3F (call subroutine)
            0x3F => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let lo = self.ram[self.spc700.pc as usize] as u16;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let hi = self.ram[self.spc700.pc as usize] as u16;
                let ret_addr = self.spc700.pc;
                // Push return address
                self.ram[0x100 | self.spc700.sp as usize] = (ret_addr >> 8) as u8;
                self.spc700.sp = self.spc700.sp.wrapping_sub(1);
                self.ram[0x100 | self.spc700.sp as usize] = ret_addr as u8;
                self.spc700.sp = self.spc700.sp.wrapping_sub(1);
                self.spc700.pc = (hi << 8) | lo;
            }
            
            // BRA offset - 0x2F (branch always)
            0x2F => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let offset = self.ram[self.spc700.pc as usize] as i8;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                self.spc700.pc = (self.spc700.pc as i16 + offset as i16) as u16;
            }
            
            // BEQ offset - 0xF0 (branch if equal/zero)
            0xF0 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let offset = self.ram[self.spc700.pc as usize] as i8;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                if self.spc700.z {
                    self.spc700.pc = (self.spc700.pc as i16 + offset as i16) as u16;
                }
            }
            
            // BNE offset - 0xD0 (branch if not equal)
            0xD0 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let offset = self.ram[self.spc700.pc as usize] as i8;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                if !self.spc700.z {
                    self.spc700.pc = (self.spc700.pc as i16 + offset as i16) as u16;
                }
            }
            
            // BCS offset - 0xB0 (branch if carry set)
            0xB0 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let offset = self.ram[self.spc700.pc as usize] as i8;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                if self.spc700.c {
                    self.spc700.pc = (self.spc700.pc as i16 + offset as i16) as u16;
                }
            }
            
            // BCC offset - 0x90 (branch if carry clear)
            0x90 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let offset = self.ram[self.spc700.pc as usize] as i8;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                if !self.spc700.c {
                    self.spc700.pc = (self.spc700.pc as i16 + offset as i16) as u16;
                }
            }
            
            // BMI offset - 0x30 (branch if minus/negative)
            0x30 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let offset = self.ram[self.spc700.pc as usize] as i8;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                if self.spc700.n {
                    self.spc700.pc = (self.spc700.pc as i16 + offset as i16) as u16;
                }
            }
            
            // BPL offset - 0x10 (branch if plus/positive)
            0x10 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let offset = self.ram[self.spc700.pc as usize] as i8;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                if !self.spc700.n {
                    self.spc700.pc = (self.spc700.pc as i16 + offset as i16) as u16;
                }
            }
            
            // BVS offset - 0x70 (branch if overflow set)
            0x70 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let offset = self.ram[self.spc700.pc as usize] as i8;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                if self.spc700.v {
                    self.spc700.pc = (self.spc700.pc as i16 + offset as i16) as u16;
                }
            }
            
            // BVC offset - 0x50 (branch if overflow clear)
            0x50 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let offset = self.ram[self.spc700.pc as usize] as i8;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                if !self.spc700.v {
                    self.spc700.pc = (self.spc700.pc as i16 + offset as i16) as u16;
                }
            }
            
            // RET - 0x6F
            0x6F => {
                let lo = self.ram[0x100 | self.spc700.sp as usize] as u16;
                self.spc700.sp = self.spc700.sp.wrapping_add(1);
                let hi = self.ram[0x100 | self.spc700.sp as usize] as u16;
                self.spc700.sp = self.spc700.sp.wrapping_add(1);
                self.spc700.pc = (hi << 8) | lo;
            }
            
            // RETI - 0x8F (return from interrupt)
            0x8F => {
                let psw = self.ram[0x100 | self.spc700.sp as usize];
                self.spc700.sp = self.spc700.sp.wrapping_add(1);
                let lo = self.ram[0x100 | self.spc700.sp as usize] as u16;
                self.spc700.sp = self.spc700.sp.wrapping_add(1);
                let hi = self.ram[0x100 | self.spc700.sp as usize] as u16;
                self.spc700.sp = self.spc700.sp.wrapping_add(1);
                self.spc700.set_psw(psw);
                self.spc700.pc = (hi << 8) | lo;
            }
            
            // PUSH A - 0x2D
            0x2D => {
                self.ram[0x100 | self.spc700.sp as usize] = self.spc700.a;
                self.spc700.sp = self.spc700.sp.wrapping_sub(1);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // PUSH X - 0x0D
            0x0D => {
                self.ram[0x100 | self.spc700.sp as usize] = self.spc700.x;
                self.spc700.sp = self.spc700.sp.wrapping_sub(1);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // PUSH Y - 0x4D
            0x4D => {
                self.ram[0x100 | self.spc700.sp as usize] = self.spc700.y;
                self.spc700.sp = self.spc700.sp.wrapping_sub(1);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // PUSH PSW - 0x0F
            0x0F => {
                self.ram[0x100 | self.spc700.sp as usize] = self.spc700.get_psw();
                self.spc700.sp = self.spc700.sp.wrapping_sub(1);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // POP A - 0xAE (conflicts - actual POP A not 0x4D)
            0xAE => {
                self.spc700.sp = self.spc700.sp.wrapping_add(1);
                self.spc700.a = self.ram[0x100 | self.spc700.sp as usize];
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // POP X - 0x2E
            0x2E => {
                self.spc700.sp = self.spc700.sp.wrapping_add(1);
                self.spc700.x = self.ram[0x100 | self.spc700.sp as usize];
                self.update_nz(self.spc700.x);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // POP Y - 0x6E
            0x6E => {
                self.spc700.sp = self.spc700.sp.wrapping_add(1);
                self.spc700.y = self.ram[0x100 | self.spc700.sp as usize];
                self.update_nz(self.spc700.y);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // POP PSW - 0xCE
            0xCE => {
                self.spc700.sp = self.spc700.sp.wrapping_add(1);
                let psw = self.ram[0x100 | self.spc700.sp as usize];
                self.spc700.set_psw(psw);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // CLR1 bit,addr - clear bit - 0x12
            0x12 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let bit = (self.ram[self.spc700.pc as usize] >> 5) & 7;
                let addr = (self.ram[self.spc700.pc as usize] & 0x1F) as u16;
                let val = self.ram[addr as usize];
                self.ram[addr as usize] = val & !(1 << bit);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // SET1 bit,addr - set bit - 0x02
            0x02 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let bit = (self.ram[self.spc700.pc as usize] >> 5) & 7;
                let addr = (self.ram[self.spc700.pc as usize] & 0x1F) as u16;
                let val = self.ram[addr as usize];
                self.ram[addr as usize] = val | (1 << bit);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // CLRC - clear carry flag - 0x60
            0x60 => {
                self.spc700.c = false;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // CLRD - clear decimal flag - 0x20
            0x20 => {
                self.spc700.d = false;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // CLRI - clear interrupt flag - 0x40
            0x40 => {
                self.spc700.i = false;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // CLRV - clear overflow flag - 0xED
            0xED => {
                self.spc700.v = false;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // SETC - set carry flag - 0x80
            0x80 => {
                self.spc700.c = true;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // SETD - set decimal flag - 0xA0
            0xA0 => {
                self.spc700.d = true;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // SETI - set interrupt flag - 0xC0
            0xC0 => {
                self.spc700.i = true;
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // Unknown opcode - skip
            _ => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
        }
    }
    
    fn update_nz(&mut self, val: u8) {
        self.spc700.n = (val & 0x80) != 0;
        self.spc700.z = val == 0;
    }
    
    pub fn read_ram(&self, addr: u16) -> u8 {
        self.ram[addr as usize]
    }
    
    pub fn write_ram(&mut self, addr: u16, val: u8) {
        self.ram[addr as usize] = val;
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brr_decode_silence() {
        let mut voice = Voice::new();
        let brr_block = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        voice.decode_brr_block(&brr_block);
        for sample in voice.brr_buffer.iter() {
            assert!(*sample <= 2 && *sample >= -2);
        }
    }

    #[test]
    fn test_envelope_adsr_mode() {
        let mut voice = Voice::new();
        voice.adsr1 = 0xEF;
        voice.adsr2 = 0x00;
        voice.envx = 0;
        voice.update_envelope();
        assert!(voice.envx > 0);
    }

    #[test]
    fn test_apu_initialization() {
        let apu = APU::new();
        assert_eq!(apu.spc700.pc, 0xFFC0);
        assert_eq!(apu.spc700.sp, 0xEF);
    }

    #[test]
    fn test_dsp_register_read_write() {
        let mut dsp = DSP::new();
        dsp.write_register(0x00, 127);
        assert_eq!(dsp.read_register(0x00), 127);
    }
}
