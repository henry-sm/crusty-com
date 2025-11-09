#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
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
        let mut frame_count = 0;
        let mut last_time = Instant::now();
        
        loop {
            let mut guard = emu_arc.lock().unwrap();
            if let Some(emu) = guard.as_mut() {
                // Run CPU cycles for one frame (~88657 cycles at 21.477 MHz for NTSC)
                for _ in 0..88657 {
                    emu.cpu.tick(&mut emu.bus);
                }
                
                // Copy PPU framebuffer to screen buffer
                {
                    let mut screen = emu.screen_buffer.lock().unwrap();
                    for (i, pixel) in emu.bus.ppu.framebuffer.iter().enumerate() {
                        if i < screen.len() {
                            screen[i] = *pixel;
                        }
                    }
                }
                
                frame_count += 1;
                let elapsed = last_time.elapsed();
                if elapsed >= Duration::from_secs(1) {
                    println!("FPS: {}", frame_count);
                    frame_count = 0;
                    last_time = Instant::now();
                }
            }
            drop(guard);
            
            // Emit frame update to frontend
            let guard = emu_arc.lock().unwrap();
            if let Some(emu) = guard.as_ref() {
                let screen_data = emu.screen_buffer.lock().unwrap().clone();
                let _ = window_clone.emit("frame-update", &screen_data);
            }
            drop(guard);
            
            // Frame rate limiting (~60 FPS)
            thread::sleep(Duration::from_millis(16));
        }
    });

    Ok(())
}

#[tauri::command]
fn press_button(button: u8, state: State<AppState>) -> Result<(), String> {
    let mut guard = state.emulator.lock().unwrap();
    if let Some(emu) = guard.as_mut() {
        emu.bus.input.set_button(button as usize, true);
    }
    Ok(())
}

#[tauri::command]
fn release_button(button: u8, state: State<AppState>) -> Result<(), String> {
    let mut guard = state.emulator.lock().unwrap();
    if let Some(emu) = guard.as_mut() {
        emu.bus.input.set_button(button as usize, false);
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![load_rom, press_button, release_button])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}