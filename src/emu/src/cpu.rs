use crate::bus::Bus;

// Enum for Processor Status Flags
#[repr(u8)]
pub enum Flags {
    C = 1 << 0, // Carry
    Z = 1 << 1, // Zero
    I = 1 << 2, // Interrupt Disable
    D = 1 << 3, // Decimal
    X = 1 << 4, // Index Register Width
    M = 1 << 5, // Accumulator/Memory Width
    V = 1 << 6, // Overflow
    N = 1 << 7, // Negative
}

pub struct _65816 {
    // Registers
    pub a: u16,
    pub dbr: u8,
    pub dr: u16,
    pub k: u8, // Program Bank Register
    pub pc: u16,
    pub p: u8, // Status Register (now a u8)
    pub s: u16,
    pub x: u16,
    pub y: u16,
    pub e: bool, // Emulation mode flag

    // Clock
    pub cycles: u64,
}

impl _65816 {
    pub fn new() -> Self {
        _65816 {
            a: 0, dbr: 0, dr: 0, k: 0, pc: 0, p: 0, s: 0x01FF, x: 0, y: 0,
            e: true, // SNES starts in emulation mode
            cycles: 0,
        }
    }
    
    // --- Flag helpers ---
    fn get_flag(&self, flag: Flags) -> bool {
        (self.p & flag as u8) > 0
    }

    fn set_flag(&mut self, flag: Flags, val: bool) {
        if val {
            self.p |= flag as u8;
        } else {
            self.p &= !(flag as u8);
        }
    }

    // --- PC helpers ---
    fn read_pc(&mut self, bus: &Bus) -> u8 {
        let val = bus.read(self.k, self.pc);
        self.pc = self.pc.wrapping_add(1);
        self.cycles += 8;
        val
    }

    fn read_pc16(&mut self, bus: &Bus) -> u16 {
        let low = self.read_pc(bus) as u16;
        let high = self.read_pc(bus) as u16;
        (high << 8) | low
    }
    
    // --- Addressing Modes ---
    fn addr_absolute(&mut self, bus: &Bus) -> u32 {
        let addr = self.read_pc16(bus) as u32;
        ((self.dbr as u32) << 16) | addr
    }
    
    // --- Opcodes ---
    fn lda(&mut self, val: u16) {
        if self.get_flag(Flags::M) { // 8-bit mode
            self.a = (self.a & 0xFF00) | (val & 0x00FF);
        } else { // 16-bit mode
            self.a = val;
        }
        self.set_flag(Flags::Z, self.a == 0);
        self.set_flag(Flags::N, (self.a & 0x8000) != 0);
    }
    
    fn sep(&mut self, bus: &mut Bus) {
        let val = self.read_pc(bus);
        self.p |= val;
    }

    pub fn tick(&mut self, bus: &mut Bus) {
        let opcode = self.read_pc(bus);

        match opcode {
            // LDA [absolute] - LoaD Accumulator
            0xAD => {
                let addr = self.addr_absolute(bus);
                let val_low = bus.read((addr >> 16) as u8, addr as u16) as u16;
                let val_high = bus.read((addr >> 16) as u8, (addr + 1) as u16) as u16;
                self.lda((val_high << 8) | val_low);
            }
            
            // SEP #imm - SEt Processor status bits
            0xE2 => {
                self.sep(bus);
            }

            // NOP - No OPeration
            0xEA => {
                self.cycles += 2;
            }

            // ... Implement the other 253 opcodes ...
            _ => { /* Unimplemented opcode, do nothing for now */ }
        }
    }

    // ... (keep your irq and nmi functions, but they will need to be updated later
    // to push to the stack and handle emulation mode correctly)
}