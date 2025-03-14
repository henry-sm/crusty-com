// main.rs
use egui::{ColorImage, TextureHandle, TextureOptions};
use egui_wgpu::wgpu;
use gui::EmulatorWindow;
use std::sync::{Arc, Mutex};

mod gui;

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "SNES Emulator GUI",
        options,
        Box::new(|cc| Box::new(MyApp::new(cc))),
    )
    .unwrap();
}

struct MyApp {
    emulator_window: EmulatorWindow,
    emulator_screen_texture: Option<TextureHandle>,
    emulator_screen_data: Arc<Mutex<Vec<u8>>>,
    opcode_trace: Arc<Mutex<Vec<String>>>,
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let gl = cc.gl.as_ref().expect("You need to run egui with OpenGL");
        let emulator_screen_data = Arc::new(Mutex::new(vec![0; 256 * 224 * 4]));
        let opcode_trace = Arc::new(Mutex::new(Vec::new()));
        Self {
            emulator_window: EmulatorWindow::new(),
            emulator_screen_texture: None,
            emulator_screen_data,
            opcode_trace,
        }
    }

    fn update_emulator_screen(&mut self, ctx: &egui::Context, gl: &egui_wgpu::glow::Context) {
        let screen_data = self.emulator_screen_data.lock().unwrap();
        let color_image = ColorImage::from_rgba_unmultiplied([256, 224], &screen_data);

        if self.emulator_screen_texture.is_none() {
            let texture_options = TextureOptions::default();
            self.emulator_screen_texture = Some(ctx.load_texture("emulator_screen", color_image, texture_options));
        } else {
            self.emulator_screen_texture
                .as_mut()
                .unwrap()
                .set(color_image, TextureOptions::default());
        }
    }

    pub fn add_opcode_to_trace(&mut self, opcode: String) {
        let mut trace = self.opcode_trace.lock().unwrap();
        trace.push(opcode);
        if trace.len() > 100 {
            trace.remove(0);
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let gl = frame
            .gl()
            .expect("You need to run egui with OpenGL for custom painting");

        self.update_emulator_screen(ctx, gl);

        egui::CentralPanel::default().show(ctx, |ui| {
            self.emulator_window.show(ui, self.emulator_screen_texture.as_ref(), self.opcode_trace.clone());
        });
    }
}