// gui/emulator_window.rs
use egui::{TextureHandle, Ui, Label};
use std::sync::{Arc, Mutex};

pub struct EmulatorWindow {}

impl EmulatorWindow {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, ui: &mut Ui, texture: Option<&TextureHandle>, opcode_trace: Arc<Mutex<Vec<String>>>) {
        ui.horizontal(|ui| {
            if let Some(texture) = texture {
                let image = egui::Image::new(texture, egui::Vec2::new(256.0, 224.0));
                ui.add(image);
            } else {
                ui.label("Emulator Screen Placeholder");
            }

            ui.vertical(|ui| {
                let trace = opcode_trace.lock().unwrap();
                let max_lines = 14; // Adjust this to fit your desired number of lines
                let start_index = if trace.len() > max_lines {
                    trace.len() - max_lines
                } else {
                    0
                }
                for opcode in trace[start_index..].iter() {
                    ui.add(Label::new(opcode.clone()));
                }
            });
        });
    }
}
