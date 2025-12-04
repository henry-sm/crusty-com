#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::{Arc, Mutex};
use emu::cpu::_65816;
use emu::bus::Bus;

// Global emulator state
pub struct EmuState {
    pub cpu: _65816,
    pub bus: Bus,
    pub running: bool,
}

#[derive(Default)]
pub struct AppState {
    pub emulator: Arc<Mutex<EmuState>>,
}

impl Default for EmuState {
    fn default() -> Self {
        EmuState {
            cpu: _65816::new(),
            bus: Bus::new(),
            running: false,
        }
    }
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
    println!("Loading ROM from: {}", path);
    
    let mut emu_state = state.emulator.lock().unwrap();
    
    // Load ROM into cartridge
    match emu_state.bus.cart.load_rom(&path) {
        Ok(_) => {
            println!("ROM loaded successfully");
            emu_state.running = true;
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
    
    match tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
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