
pub struct _65816 {
    // Registers 
    pub a : u16 , // Accumulator 
    pub dbr : u8 , // Data Bank Register
    pub d : u16 , // Direct Register
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


