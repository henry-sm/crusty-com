#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use emu::cpu::_65816;
use emu::bus::Bus;

// Global emulator state
pub struct EmuState {
    pub cpu: _65816,
    pub bus: Bus,
    pub running: bool,
    pub total_cycles: u64,
    pub frame_count: u64,
}

pub struct AppState {
    pub emulator: Arc<Mutex<EmuState>>,
}

#[tauri::command]
fn init_emu() -> Result<String, String> {
    println!("Emulator initialized");
    Ok("OK".to_string())
}

#[tauri::command]
fn get_status() -> String {
    "Ready".to_string()
}

#[tauri::command]
fn load_rom(path: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    use std::io::{self, Write};
    
    println!("Loading ROM from: {}", path);
    let _ = io::stdout().flush();
    
    let mut emu_state = state.emulator.lock().unwrap();
    
    // Load ROM into cartridge
    match emu_state.bus.cart.load_rom(&path) {
        Ok(_) => {
            println!("ROM loaded successfully");
            let _ = io::stdout().flush();
            
            // Read reset vector from cartridge to initialize CPU
            let vector_lo = emu_state.bus.read(0x00, 0xFFFC) as u16;
            let vector_hi = emu_state.bus.read(0x00, 0xFFFD) as u16;
            eprintln!("[DEBUG] Vector bytes: lo=0x{:02X}, hi=0x{:02X}", vector_lo, vector_hi);
            let _ = io::stderr().flush();
            
            let reset_pc = (vector_hi << 8) | vector_lo;
            emu_state.cpu.pc = reset_pc;
            eprintln!("CPU reset: PC set to ${:04X}", reset_pc);
            println!("CPU reset: PC set to ${:04X}", reset_pc);
            let _ = io::stdout().flush();
            let _ = io::stderr().flush();
            
            emu_state.running = true;
            println!("Starting emulation loop...");
            let _ = io::stdout().flush();
            Ok(format!("ROM loaded: {}", path))
        }
        Err(e) => {
            eprintln!("Failed to load ROM: {}", e);
            Err(format!("Failed to load ROM: {}", e))
        }
    }
}

#[tauri::command]
fn get_frame(state: tauri::State<'_, AppState>) -> Vec<u32> {
    let emu = state.emulator.lock().unwrap();
    // Log diagnostics every 10 frames (~600ms)
    if emu.frame_count % 10 == 0 && emu.frame_count > 0 {
        eprintln!("[GUI] Frame {}: {} cycles, CPU PC=0x{:04X}, VRAM filled: {} bytes", 
            emu.frame_count, 
            emu.total_cycles,
            emu.cpu.pc,
            emu.bus.ppu.vram.iter().filter(|b| **b != 0).count()
        );
    }
    // Convert Box<[u32; 57344]> to Vec<u32>
    emu.bus.ppu.framebuffer.to_vec()
}

#[tauri::command]
fn press_button(button: u8, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut emu = state.emulator.lock().unwrap();
    emu.bus.input.set_button(button as usize, true);
    Ok(())
}

#[tauri::command]
fn release_button(button: u8, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut emu = state.emulator.lock().unwrap();
    emu.bus.input.set_button(button as usize, false);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    println!("Starting Tauri application...");
    
    // Create emulator state in a dedicated thread to avoid stack overflow
    let emu_arc = Arc::new(Mutex::new(EmuState {
        cpu: _65816::new(),
        bus: Bus::new(),
        running: false,
        total_cycles: 0,
        frame_count: 0,
    }));
    
    let app_state = AppState {
        emulator: Arc::clone(&emu_arc),
    };
    
    let emu_arc_clone = Arc::clone(&emu_arc);
    
    // Start emulation thread
    thread::spawn(move || {
        loop {
            {
                let mut emu_guard = emu_arc_clone.lock().unwrap();
                
                if emu_guard.running {
                    // Run CPU cycles for one frame (~88657 cycles at 21.477 MHz for NTSC)
                    for _ in 0..88657 {
                        // Use unsafe to bypass borrow checker since we know they're separate fields
                        unsafe {
                            let cpu_ptr: *mut _65816 = &mut emu_guard.cpu;
                            let bus_ptr: *mut Bus = &mut emu_guard.bus;
                            (*cpu_ptr).tick(&mut *bus_ptr);
                        }
                    }
                    emu_guard.total_cycles += 88657;
                    emu_guard.frame_count += 1;
                    
                    // Every 100 frames, log VRAM status
                    if emu_guard.frame_count % 100 == 0 {
                        eprintln!("[EMU] Frame {}: PC=0x{:04X}, total_cycles={}, VRAM non-zero bytes: {}", 
                            emu_guard.frame_count,
                            emu_guard.cpu.pc,
                            emu_guard.total_cycles,
                            emu_guard.bus.ppu.vram.iter().filter(|b| **b != 0).count()
                        );
                    }
                }
            } // Lock is released here
            
            thread::sleep(Duration::from_millis(16)); // ~60 FPS
        }
    });
    
    match tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            init_emu, 
            get_status, 
            load_rom, 
            get_frame, 
            press_button, 
            release_button
        ])
        .build(tauri::generate_context!()) {
            Ok(app) => {
                println!("Running app...");
                app.run(|_app, event| {
                    println!("Event: {:?}", event);
                });
            }
            Err(e) => {
                eprintln!("Failed to build app: {}", e);
            }
        }
}