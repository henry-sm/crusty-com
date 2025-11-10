use std::env;
use gui_lib::run as gui_run;


fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "--gui" => {
                println!("launching gui...");
                gui_run();
            }
            "--help" | "-h" => {
                println!("usage: crusty-com --gui");
            }
            _ => {
                println!("invalid argument. use --help for usage.");
            }
        }
    } else {
        println!("usage: crusty-com --gui");
    }
}