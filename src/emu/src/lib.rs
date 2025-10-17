
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

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Emulator tick
        my_cpu.tick(&mut my_bus);

        // TODO: Update buffer with framebuffer data from PPU
        // For now, fill with a color pattern
        for i in 0..buffer.len() {
            buffer[i] = 0x00336699; // ARGB
        }

        window.update_with_buffer(&buffer, width, height).unwrap();
        std::thread::sleep(Duration::from_millis(16));
    }
}