// crusty-com/src/emu/lib.rs

pub mod bus;
pub mod cpu;

// Add any public functions or structs here that you want to expose from the emu crate.

pub fn run() {
    // Example: Start the emulator
    println!("Starting the emulator...");
    // ... your emulator initialization and execution logic ...
    // Example: create a bus and cpu.
    let mut my_bus = bus::Bus::new();
    let mut my_cpu = cpu::Cpu::new();

    // Example: run the cpu for a certain number of cycles.
    for _i in 0..10 {
        my_cpu.tick(&mut my_bus);
    }
}

// Add any other public functions, structs, or enums here.