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

impl CPU {
    pub fn new() -> Self {
        CPU {
            a: 0,
            dbr: 0,
            dr: 0,
            k: 0,
            pc: 0,
            p: 0,
            s: 0x1FF,  // Initialize stack pointer to top of stack
            X: 0,
            Y: 0,
            b: false,
            c: false,
            d: false,
            e: true,   // Start in emulation mode
            i: true,   // Interrupts disabled on startup
            m: true,
            n: false,
            v: false,
            x: false,
            z: false,
            cycles: 0,
            opcode: 0,
            ram: [0; 131072],
        }
    }

    // Improved memory operations
    pub fn read_byte(&self, addr: u16) -> u8 {
        self.ram[addr as usize]
    }

    pub fn write_byte(&mut self, addr: u16, data: u8) {
        self.ram[addr as usize] = data;
    }

    pub fn read_word(&mut self, addr: u16) -> u16 {
        let lo = self.read_byte(addr) as u16;
        let hi = self.read_byte(addr.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    pub fn write_word(&mut self, addr: u16, data: u16) {
        self.write_byte(addr, (data & 0xFF) as u8);
        self.write_byte(addr.wrapping_add(1), (data >> 8) as u8);
    }

    // Flag management
    fn update_nz_flags(&mut self, value: u16) {
        self.n = (value & 0x8000) != 0;
        self.z = value == 0;
    }

    // Stack operations with 16-bit support
    pub fn push_word(&mut self, data: u16) {
        self.push((data >> 8) as u8);
        self.push(data as u8);
    }

    pub fn pop_word(&mut self) -> u16 {
        let lo = self.pop() as u16;
        let hi = self.pop() as u16;
        (hi << 8) | lo
    }

    // Replace existing read function with:
    pub fn read(&mut self, addr: u16) -> u16 {
        self.read_word(addr)
    }
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

} 

impl CPU {
    // match the opcode with the instructions
    // god help me there are 256 opcodes

    pub fn opcode(&mut self) { // opcode
        let opcode = self.opcode;
        let pc_before = self.pc;

        match opcode {

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
                    self.a = self.a | self.read(self.ram[self.dr as usize] as u16);
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
                
                0x09 => {
                    // ora #const
                    self.a = self.a | self.ram[self.pc as usize] as u16;
                    self.z = self.a == 0;
                    self.n = (self.a & 0x80) != 0;
                    self.c = false;
                    self.cycles += 2;
                    self.pc += 2;
                }

                0x0A => {
                    // ASL A (Accumulator)
                    if self.m {
                        // 8-bit mode
                        self.c = (self.a & 0x80) != 0;
                        self.a = (self.a << 1) & 0xFF;
                    } else {
                        // 16-bit mode
                        self.c = (self.a & 0x8000) != 0;
                        self.a = self.a << 1;
                    }
                    self.update_nz_flags(self.a);
                    self.cycles += 2;
                    self.pc = pc_before.wrapping_add(1);
                }
                
                _ => {
                  println!( "Invalid opcode/ work in progress");
                }


        }
    }

}
