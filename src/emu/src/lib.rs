
pub mod bus;
pub mod cpu;
pub mod ppu;
pub mod apu;
pub mod cart;
pub mod input;


use minifb::{Window, WindowOptions, Key};
use std::time::Duration;


pub fn run() {
    println!("Starting the emulator...");

    let width = 256;
    let height = 224;
    let mut window = Window::new(
        "crusty-com",
        width,
        height,
        WindowOptions::default(),
    ).unwrap_or_else(|e| {
        panic!("Unable to open window: {}", e);
    });

    let mut buffer: Vec<u32> = vec![0; width * height];

    let mut my_bus = bus::Bus::new();
    let mut my_cpu = cpu::_65816::new();

    let mut ppu_cycle_counter: u32 = 0;
    let mut last_vblank_state = false;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Execute CPU instruction (assume ~4 cycles average per instruction)
        my_cpu.tick(&mut my_bus);
        
        // Convert to PPU cycles (rough approximation: 1 CPU cycle ≈ 1 PPU cycle)
        ppu_cycle_counter += 4;
        
        // Process PPU cycles
        while ppu_cycle_counter > 0 {
            my_bus.ppu.tick();
            
            // Render scanline at the start of a new scanline
            if my_bus.ppu.cycle == 0 && my_bus.ppu.scanline < 224 {
                my_bus.ppu.render_scanline();
            }
            
            ppu_cycle_counter -= 1;
        }
        
        // Trigger NMI when V-blank starts (falling edge detection)
        if my_bus.ppu.vblank && !last_vblank_state {
            my_bus.trigger_nmi(&mut my_cpu);
        }
        last_vblank_state = my_bus.ppu.vblank;

        // Copy PPU framebuffer to display buffer
        for (i, pixel) in my_bus.ppu.framebuffer.iter().enumerate() {
            buffer[i] = *pixel;
        }

        window.update_with_buffer(&buffer, width, height).unwrap();
        std::thread::sleep(Duration::from_millis(16));
    }
}