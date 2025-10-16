
pub mod bus;
pub mod cpu;
pub mod ppu;
pub mod apu;
pub mod cart;
pub mod input;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;


pub fn run() {
    println!("Starting the emulator...");
    
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    
    let window = video_subsystem.window("crusty-com", 256, 224)
        .position_centered()
        .build()
        .unwrap();
        
    let mut canvas = window.into_canvas().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    
    let mut my_bus = bus::Bus::new();
    let mut my_cpu = cpu::_65816::new();
    
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
        }

        my_cpu.tick(&mut my_bus);
        
        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}