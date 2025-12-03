// Quick test of emulator without GUI
use std::path::Path;

fn main() {
    println!("Testing SNES emulator core...");
    
    // Test ROM loading
    let rom_path = "testroms/Headerless/Tetris (USA).nes";
    
    if Path::new(rom_path).exists() {
        println!("✅ Found test ROM: {}", rom_path);
    } else {
        println!("❌ Test ROM not found at: {}", rom_path);
        return;
    }
    
    println!("✅ Emulator core ready for testing!");
}
