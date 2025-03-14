// crusty-com/src/main.rs

use std::env;
use emu::run as emu_run;
use gui::run as gui_run;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "--emu" => {
                println!("Running emulator...");
                emu_run(); // Call the run function from the emu crate
            }
            "--gui" => {
                println!("Launching GUI...");
                gui_run(); // Call the run function from the gui crate
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