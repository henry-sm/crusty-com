
pub mod bus;
pub mod cpu;
pub mod ppu;
pub mod apu;
pub mod cart;
pub mod input;
pub mod mapper;

// Note: minifb CLI mode has been removed in favor of Tauri GUI
// For GUI mode, use: cd src/gui/src-tauri && cargo tauri dev