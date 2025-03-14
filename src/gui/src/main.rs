#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::{Arc, Mutex};
use tauri::{Manager, State, Window};
use snes; // Import the snes crate.

#[derive(Default)]
struct AppState {
    emulator_screen_data: Arc<Mutex<Vec<u8>>>,
    opcode_trace: Arc<Mutex<Vec<String>>>,
}

// ... (rest of your Tauri commands and main function)