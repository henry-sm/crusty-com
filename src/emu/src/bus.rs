use ::cpu::CPU;


pub struct Bus {
    ppu : PPU,
    apu : APU,
    cart : Cartridge,
    cpu : CPU,

}

