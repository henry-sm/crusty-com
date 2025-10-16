use std::env;
use emu::run as emu_run;
use gui::run as gui_run;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "--emu" => {
                // check for a rom file path
                if let Some(rom_path) = args.get(2) {
                    println!("running emulator with rom: {}", rom_path);
                    emu_run(rom_path); // pass the path to the emu's run function
                } else {
                    println!("error: please provide a path to a rom file.");
                    println!("usage: crusty-com --emu <path/to/rom>");
                }
            }
            "--gui" => {
                println!("launching gui...");
                gui_run();
            }
            "--help" | "-h" => {
                println!("usage: crusty-com --emu <path/to/rom>");
            }
            _ => {
                println!("invalid argument. use --help for usage.");
            }
        }
    } else {
        println!("usage: crusty-com --emu <path/to/rom>");
    }
}
