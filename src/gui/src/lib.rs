// crusty-com/src/gui/src/lib.rs (optional)

// If you want to use the gui crate as a library, add code here.
// You might define functions or structs that can be used by other parts of your application.

pub fn run() {
    // Example: Launch the GUI
    println!("Launching the GUI...");
    // ... your GUI initialization and event loop logic ...
    // If using tauri
    std::process::Command::new("cargo")
        .args(&["tauri","dev"])
        .current_dir("../../gui")
        .spawn()
        .expect("failed to execute process");

}

// Add any other public functions, structs, or enums here.