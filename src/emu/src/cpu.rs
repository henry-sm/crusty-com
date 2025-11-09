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

// Addressing modes
#[derive(Debug, Clone, Copy)]
pub enum AddrMode {
    Implied,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    ZeroPageIndirect,
    ZeroPageIndirectX,
    ZeroPageIndirectY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    AbsoluteIndirect,
    AbsoluteIndexedIndirect,
    IndirectIndexedAbsolute,
    LongAbsolute,
    LongAbsoluteX,
    Relative,
    LongRelative,
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
    
    // Interrupts
    pub nmi_pending: bool,
    pub irq_pending: bool,
}

impl _65816 {
    pub fn new() -> Self {
        _65816 {
            a: 0, dbr: 0, dr: 0, k: 0, pc: 0, p: 0, s: 0x01FF, x: 0, y: 0,
            e: true, // SNES starts in emulation mode
            cycles: 0,
            nmi_pending: false,
            irq_pending: false,
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

    fn addr_absolute_x(&mut self, bus: &Bus) -> u32 {
        let addr = self.read_pc16(bus) as u32;
        let offset = self.x as u32;
        ((self.dbr as u32) << 16) | ((addr.wrapping_add(offset)) as u16 as u32)
    }

    fn addr_absolute_y(&mut self, bus: &Bus) -> u32 {
        let addr = self.read_pc16(bus) as u32;
        let offset = self.y as u32;
        ((self.dbr as u32) << 16) | ((addr.wrapping_add(offset)) as u16 as u32)
    }

    fn addr_zeropage(&mut self, bus: &Bus) -> u16 {
        self.read_pc(bus) as u16
    }

    fn addr_zeropage_x(&mut self, bus: &Bus) -> u16 {
        let addr = self.read_pc(bus);
        addr.wrapping_add(self.x as u8) as u16
    }

    fn addr_zeropage_y(&mut self, bus: &Bus) -> u16 {
        let addr = self.read_pc(bus);
        addr.wrapping_add(self.y as u8) as u16
    }

    fn addr_indirect_x(&mut self, bus: &Bus) -> u16 {
        let addr = self.read_pc(bus).wrapping_add(self.x as u8);
        let low = bus.read(0, addr as u16) as u16;
        let high = bus.read(0, addr.wrapping_add(1) as u16) as u16;
        (high << 8) | low
    }

    fn addr_indirect_y(&mut self, bus: &Bus) -> u16 {
        let addr = self.read_pc(bus);
        let low = bus.read(0, addr as u16) as u16;
        let high = bus.read(0, addr.wrapping_add(1) as u16) as u16;
        let base = (high << 8) | low;
        base.wrapping_add(self.y as u16)
    }

    fn addr_long_absolute(&mut self, bus: &Bus) -> u32 {
        let low = self.read_pc(bus) as u32;
        let mid = self.read_pc(bus) as u32;
        let high = self.read_pc(bus) as u32;
        (high << 16) | (mid << 8) | low
    }

    // --- Flag manipulation ---
    fn update_flags_nz(&mut self, val: u16) {
        self.set_flag(Flags::Z, val == 0);
        self.set_flag(Flags::N, (val & 0x8000) != 0);
    }

    fn update_flags_nz_8bit(&mut self, val: u8) {
        self.set_flag(Flags::Z, val == 0);
        self.set_flag(Flags::N, (val & 0x80) != 0);
    }
    
    // --- Load/Store Instructions ---
    fn lda(&mut self, val: u16) {
        if self.get_flag(Flags::M) { // 8-bit mode
            self.a = (self.a & 0xFF00) | (val & 0x00FF);
            self.update_flags_nz_8bit((self.a & 0xFF) as u8);
        } else { // 16-bit mode
            self.a = val;
            self.update_flags_nz(self.a);
        }
    }

    fn ldx(&mut self, val: u16) {
        if self.get_flag(Flags::X) { // 8-bit mode
            self.x = (self.x & 0xFF00) | (val & 0x00FF);
            self.update_flags_nz_8bit((self.x & 0xFF) as u8);
        } else { // 16-bit mode
            self.x = val;
            self.update_flags_nz(self.x);
        }
    }

    fn ldy(&mut self, val: u16) {
        if self.get_flag(Flags::X) { // 8-bit mode
            self.y = (self.y & 0xFF00) | (val & 0x00FF);
            self.update_flags_nz_8bit((self.y & 0xFF) as u8);
        } else { // 16-bit mode
            self.y = val;
            self.update_flags_nz(self.y);
        }
    }

    fn sta(&mut self, bus: &mut Bus, addr: u32) {
        if self.get_flag(Flags::M) { // 8-bit mode
            bus.write((addr >> 16) as u8, addr as u16, (self.a & 0xFF) as u8);
        } else { // 16-bit mode
            bus.write((addr >> 16) as u8, addr as u16, (self.a & 0xFF) as u8);
            bus.write((addr >> 16) as u8, (addr + 1) as u16, ((self.a >> 8) & 0xFF) as u8);
        }
    }

    fn stx(&mut self, bus: &mut Bus, addr: u32) {
        if self.get_flag(Flags::X) { // 8-bit mode
            bus.write((addr >> 16) as u8, addr as u16, (self.x & 0xFF) as u8);
        } else { // 16-bit mode
            bus.write((addr >> 16) as u8, addr as u16, (self.x & 0xFF) as u8);
            bus.write((addr >> 16) as u8, (addr + 1) as u16, ((self.x >> 8) & 0xFF) as u8);
        }
    }

    fn sty(&mut self, bus: &mut Bus, addr: u32) {
        if self.get_flag(Flags::X) { // 8-bit mode
            bus.write((addr >> 16) as u8, addr as u16, (self.y & 0xFF) as u8);
        } else { // 16-bit mode
            bus.write((addr >> 16) as u8, addr as u16, (self.y & 0xFF) as u8);
            bus.write((addr >> 16) as u8, (addr + 1) as u16, ((self.y >> 8) & 0xFF) as u8);
        }
    }

    // --- Arithmetic Instructions ---
    fn adc(&mut self, val: u16) {
        let carry = if self.get_flag(Flags::C) { 1 } else { 0 };
        if self.get_flag(Flags::M) { // 8-bit mode
            let a_low = (self.a & 0xFF) as u8;
            let result = (a_low as u16) + (val & 0xFF) + carry;
            self.a = (self.a & 0xFF00) | (result & 0xFF);
            self.set_flag(Flags::C, result > 0xFF);
            self.set_flag(Flags::V, ((a_low as u16 ^ result) & (val ^ result) & 0x80) != 0);
            self.update_flags_nz_8bit((self.a & 0xFF) as u8);
        } else { // 16-bit mode
            let result = (self.a as u32) + (val as u32) + (carry as u32);
            self.a = result as u16;
            self.set_flag(Flags::C, result > 0xFFFF);
            self.set_flag(Flags::V, ((self.a ^ result as u16) & (val ^ result as u16) & 0x8000) != 0);
            self.update_flags_nz(self.a);
        }
    }

    fn sbc(&mut self, val: u16) {
        let carry = if self.get_flag(Flags::C) { 0 } else { 1 };
        if self.get_flag(Flags::M) { // 8-bit mode
            let a_low = (self.a & 0xFF) as u8;
            let result = (a_low as u16).wrapping_sub((val & 0xFF) as u16).wrapping_sub(carry);
            self.a = (self.a & 0xFF00) | (result & 0xFF);
            self.set_flag(Flags::C, result <= 0xFF);
            self.set_flag(Flags::V, ((a_low as u16 ^ result) & ((a_low as u16 ^ val) ^ result) & 0x80) != 0);
            self.update_flags_nz_8bit((self.a & 0xFF) as u8);
        } else { // 16-bit mode
            let result = (self.a as u32).wrapping_sub(val as u32).wrapping_sub(carry as u32);
            self.a = result as u16;
            self.set_flag(Flags::C, result <= 0xFFFF);
            self.set_flag(Flags::V, ((self.a ^ result as u16) & ((self.a ^ val) ^ result as u16) & 0x8000) != 0);
            self.update_flags_nz(self.a);
        }
    }

    // --- Logical Instructions ---
    fn and(&mut self, val: u16) {
        if self.get_flag(Flags::M) { // 8-bit mode
            self.a = (self.a & 0xFF00) | ((self.a & val) & 0xFF);
            self.update_flags_nz_8bit((self.a & 0xFF) as u8);
        } else { // 16-bit mode
            self.a = self.a & val;
            self.update_flags_nz(self.a);
        }
    }

    fn ora(&mut self, val: u16) {
        if self.get_flag(Flags::M) { // 8-bit mode
            self.a = (self.a & 0xFF00) | ((self.a | val) & 0xFF);
            self.update_flags_nz_8bit((self.a & 0xFF) as u8);
        } else { // 16-bit mode
            self.a = self.a | val;
            self.update_flags_nz(self.a);
        }
    }

    fn eor(&mut self, val: u16) {
        if self.get_flag(Flags::M) { // 8-bit mode
            self.a = (self.a & 0xFF00) | ((self.a ^ val) & 0xFF);
            self.update_flags_nz_8bit((self.a & 0xFF) as u8);
        } else { // 16-bit mode
            self.a = self.a ^ val;
            self.update_flags_nz(self.a);
        }
    }

    // --- Shift/Rotate Instructions ---
    fn asl_a(&mut self) {
        if self.get_flag(Flags::M) { // 8-bit mode
            let val = (self.a & 0xFF) as u8;
            self.set_flag(Flags::C, (val & 0x80) != 0);
            self.a = (self.a & 0xFF00) | ((val << 1) as u16 & 0xFF);
            self.update_flags_nz_8bit((self.a & 0xFF) as u8);
        } else { // 16-bit mode
            self.set_flag(Flags::C, (self.a & 0x8000) != 0);
            self.a = self.a.wrapping_shl(1);
            self.update_flags_nz(self.a);
        }
    }

    fn lsr_a(&mut self) {
        if self.get_flag(Flags::M) { // 8-bit mode
            let val = (self.a & 0xFF) as u8;
            self.set_flag(Flags::C, (val & 0x01) != 0);
            self.a = (self.a & 0xFF00) | ((val >> 1) as u16 & 0xFF);
            self.update_flags_nz_8bit((self.a & 0xFF) as u8);
        } else { // 16-bit mode
            self.set_flag(Flags::C, (self.a & 0x01) != 0);
            self.a = self.a.wrapping_shr(1);
            self.update_flags_nz(self.a);
        }
    }

    fn rol_a(&mut self) {
        if self.get_flag(Flags::M) { // 8-bit mode
            let val = (self.a & 0xFF) as u8;
            let carry = if self.get_flag(Flags::C) { 1 } else { 0 };
            self.set_flag(Flags::C, (val & 0x80) != 0);
            self.a = (self.a & 0xFF00) | (((val << 1) | carry) as u16 & 0xFF);
            self.update_flags_nz_8bit((self.a & 0xFF) as u8);
        } else { // 16-bit mode
            let carry = if self.get_flag(Flags::C) { 1 } else { 0 };
            self.set_flag(Flags::C, (self.a & 0x8000) != 0);
            self.a = (self.a.wrapping_shl(1)) | carry as u16;
            self.update_flags_nz(self.a);
        }
    }

    fn ror_a(&mut self) {
        if self.get_flag(Flags::M) { // 8-bit mode
            let val = (self.a & 0xFF) as u8;
            let carry = if self.get_flag(Flags::C) { 0x80 } else { 0 };
            self.set_flag(Flags::C, (val & 0x01) != 0);
            self.a = (self.a & 0xFF00) | (((val >> 1) | carry) as u16 & 0xFF);
            self.update_flags_nz_8bit((self.a & 0xFF) as u8);
        } else { // 16-bit mode
            let carry = if self.get_flag(Flags::C) { 0x8000 } else { 0 };
            self.set_flag(Flags::C, (self.a & 0x01) != 0);
            self.a = (self.a.wrapping_shr(1)) | carry;
            self.update_flags_nz(self.a);
        }
    }

    // --- Compare Instructions ---
    fn cmp(&mut self, val: u16) {
        if self.get_flag(Flags::M) { // 8-bit mode
            let a_low = (self.a & 0xFF) as u8;
            let result = a_low.wrapping_sub((val & 0xFF) as u8);
            self.set_flag(Flags::C, a_low >= (val & 0xFF) as u8);
            self.update_flags_nz_8bit(result);
        } else { // 16-bit mode
            let result = self.a.wrapping_sub(val);
            self.set_flag(Flags::C, self.a >= val);
            self.update_flags_nz(result);
        }
    }

    fn cpx(&mut self, val: u16) {
        if self.get_flag(Flags::X) { // 8-bit mode
            let x_low = (self.x & 0xFF) as u8;
            let result = x_low.wrapping_sub((val & 0xFF) as u8);
            self.set_flag(Flags::C, x_low >= (val & 0xFF) as u8);
            self.update_flags_nz_8bit(result);
        } else { // 16-bit mode
            let result = self.x.wrapping_sub(val);
            self.set_flag(Flags::C, self.x >= val);
            self.update_flags_nz(result);
        }
    }

    fn cpy(&mut self, val: u16) {
        if self.get_flag(Flags::X) { // 8-bit mode
            let y_low = (self.y & 0xFF) as u8;
            let result = y_low.wrapping_sub((val & 0xFF) as u8);
            self.set_flag(Flags::C, y_low >= (val & 0xFF) as u8);
            self.update_flags_nz_8bit(result);
        } else { // 16-bit mode
            let result = self.y.wrapping_sub(val);
            self.set_flag(Flags::C, self.y >= val);
            self.update_flags_nz(result);
        }
    }

    // --- Bit Test Instructions ---
    fn bit(&mut self, val: u16) {
        if self.get_flag(Flags::M) { // 8-bit mode
            let result = (self.a & val) as u8;
            self.set_flag(Flags::Z, result == 0);
            self.set_flag(Flags::N, (val & 0x80) != 0);
            self.set_flag(Flags::V, (val & 0x40) != 0);
        } else { // 16-bit mode
            let result = self.a & val;
            self.set_flag(Flags::Z, result == 0);
            self.set_flag(Flags::N, (val & 0x8000) != 0);
            self.set_flag(Flags::V, (val & 0x4000) != 0);
        }
    }

    // --- Increment/Decrement Instructions ---
    fn inc_a(&mut self) {
        if self.get_flag(Flags::M) { // 8-bit mode
            self.a = (self.a & 0xFF00) | ((self.a & 0xFF).wrapping_add(1) & 0xFF);
            self.update_flags_nz_8bit((self.a & 0xFF) as u8);
        } else { // 16-bit mode
            self.a = self.a.wrapping_add(1);
            self.update_flags_nz(self.a);
        }
    }

    fn dec_a(&mut self) {
        if self.get_flag(Flags::M) { // 8-bit mode
            self.a = (self.a & 0xFF00) | ((self.a & 0xFF).wrapping_sub(1) & 0xFF);
            self.update_flags_nz_8bit((self.a & 0xFF) as u8);
        } else { // 16-bit mode
            self.a = self.a.wrapping_sub(1);
            self.update_flags_nz(self.a);
        }
    }

    fn inx(&mut self) {
        if self.get_flag(Flags::X) { // 8-bit mode
            self.x = (self.x & 0xFF00) | ((self.x & 0xFF).wrapping_add(1) & 0xFF);
            self.update_flags_nz_8bit((self.x & 0xFF) as u8);
        } else { // 16-bit mode
            self.x = self.x.wrapping_add(1);
            self.update_flags_nz(self.x);
        }
    }

    fn dex(&mut self) {
        if self.get_flag(Flags::X) { // 8-bit mode
            self.x = (self.x & 0xFF00) | ((self.x & 0xFF).wrapping_sub(1) & 0xFF);
            self.update_flags_nz_8bit((self.x & 0xFF) as u8);
        } else { // 16-bit mode
            self.x = self.x.wrapping_sub(1);
            self.update_flags_nz(self.x);
        }
    }

    fn iny(&mut self) {
        if self.get_flag(Flags::X) { // 8-bit mode
            self.y = (self.y & 0xFF00) | ((self.y & 0xFF).wrapping_add(1) & 0xFF);
            self.update_flags_nz_8bit((self.y & 0xFF) as u8);
        } else { // 16-bit mode
            self.y = self.y.wrapping_add(1);
            self.update_flags_nz(self.y);
        }
    }

    fn dey(&mut self) {
        if self.get_flag(Flags::X) { // 8-bit mode
            self.y = (self.y & 0xFF00) | ((self.y & 0xFF).wrapping_sub(1) & 0xFF);
            self.update_flags_nz_8bit((self.y & 0xFF) as u8);
        } else { // 16-bit mode
            self.y = self.y.wrapping_sub(1);
            self.update_flags_nz(self.y);
        }
    }

    // --- Transfer Instructions ---
    fn tax(&mut self) {
        self.x = self.a;
        self.update_flags_nz(self.x);
    }

    fn tay(&mut self) {
        self.y = self.a;
        self.update_flags_nz(self.y);
    }

    fn txa(&mut self) {
        self.a = self.x;
        self.update_flags_nz(self.a);
    }

    fn tya(&mut self) {
        self.a = self.y;
        self.update_flags_nz(self.a);
    }

    fn tsx(&mut self) {
        self.x = self.s;
        self.update_flags_nz(self.x);
    }

    fn txs(&mut self) {
        self.s = self.x;
    }

    // --- Stack Instructions ---
    fn push_8(&mut self, bus: &mut Bus, val: u8) {
        bus.write(0, self.s, val);
        self.s = self.s.wrapping_sub(1);
    }

    fn push_16(&mut self, bus: &mut Bus, val: u16) {
        self.push_8(bus, ((val >> 8) & 0xFF) as u8);
        self.push_8(bus, (val & 0xFF) as u8);
    }

    fn pop_8(&mut self, bus: &Bus) -> u8 {
        self.s = self.s.wrapping_add(1);
        bus.read(0, self.s)
    }

    fn pop_16(&mut self, bus: &Bus) -> u16 {
        let low = self.pop_8(bus) as u16;
        let high = self.pop_8(bus) as u16;
        (high << 8) | low
    }

    fn pha(&mut self, bus: &mut Bus) {
        if self.get_flag(Flags::M) { // 8-bit mode
            self.push_8(bus, (self.a & 0xFF) as u8);
        } else { // 16-bit mode
            self.push_16(bus, self.a);
        }
    }

    fn phx(&mut self, bus: &mut Bus) {
        if self.get_flag(Flags::X) { // 8-bit mode
            self.push_8(bus, (self.x & 0xFF) as u8);
        } else { // 16-bit mode
            self.push_16(bus, self.x);
        }
    }

    fn phy(&mut self, bus: &mut Bus) {
        if self.get_flag(Flags::X) { // 8-bit mode
            self.push_8(bus, (self.y & 0xFF) as u8);
        } else { // 16-bit mode
            self.push_16(bus, self.y);
        }
    }

    fn pla(&mut self, bus: &Bus) {
        let val = if self.get_flag(Flags::M) { // 8-bit mode
            self.pop_8(bus) as u16
        } else { // 16-bit mode
            self.pop_16(bus)
        };
        self.lda(val);
    }

    fn plx(&mut self, bus: &Bus) {
        let val = if self.get_flag(Flags::X) { // 8-bit mode
            self.pop_8(bus) as u16
        } else { // 16-bit mode
            self.pop_16(bus)
        };
        self.ldx(val);
    }

    fn ply(&mut self, bus: &Bus) {
        let val = if self.get_flag(Flags::X) { // 8-bit mode
            self.pop_8(bus) as u16
        } else { // 16-bit mode
            self.pop_16(bus)
        };
        self.ldy(val);
    }

    // --- Branch Instructions ---
    fn bra(&mut self, bus: &Bus) {
        let offset = self.read_pc(bus) as i8;
        self.pc = self.pc.wrapping_add(offset as u16);
    }

    fn bcc(&mut self, bus: &Bus) {
        let offset = self.read_pc(bus) as i8;
        if !self.get_flag(Flags::C) {
            self.pc = self.pc.wrapping_add(offset as u16);
        }
    }

    fn bcs(&mut self, bus: &Bus) {
        let offset = self.read_pc(bus) as i8;
        if self.get_flag(Flags::C) {
            self.pc = self.pc.wrapping_add(offset as u16);
        }
    }

    fn beq(&mut self, bus: &Bus) {
        let offset = self.read_pc(bus) as i8;
        if self.get_flag(Flags::Z) {
            self.pc = self.pc.wrapping_add(offset as u16);
        }
    }

    fn bne(&mut self, bus: &Bus) {
        let offset = self.read_pc(bus) as i8;
        if !self.get_flag(Flags::Z) {
            self.pc = self.pc.wrapping_add(offset as u16);
        }
    }

    fn bmi(&mut self, bus: &Bus) {
        let offset = self.read_pc(bus) as i8;
        if self.get_flag(Flags::N) {
            self.pc = self.pc.wrapping_add(offset as u16);
        }
    }

    fn bpl(&mut self, bus: &Bus) {
        let offset = self.read_pc(bus) as i8;
        if !self.get_flag(Flags::N) {
            self.pc = self.pc.wrapping_add(offset as u16);
        }
    }

    fn bvs(&mut self, bus: &Bus) {
        let offset = self.read_pc(bus) as i8;
        if self.get_flag(Flags::V) {
            self.pc = self.pc.wrapping_add(offset as u16);
        }
    }

    fn bvc(&mut self, bus: &Bus) {
        let offset = self.read_pc(bus) as i8;
        if !self.get_flag(Flags::V) {
            self.pc = self.pc.wrapping_add(offset as u16);
        }
    }

    // --- Jump/Return Instructions ---
    fn jmp(&mut self, addr: u32) {
        self.k = (addr >> 16) as u8;
        self.pc = addr as u16;
    }

    fn jsr(&mut self, bus: &mut Bus, addr: u32) {
        self.push_16(bus, self.pc.wrapping_sub(1));
        self.k = (addr >> 16) as u8;
        self.pc = addr as u16;
    }

    fn rts(&mut self, bus: &Bus) {
        self.pc = self.pop_16(bus).wrapping_add(1);
    }

    fn rti(&mut self, bus: &Bus) {
        self.p = self.pop_8(bus);
        self.pc = self.pop_16(bus);
    }

    // --- Status Register Instructions ---
    fn sep(&mut self, bus: &Bus) {
        let val = self.read_pc(bus);
        self.p |= val;
    }

    fn rep(&mut self, bus: &Bus) {
        let val = self.read_pc(bus);
        self.p &= !val;
    }

    fn clc(&mut self) {
        self.set_flag(Flags::C, false);
    }

    fn sec(&mut self) {
        self.set_flag(Flags::C, true);
    }

    fn cld(&mut self) {
        self.set_flag(Flags::D, false);
    }

    fn sed(&mut self) {
        self.set_flag(Flags::D, true);
    }

    fn cli(&mut self) {
        self.set_flag(Flags::I, false);
    }

    fn sei(&mut self) {
        self.set_flag(Flags::I, true);
    }

    fn clv(&mut self) {
        self.set_flag(Flags::V, false);
    }

    // --- Interrupt Handling ---
    
    fn push_byte(&mut self, bus: &mut Bus, val: u8) {
        bus.write(0, self.s, val);
        self.s = self.s.wrapping_sub(1);
        self.cycles += 8;
    }

    fn push_word(&mut self, bus: &mut Bus, val: u16) {
        let high = (val >> 8) as u8;
        let low = val as u8;
        self.push_byte(bus, high);
        self.push_byte(bus, low);
    }

    fn handle_nmi(&mut self, bus: &mut Bus) {
        // Push status register
        self.push_byte(bus, self.p);
        // Push PC
        self.push_word(bus, self.pc);
        // Set interrupt disable flag
        self.set_flag(Flags::I, true);
        // Jump to NMI vector at 0xFFEA
        let nmi_vector = 0xFFEA;
        let low = bus.read(0, nmi_vector) as u16;
        let high = bus.read(0, nmi_vector + 1) as u16;
        self.pc = (high << 8) | low;
        self.cycles += 24; // NMI takes ~8 cycles (3 pushes)
    }

    fn handle_irq(&mut self, bus: &mut Bus) {
        // Don't handle IRQ if interrupt disable flag is set
        if self.get_flag(Flags::I) {
            return;
        }
        
        // Push status register
        self.push_byte(bus, self.p);
        // Push PC
        self.push_word(bus, self.pc);
        // Set interrupt disable flag
        self.set_flag(Flags::I, true);
        // Jump to IRQ vector at 0xFFEE
        let irq_vector = 0xFFEE;
        let low = bus.read(0, irq_vector) as u16;
        let high = bus.read(0, irq_vector + 1) as u16;
        self.pc = (high << 8) | low;
        self.cycles += 24;
    }

    pub fn tick(&mut self, bus: &mut Bus) {
        // Handle interrupts at the start of each cycle
        if self.nmi_pending {
            self.handle_nmi(bus);
            self.nmi_pending = false;
        } else if self.irq_pending {
            self.handle_irq(bus);
            self.irq_pending = false;
        }

        let opcode = self.read_pc(bus);

        match opcode {
            // ADC - Add with Carry
            0x69 => { let val = self.read_pc(bus) as u16; self.adc(val); } // immediate
            0x65 => { let addr = self.addr_zeropage(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.adc(val); } // zero page
            0x75 => { let addr = self.addr_zeropage_x(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.adc(val); } // zero page, X
            0x6D => { let addr = self.addr_absolute(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.adc(val); } // absolute
            0x7D => { let addr = self.addr_absolute_x(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.adc(val); } // absolute, X
            0x79 => { let addr = self.addr_absolute_y(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.adc(val); } // absolute, Y
            0x61 => { let addr = self.addr_indirect_x(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.adc(val); } // (indirect, X)
            0x71 => { let addr = self.addr_indirect_y(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.adc(val); } // (indirect), Y

            // AND - Logical AND
            0x29 => { let val = self.read_pc(bus) as u16; self.and(val); } // immediate
            0x25 => { let addr = self.addr_zeropage(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.and(val); } // zero page
            0x35 => { let addr = self.addr_zeropage_x(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.and(val); } // zero page, X
            0x2D => { let addr = self.addr_absolute(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.and(val); } // absolute
            0x3D => { let addr = self.addr_absolute_x(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.and(val); } // absolute, X
            0x39 => { let addr = self.addr_absolute_y(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.and(val); } // absolute, Y
            0x21 => { let addr = self.addr_indirect_x(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.and(val); } // (indirect, X)
            0x31 => { let addr = self.addr_indirect_y(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.and(val); } // (indirect), Y

            // ASL - Arithmetic Shift Left
            0x0A => self.asl_a(), // accumulator

            // BCC - Branch if Carry Clear
            0x90 => self.bcc(bus),

            // BCS - Branch if Carry Set
            0xB0 => self.bcs(bus),

            // BEQ - Branch if Equal (Zero flag set)
            0xF0 => self.beq(bus),

            // BIT - Bit Test
            0x24 => { let addr = self.addr_zeropage(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.bit(val); } // zero page
            0x2C => { let addr = self.addr_absolute(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.bit(val); } // absolute

            // BMI - Branch if Minus (Negative flag set)
            0x30 => self.bmi(bus),

            // BNE - Branch if Not Equal (Zero flag clear)
            0xD0 => self.bne(bus),

            // BPL - Branch if Plus (Negative flag clear)
            0x10 => self.bpl(bus),

            // BRA - Branch Always
            0x80 => self.bra(bus),

            // BVC - Branch if Overflow Clear
            0x50 => self.bvc(bus),

            // BVS - Branch if Overflow Set
            0x70 => self.bvs(bus),

            // CLC - Clear Carry Flag
            0x18 => self.clc(),

            // CLD - Clear Decimal Flag
            0xD8 => self.cld(),

            // CLI - Clear Interrupt Disable Flag
            0x58 => self.cli(),

            // CLV - Clear Overflow Flag
            0xB8 => self.clv(),

            // CMP - Compare Accumulator
            0xC9 => { let val = self.read_pc(bus) as u16; self.cmp(val); } // immediate
            0xC5 => { let addr = self.addr_zeropage(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.cmp(val); } // zero page
            0xD5 => { let addr = self.addr_zeropage_x(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.cmp(val); } // zero page, X
            0xCD => { let addr = self.addr_absolute(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.cmp(val); } // absolute
            0xDD => { let addr = self.addr_absolute_x(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.cmp(val); } // absolute, X
            0xD9 => { let addr = self.addr_absolute_y(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.cmp(val); } // absolute, Y
            0xC1 => { let addr = self.addr_indirect_x(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.cmp(val); } // (indirect, X)
            0xD1 => { let addr = self.addr_indirect_y(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.cmp(val); } // (indirect), Y

            // CPX - Compare X Register
            0xE0 => { let val = self.read_pc(bus) as u16; self.cpx(val); } // immediate
            0xE4 => { let addr = self.addr_zeropage(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.cpx(val); } // zero page
            0xEC => { let addr = self.addr_absolute(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.cpx(val); } // absolute

            // CPY - Compare Y Register
            0xC0 => { let val = self.read_pc(bus) as u16; self.cpy(val); } // immediate
            0xC4 => { let addr = self.addr_zeropage(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.cpy(val); } // zero page
            0xCC => { let addr = self.addr_absolute(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.cpy(val); } // absolute

            // DEC - Decrement
            0xC6 => { let addr = self.addr_zeropage(bus) as u32; let val = bus.read(0, addr as u16).wrapping_sub(1); bus.write(0, addr as u16, val); } // zero page
            0xD6 => { let addr = self.addr_zeropage_x(bus) as u32; let val = bus.read(0, addr as u16).wrapping_sub(1); bus.write(0, addr as u16, val); } // zero page, X
            0xCE => { let addr = self.addr_absolute(bus); let val = bus.read((addr >> 16) as u8, addr as u16).wrapping_sub(1); bus.write((addr >> 16) as u8, addr as u16, val); } // absolute
            0xDE => { let addr = self.addr_absolute_x(bus); let val = bus.read((addr >> 16) as u8, addr as u16).wrapping_sub(1); bus.write((addr >> 16) as u8, addr as u16, val); } // absolute, X

            // DEX - Decrement X
            0xCA => self.dex(),

            // DEY - Decrement Y
            0x88 => self.dey(),

            // EOR - Exclusive OR
            0x49 => { let val = self.read_pc(bus) as u16; self.eor(val); } // immediate
            0x45 => { let addr = self.addr_zeropage(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.eor(val); } // zero page
            0x55 => { let addr = self.addr_zeropage_x(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.eor(val); } // zero page, X
            0x4D => { let addr = self.addr_absolute(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.eor(val); } // absolute
            0x5D => { let addr = self.addr_absolute_x(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.eor(val); } // absolute, X
            0x59 => { let addr = self.addr_absolute_y(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.eor(val); } // absolute, Y
            0x41 => { let addr = self.addr_indirect_x(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.eor(val); } // (indirect, X)
            0x51 => { let addr = self.addr_indirect_y(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.eor(val); } // (indirect), Y

            // INC - Increment
            0xE6 => { let addr = self.addr_zeropage(bus) as u32; let val = bus.read(0, addr as u16).wrapping_add(1); bus.write(0, addr as u16, val); } // zero page
            0xF6 => { let addr = self.addr_zeropage_x(bus) as u32; let val = bus.read(0, addr as u16).wrapping_add(1); bus.write(0, addr as u16, val); } // zero page, X
            0xEE => { let addr = self.addr_absolute(bus); let val = bus.read((addr >> 16) as u8, addr as u16).wrapping_add(1); bus.write((addr >> 16) as u8, addr as u16, val); } // absolute
            0xFE => { let addr = self.addr_absolute_x(bus); let val = bus.read((addr >> 16) as u8, addr as u16).wrapping_add(1); bus.write((addr >> 16) as u8, addr as u16, val); } // absolute, X

            // INX - Increment X
            0xE8 => self.inx(),

            // INY - Increment Y
            0xC8 => self.iny(),

            // JMP - Jump
            0x4C => { let addr = self.addr_absolute(bus); self.jmp(addr); } // absolute
            0x5C => { let addr = self.addr_long_absolute(bus); self.jmp(addr); } // long absolute

            // JSR - Jump to Subroutine
            0x20 => { let addr = self.addr_absolute(bus); self.jsr(bus, addr); } // absolute

            // LDA - Load Accumulator
            0xA9 => { let val = self.read_pc(bus) as u16; self.lda(val); } // immediate
            0xA5 => { let addr = self.addr_zeropage(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.lda(val); } // zero page
            0xB5 => { let addr = self.addr_zeropage_x(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.lda(val); } // zero page, X
            0xAD => { let addr = self.addr_absolute(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.lda(val); } // absolute
            0xBD => { let addr = self.addr_absolute_x(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.lda(val); } // absolute, X
            0xB9 => { let addr = self.addr_absolute_y(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.lda(val); } // absolute, Y
            0xA1 => { let addr = self.addr_indirect_x(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.lda(val); } // (indirect, X)
            0xB1 => { let addr = self.addr_indirect_y(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.lda(val); } // (indirect), Y

            // LDX - Load X Register
            0xA2 => { let val = self.read_pc(bus) as u16; self.ldx(val); } // immediate
            0xA6 => { let addr = self.addr_zeropage(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.ldx(val); } // zero page
            0xB6 => { let addr = self.addr_zeropage_y(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.ldx(val); } // zero page, Y
            0xAE => { let addr = self.addr_absolute(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.ldx(val); } // absolute
            0xBE => { let addr = self.addr_absolute_y(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.ldx(val); } // absolute, Y

            // LDY - Load Y Register
            0xA0 => { let val = self.read_pc(bus) as u16; self.ldy(val); } // immediate
            0xA4 => { let addr = self.addr_zeropage(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.ldy(val); } // zero page
            0xB4 => { let addr = self.addr_zeropage_x(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.ldy(val); } // zero page, X
            0xAC => { let addr = self.addr_absolute(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.ldy(val); } // absolute
            0xBC => { let addr = self.addr_absolute_x(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.ldy(val); } // absolute, X

            // LSR - Logical Shift Right
            0x4A => self.lsr_a(), // accumulator

            // NOP - No Operation
            0xEA => { self.cycles += 2; }

            // ORA - Logical OR
            0x09 => { let val = self.read_pc(bus) as u16; self.ora(val); } // immediate
            0x05 => { let addr = self.addr_zeropage(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.ora(val); } // zero page
            0x15 => { let addr = self.addr_zeropage_x(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.ora(val); } // zero page, X
            0x0D => { let addr = self.addr_absolute(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.ora(val); } // absolute
            0x1D => { let addr = self.addr_absolute_x(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.ora(val); } // absolute, X
            0x19 => { let addr = self.addr_absolute_y(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.ora(val); } // absolute, Y
            0x01 => { let addr = self.addr_indirect_x(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.ora(val); } // (indirect, X)
            0x11 => { let addr = self.addr_indirect_y(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.ora(val); } // (indirect), Y

            // PHA - Push Accumulator
            0x48 => self.pha(bus),

            // PHP - Push Processor Status
            0x08 => self.push_8(bus, self.p),

            // PLA - Pull Accumulator
            0x68 => self.pla(bus),

            // PLP - Pull Processor Status
            0x28 => { self.p = self.pop_8(bus); }

            // PHX - Push X Register
            0xDA => self.phx(bus),

            // PLX - Pull X Register
            0xFA => self.plx(bus),

            // PHY - Push Y Register
            0x5A => self.phy(bus),

            // PLY - Pull Y Register
            0x7A => self.ply(bus),

            // REP - Reset Processor Status Bits
            0xC2 => self.rep(bus),

            // ROL - Rotate Left
            0x2A => self.rol_a(), // accumulator

            // ROR - Rotate Right
            0x6A => self.ror_a(), // accumulator

            // RTI - Return from Interrupt
            0x40 => self.rti(bus),

            // RTS - Return from Subroutine
            0x60 => self.rts(bus),

            // SBC - Subtract with Carry
            0xE9 => { let val = self.read_pc(bus) as u16; self.sbc(val); } // immediate
            0xE5 => { let addr = self.addr_zeropage(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.sbc(val); } // zero page
            0xF5 => { let addr = self.addr_zeropage_x(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.sbc(val); } // zero page, X
            0xED => { let addr = self.addr_absolute(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.sbc(val); } // absolute
            0xFD => { let addr = self.addr_absolute_x(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.sbc(val); } // absolute, X
            0xF9 => { let addr = self.addr_absolute_y(bus); let val = bus.read((addr >> 16) as u8, addr as u16) as u16; self.sbc(val); } // absolute, Y
            0xE1 => { let addr = self.addr_indirect_x(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.sbc(val); } // (indirect, X)
            0xF1 => { let addr = self.addr_indirect_y(bus) as u32; let val = bus.read(0, addr as u16) as u16; self.sbc(val); } // (indirect), Y

            // SEC - Set Carry Flag
            0x38 => self.sec(),

            // SED - Set Decimal Flag
            0xF8 => self.sed(),

            // SEI - Set Interrupt Disable Flag
            0x78 => self.sei(),

            // SEP - Set Processor Status Bits
            0xE2 => self.sep(bus),

            // STA - Store Accumulator
            0x85 => { let addr = self.addr_zeropage(bus) as u32; self.sta(bus, addr); } // zero page
            0x95 => { let addr = self.addr_zeropage_x(bus) as u32; self.sta(bus, addr); } // zero page, X
            0x8D => { let addr = self.addr_absolute(bus); self.sta(bus, addr); } // absolute
            0x9D => { let addr = self.addr_absolute_x(bus); self.sta(bus, addr); } // absolute, X
            0x99 => { let addr = self.addr_absolute_y(bus); self.sta(bus, addr); } // absolute, Y
            0x81 => { let addr = self.addr_indirect_x(bus) as u32; self.sta(bus, addr); } // (indirect, X)
            0x91 => { let addr = self.addr_indirect_y(bus) as u32; self.sta(bus, addr); } // (indirect), Y

            // STX - Store X Register
            0x86 => { let addr = self.addr_zeropage(bus) as u32; self.stx(bus, addr); } // zero page
            0x96 => { let addr = self.addr_zeropage_y(bus) as u32; self.stx(bus, addr); } // zero page, Y
            0x8E => { let addr = self.addr_absolute(bus); self.stx(bus, addr); } // absolute

            // STY - Store Y Register
            0x84 => { let addr = self.addr_zeropage(bus) as u32; self.sty(bus, addr); } // zero page
            0x94 => { let addr = self.addr_zeropage_x(bus) as u32; self.sty(bus, addr); } // zero page, X
            0x8C => { let addr = self.addr_absolute(bus); self.sty(bus, addr); } // absolute

            // TAX - Transfer Accumulator to X
            0xAA => self.tax(),

            // TAY - Transfer Accumulator to Y
            0xA8 => self.tay(),

            // TXA - Transfer X to Accumulator
            0x8A => self.txa(),

            // TYA - Transfer Y to Accumulator
            0x98 => self.tya(),

            // TSX - Transfer Stack Pointer to X
            0xBA => self.tsx(),

            // TXS - Transfer X to Stack Pointer
            0x9A => self.txs(),

            // Unimplemented opcode
            _ => { /* Unimplemented opcode, do nothing for now */ self.cycles += 2; }
        }
    }

    // ... (keep your irq and nmi functions, but they will need to be updated later
    // to push to the stack and handle emulation mode correctly)
}