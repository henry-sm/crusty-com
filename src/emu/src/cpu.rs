
pub struct CPU {
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
    ram : [u8; 131072], // 128*1024 bytes 
}

impl CPU  {
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

    // stack operations

    pub fn push(&mut self, data : u8) {
        self.ram[self.s as usize] = data;
        self.s = self.s - 1;
    }

    pub fn pop(&mut self) -> u8 {
        self.s = self.s + 1;
        self.ram[self.s as usize]
    }


    // mem read and write

    pub fn read(&mut self, addr : u16) -> u16 {
        let hi = self.ram[addr as usize] as u16;
        let lo = self.ram[(addr + 1) as usize] as u16;
        (hi << 8) | lo 
    }

    

} 

impl CPU {
    // match the opcode with the instructions
    // god help me there are 256 opcodes

    pub fn opcode(&mut self) { // opcode
        match self.opcode {

                0x00 => {
                    // brk
                    self.pc += 1; // Increment PC to the next instruction

                    // Push PC and status register onto the stack
                    self.push((self.pc >> 8) as u8); 
                    self.push(self.pc as u8); 
                    self.push(self.p | 0x10); // Set the break flag
                    
                    // Set the interrupt disable flag
                    self.i = true;

                    // Load the interrupt vector
                    self.pc = self.read(0xFFFE);
                    self.cycles = self.cycles + 7;
                    self.b = true;

                }
                
                0x01 => {
                    // ora (dp,x)
                    let addr = self.dr + self.X;
                    let data = self.ram[addr as usize];
                    self.a = self.a | data as u16;
                    self.z = self.a == 0;
                    self.n = (self.a & 0x80) != 0;
                    self.c = false;
                    self.cycles = self.cycles + 6;
                    self.pc += 1;
                }

                0x02 => {
                    // cop
                    self.pc += 1;
                    self.push((self.pc >> 8) as u8);
                    self.push(self.pc as u8);
                    self.push(self.p | 0x10);
                    self.i = true;
                    self.pc = self.read(0xFFE4);
                    self.cycles = self.cycles + 7;
                    self.b = true;
                }

                0x03 => {
                    // ora sr,s
                    let addr = self.s;
                    let data = self.ram[addr as usize];
                    self.a = self.a | data as u16;
                    self.z = self.a == 0;
                    self.n = (self.a & 0x80) != 0;
                    self.c = false;
                    self.cycles = self.cycles + 4;
                    self.pc += 1;
                }

                0x04 => {
                    // tsb dp
                    self.z = (self.a & self.ram[self.dr as usize] as u16) == 0;
                    self.ram[self.dr as usize] = self.ram[self.dr as usize] | self.a as u8;
                    self.cycles = self.cycles + 5;
                    self.pc += 1;
                }

                0x05 =>{
                    // ora dp
                    self.a = self.a | self.ram[self.dr as usize] as u16;
                    self.z = self.a == 0;
                    self.n = (self.a & 0x80) != 0;
                    self.c = false;
                    self.cycles = self.cycles + 3;
                    self.pc += 1;
                }

                0x06 => {
                    // asl dp
                    self.c = (self.ram[self.dr as usize] & 0x80) != 0;
                    self.ram[self.dr as usize] = self.ram[self.dr as usize] << 1;
                    self.z = self.ram[self.dr as usize] == 0;
                    self.n = (self.ram[self.dr as usize] & 0x80) != 0;
                    self.cycles = self.cycles + 5;
                    self.pc += 1;
                    if !self.m {
                        self.cycles = self.cycles + 1;
                    }
                }

                0x07 => {
                    // ora [dp]
                    self.a = self.a | self.read(self.ram[self.dr as usize]);
                    self.z = self.a == 0;
                    self.n = (self.a & 0x80) != 0;
                    self.cycles += 6;
                    self.pc += 1;
                }

                0x08 => {
                    // php
                    self.push(self.p | 0x10);
                    self.cycles += 3;
                    self.pc += 1;
                }
                

                _ => {
                  println!( "Invalid opcode/ work in progress");
                }


        }
    }

}
