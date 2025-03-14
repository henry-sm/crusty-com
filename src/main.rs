// crusty-com/src/main.rs

use std::env;
pub mod emu;
pub mod gui;


fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "--emu" => {
                println!("Running emulator...");
                emu::run(); // Assuming run() is a function in emu/src/lib.rs
            }
            "--gui" => {
                println!("Launching GUI...");
                gui::run(); // Assuming run() is a function in gui/src/main.rs.
            }
            "--help" | "-h" => {
                println!("Usage: crusty-com [--emu | --gui]");
            }
            _ => {
                println!("Invalid argument. Use --help for usage.");
            }
        }
    } else {
        println!("Usage: crusty-com [--emu | --gui]");
    }
}