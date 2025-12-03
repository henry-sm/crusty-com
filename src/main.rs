use std::env;
use emu::bus::Bus;
use emu::cpu::_65816;
use std::process::Command;

fn main() {
    println!("Crusty SNES Emulator");
    
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "--gui" => {
                println!("Launching GUI...");
                // Spawn gui_main as a separate process
                let status = Command::new("./target/debug/deps/gui_main.exe")
                    .status()
                    .expect("Failed to launch GUI");
                std::process::exit(status.code().unwrap_or(1));
            }
            "--rom" => {
                if args.len() < 3 {
                    println!("usage: crusty-com --rom <path>");
                    return;
                }
                run_emulator(&args[2]);
            }
            "--help" | "-h" => {
                println!("usage: crusty-com [--gui|--rom <path>]");
                println!("  --gui:        Launch the GUI (disabled)");
                println!("  --rom <path>: Load and run a ROM file");
                println!("  --help:       Show this message");
            }
            _ => {
                println!("invalid argument. use --help for usage.");
            }
        }
    } else {
        // Default to help if no args provided
        println!("usage: crusty-com [--gui|--rom <path>]");
        println!("Use --help for more information");
    }
}

fn run_emulator(rom_path: &str) {
    println!("Loading ROM: {}", rom_path);
    
    // Check if file exists
    if !std::path::Path::new(rom_path).exists() {
        eprintln!("Error: ROM file not found: {}", rom_path);
        return;
    }

    let mut cpu = _65816::new();
    let mut bus = Bus::new();

    // Load ROM
    match bus.cart.load_rom(rom_path) {
        Ok(_) => println!("ROM loaded successfully"),
        Err(e) => {
            eprintln!("Error loading ROM: {}", e);
            return;
        }
    }

    println!("Running emulator...");
    println!("Press Ctrl+C to exit");

    // Simple emulation loop - run for a limited number of frames for testing
    for frame in 0..300 {
        // Run CPU cycles for one frame (~88657 cycles at 21.477 MHz for NTSC)
        for _ in 0..88657 {
            cpu.tick(&mut bus);
        }

        if frame % 60 == 0 {
            println!("Frame: {}", frame);
        }
    }

    println!("Emulation complete");
}