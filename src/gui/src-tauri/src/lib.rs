#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;
use emu::cpu::_65816;
use emu::bus::Bus;
use tauri::Emitter;

// Global app handle for sending events
static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

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

fn emit_log(msg: &str) {
    if let Some(handle) = APP_HANDLE.get() {
        let _ = handle.emit("log-message", msg);
    }
}

#[tauri::command]
fn init_emu() -> Result<String, String> {
    Ok("OK".to_string())
}

#[tauri::command]
fn get_status() -> String {
    "Ready".to_string()
}

#[tauri::command]
fn load_rom(path: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    let msg = format!("Loading ROM from: {}", path);
    emit_log(&msg);
    
    let mut emu_state = state.emulator.lock().unwrap();
    
    // Load ROM into cartridge
    match emu_state.bus.cart.load_rom(&path) {
        Ok(_) => {
            emit_log("ROM loaded successfully");
            
            // Read reset vector from cartridge to initialize CPU
            let vector_lo = emu_state.bus.read(0x00, 0xFFFC) as u16;
            let vector_hi = emu_state.bus.read(0x00, 0xFFFD) as u16;
            let debug_msg = format!("[DEBUG] Vector bytes: lo=0x{:02X}, hi=0x{:02X}", vector_lo, vector_hi);
            emit_log(&debug_msg);
            
            let reset_pc = (vector_hi << 8) | vector_lo;
            emu_state.cpu.pc = reset_pc;
            let reset_msg = format!("CPU reset: PC set to ${:04X}", reset_pc);
            emit_log(&reset_msg);
            
            emu_state.running = true;
            emit_log("Starting emulation loop...");
            Ok(format!("ROM loaded: {}", path))
        }
        Err(e) => {
            let err_msg = format!("Failed to load ROM: {}", e);
            emit_log(&err_msg);
            Err(err_msg)
        }
    }
}

#[tauri::command]
fn get_frame(state: tauri::State<'_, AppState>) -> Vec<u32> {
    let emu = state.emulator.lock().unwrap();
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

#[tauri::command]
fn save_sram(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let emu = state.emulator.lock().unwrap();
    match emu.bus.cart.save_sram() {
        Ok(_) => {
            emit_log("SRAM saved successfully");
            Ok("SRAM saved".to_string())
        }
        Err(e) => {
            let err_msg = format!("Failed to save SRAM: {}", e);
            emit_log(&err_msg);
            Err(err_msg)
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Create emulator state in a dedicated thread to avoid stack overflow
    let emu_arc = Arc::new(Mutex::new(EmuState {
        cpu: _65816::new(),
        bus: Bus::new(),
        running: false,
        total_cycles: 0,
        frame_count: 0,
    }));
    
    let emu_arc_clone = Arc::clone(&emu_arc);
    
    match tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            // Store the app handle globally
            let _ = APP_HANDLE.set(app.handle().clone());
            
            let emu_arc_for_thread = Arc::clone(&emu_arc_clone);
            
            // Start emulation thread
            thread::spawn(move || {
                loop {
                    {
                        let mut emu_guard = emu_arc_for_thread.lock().unwrap();
                        
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
                                let msg = format!("[EMU] Frame {}: PC=0x{:04X}, total_cycles={}, VRAM non-zero bytes: {}", 
                                    emu_guard.frame_count,
                                    emu_guard.cpu.pc,
                                    emu_guard.total_cycles,
                                    emu_guard.bus.ppu.vram.iter().filter(|b| **b != 0).count()
                                );
                                emit_log(&msg);
                            }
                        }
                    } // Lock is released here
                    
                    thread::sleep(Duration::from_millis(16)); // ~60 FPS
                }
            });
            
            Ok(())
        })
        .manage(AppState {
            emulator: Arc::clone(&emu_arc),
        })
        .invoke_handler(tauri::generate_handler![
            init_emu, 
            get_status, 
            load_rom, 
            get_frame, 
            press_button, 
            release_button,
            save_sram
        ])
        .build(tauri::generate_context!()) {
            Ok(app) => {
                app.run(|_app, _event| {
                    // Silent
                });
            }
            Err(e) => {
                eprintln!("Failed to build app: {}", e);
            }
        }
}