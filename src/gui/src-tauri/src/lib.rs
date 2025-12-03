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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    println!("Starting Tauri application...");
    
    match tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![init_emu, get_status])
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