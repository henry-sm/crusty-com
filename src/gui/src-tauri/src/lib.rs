#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{State, Window, Emitter};
use emu::cpu::_65816;
use emu::bus::Bus;



struct EmuState {
    cpu: _65816,
    bus: Bus,
    screen_buffer: Arc<Mutex<Vec<u32>>>,
}


#[derive(Default)]
struct AppState {
    emulator: Arc<Mutex<Option<EmuState>>>,
}





#[tauri::command]
fn load_rom(path: String, state: State<AppState>, window: Window) -> Result<(), String> {
    println!("Loading ROM from: {}", path);

    let mut emu_state = EmuState {
        cpu: _65816::new(),
        bus: Bus::new(),
        screen_buffer: Arc::new(Mutex::new(vec![0; 256 * 224])),
    };

    // Load the ROM file into the cartridge
    emu_state.bus.cart.load_rom(&path).map_err(|e| e.to_string())?;

    *state.emulator.lock().unwrap() = Some(emu_state);

    // Start the emulator loop in a background thread
    let emu_arc = Arc::clone(&state.emulator);
    let window_clone = window.clone();
    thread::spawn(move || {
        loop {
            let mut guard = emu_arc.lock().unwrap();
            if let Some(emu) = guard.as_mut() {
                for _ in 0..100000 {
                    emu.cpu.tick(&mut emu.bus);
                }
                let mut screen = emu.screen_buffer.lock().unwrap();
                for pixel in screen.iter_mut() {
                    *pixel = rand::random::<u32>() | 0xFF_00_00_00;
                }
            }
            drop(guard);
            thread::sleep(Duration::from_millis(16));
        }
    });

    // Start a separate thread to send frame updates to the frontend
    let emu_arc_clone = Arc::clone(&state.emulator);
    thread::spawn(move || {
        loop {
            let guard = emu_arc_clone.lock().unwrap();
            if let Some(emu) = guard.as_ref() {
                let screen_data = emu.screen_buffer.lock().unwrap().clone();
                window_clone.emit("frame-update", &screen_data).unwrap();
            }
            drop(guard);
            thread::sleep(Duration::from_millis(16));
        }
    });

    Ok(())
}
    
    




#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![load_rom])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}