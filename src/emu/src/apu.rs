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
    pub registers: [u8; 128],   // DSP registers (0x00-0x7F)
    pub output_samples: [i16; 8], // 8 voice outputs
}

impl DSP {
    pub fn new() -> Self {
        DSP {
            registers: [0; 128],
            output_samples: [0; 8],
        }
    }
    
    pub fn read_register(&self, addr: u8) -> u8 {
        self.registers[(addr & 0x7F) as usize]
    }
    
    pub fn write_register(&mut self, addr: u8, val: u8) {
        self.registers[(addr & 0x7F) as usize] = val;
    }
}

pub struct APU {
    pub spc700: SPC700,
    pub dsp: DSP,
    pub ram: [u8; 65536],           // 64KB APU RAM
    pub cycles: u32,                // Cycle counter for sync
}

impl APU {
    pub fn new() -> Self {
        APU {
            spc700: SPC700::new(),
            dsp: DSP::new(),
            ram: [0; 65536],
            cycles: 0,
        }
    }
    
    pub fn tick(&mut self) {
        // Execute one SPC700 instruction
        self.execute_instruction();
        
        // Update DSP (runs at 1/64 of CPU speed typically)
        self.cycles += 1;
        if self.cycles >= 64 {
            self.cycles = 0;
            self.dsp_update();
        }
    }
    
    fn execute_instruction(&mut self) {
        // Fetch opcode from APU RAM
        let opcode = self.ram[self.spc700.pc as usize];
        
        // Decode and execute - SPC700 instruction set
        match opcode {
            // NOP
            0xEA => { self.spc700.pc = self.spc700.pc.wrapping_add(1); }
            
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
            
            // MOV Y, #imm - 0x8D
            0x8D => {
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
            
            // INC A
            0xBC => {
                self.spc700.a = self.spc700.a.wrapping_add(1);
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // DEC A
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
            
            // INC Y - 0xFC
            0xFC => {
                self.spc700.y = self.spc700.y.wrapping_add(1);
                self.update_nz(self.spc700.y);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // DEC Y - 0xDC
            0xDC => {
                self.spc700.y = self.spc700.y.wrapping_sub(1);
                self.update_nz(self.spc700.y);
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
            
            // AND A, #imm - 0x24
            0x24 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let imm = self.ram[self.spc700.pc as usize];
                self.spc700.a &= imm;
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
            
            // EOR A, #imm - 0x44
            0x44 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let imm = self.ram[self.spc700.pc as usize];
                self.spc700.a ^= imm;
                self.update_nz(self.spc700.a);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // CMP A, #imm - 0x64 (but conflicts with SUB) - actually 0xC8
            0xC8 => {
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
                let imm = self.ram[self.spc700.pc as usize];
                let result = (self.spc700.a as i16) - (imm as i16);
                self.spc700.c = result >= 0;
                self.update_nz(result as u8);
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
            
            // BRA offset - 0x2F
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
            
            // RET - 0x6F
            0x6F => {
                let lo = self.ram[0x100 | self.spc700.sp as usize] as u16;
                self.spc700.sp = self.spc700.sp.wrapping_add(1);
                let hi = self.ram[0x100 | self.spc700.sp as usize] as u16;
                self.spc700.sp = self.spc700.sp.wrapping_add(1);
                self.spc700.pc = (hi << 8) | lo;
            }
            
            // PUSH A - 0x2D
            0x2D => {
                self.ram[0x100 | self.spc700.sp as usize] = self.spc700.a;
                self.spc700.sp = self.spc700.sp.wrapping_sub(1);
                self.spc700.pc = self.spc700.pc.wrapping_add(1);
            }
            
            // POP A - 0x4D
            0x4D => {
                self.spc700.sp = self.spc700.sp.wrapping_add(1);
                self.spc700.a = self.ram[0x100 | self.spc700.sp as usize];
                self.update_nz(self.spc700.a);
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
    
    fn dsp_update(&mut self) {
        // DSP processing - generates audio samples
        // This is where BRR decoding, ADSR, interpolation happens
        // For now, just a placeholder
    }
    
    pub fn read_ram(&self, addr: u16) -> u8 {
        self.ram[addr as usize]
    }
    
    pub fn write_ram(&mut self, addr: u16, val: u8) {
        self.ram[addr as usize] = val;
    }
}