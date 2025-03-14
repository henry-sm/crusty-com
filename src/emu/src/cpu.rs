use crate::bus::Bus;

pub struct _65816 {
    // Registers 
    pub a : u16 , // Accumulator 
    pub dbr : u8 , // Data Bank Register
    pub dr : u16 , // Direct Register
    pub k : u8 , // Program Bank Register
    pub pc : u16 , // Program Counter
    pub p : u8 , // status register
    pub s : u16 , // Stack Pointer
    pub X : u16 , // X Index Register
    pub Y: u16 , // Y Index Register

    // Flags
    pub b : bool , // Break 
    pub c : bool , // Carry
    pub d : bool , // Decimal
    pub e : bool , // Emulation
    pub i : bool , // Interrupt Disable
    pub m : bool , // Accumulator and Memory Width
    pub n : bool , // Negative
    pub v : bool , // Overflow
    pub x : bool , // Index Register Width
    pub z : bool , // Zero

    // Clock
    pub cycles : u64 , // Number of cycles

    //misc
    pub opcode : u8 , // opcode
    pub bus : Bus , // bus
}

impl _65816 {
    // interrupts 

    pub fn irq(&mut self) { // interrupt request
        self.b = false;
        self.e = false;
        self.i = true;
        self.pc = 0xfffe;   
    }

    pub fn nmi(&mut self) { // non askable interrupt
        self.b = false;
        self.e = false;
        self.i = true;
        self.pc = 0xfffa;
    }

    

} 

impl _65816 {
    // match the opcode with the instructions
    // god help me there are 256 opcodes

    pub fn opcode(&mut self) { // opcode
        match self.opcode {

                0x00 => {
                    // brk
                    self.pc = self.pc + 2;
                    if self.e{
                        self.pc = self.pc + 2;
                    } else {
                        self.pc = self.pc + 1;
                    }
                    self.cycles = self.cycles + 7;
                    self.b = true;
                }
        }
    }

}
