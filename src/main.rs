mod view;

use eframe::egui;
use math_playground::{command, scene::Scene};
use view::Camera;

struct Playground {
    scene: Scene,
    camera: Camera,
    request: String,
    feedback: String,
    error: bool,
}

impl Default for Playground {
    fn default() -> Self {
        Self {
            scene: Scene::default(),
            camera: Camera::default(),
            request: "create a helix with five turns".into(),
            feedback: "Explore the helix with a request or the controls.".into(),
            error: false,
        }
    }
}

impl eframe::App for Playground {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Mathematical playground");
            ui.label("A helix, a few numbers, and room to explore.");
            ui.separator();
            ui.horizontal(|ui| {
                let response = ui.add(egui::TextEdit::singleline(&mut self.request)
                    .desired_width((ui.available_width() - 85.0).max(100.0))
                    .hint_text("Describe your helix…"));
                let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button("Apply").clicked() || enter {
                    match command::parse(&self.request, self.scene.helix)
                        .and_then(|helix| self.scene.replace(helix)) {
                        Ok(()) => { self.error = false; self.feedback = "Helix updated.".into(); }
                        Err(error) => { self.error = true; self.feedback = error; }
                    }
                }
            });
            let color = if self.error { egui::Color32::LIGHT_RED } else { egui::Color32::LIGHT_GREEN };
            ui.colored_label(color, &self.feedback);
            ui.collapsing("Request examples", |ui| {
                ui.label("create a helix with five turns");
                ui.label("set radius to 2 and height to 6");
                ui.label("set pitch to 0.5 and turns to 8");
                ui.label("Supported: radius, height, turns, pitch; digits or one–ten. More wording comes later.");
            });
            ui.separator();
            let canvas_height = (ui.available_height() - 155.0).max(120.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(230.0);
                    ui.heading("Selected object · Helix");
                    let mut edited = self.scene.helix;
                    ui.add(egui::Slider::new(&mut edited.radius, 0.1..=5.0).text("Radius"));
                    ui.add(egui::Slider::new(&mut edited.height, 0.1..=20.0).text("Height"));
                    ui.add(egui::Slider::new(&mut edited.turns, 0.25..=20.0).text("Turns"));
                    ui.label("Click a slider's number to type a precise value.");
                    if edited != self.scene.helix {
                        match self.scene.replace(edited) {
                            Ok(()) => { self.error = false; self.feedback = "Helix updated from controls.".into(); }
                            Err(error) => { self.error = true; self.feedback = error; }
                        }
                    }
                    ui.label(format!("Pitch: {:.3} units / turn", self.scene.helix.pitch()));
                    ui.separator();
                    if ui.button("Reset view").clicked() { self.camera = Camera::default(); }
                    ui.add(egui::Slider::new(&mut self.camera.zoom, 0.3..=4.0).text("Zoom"));
                    ui.add(egui::Slider::new(&mut self.camera.yaw, -std::f32::consts::PI..=std::f32::consts::PI).text("Orbit"));
                    ui.add(egui::Slider::new(&mut self.camera.tilt, -1.5..=1.5).text("Tilt"));
                    ui.label("Drag the canvas to rotate. Scroll over it to zoom.");
                });
                view::canvas(ui, self.scene.helix, &mut self.camera, canvas_height);
            });
            ui.separator();
            ui.label("x = r cos(2πnt)    y = r sin(2πnt)    z = h(t − ½)    0 ≤ t ≤ 1");
            ui.label("Height stays fixed when turns change. Pitch = height / turns. Units are abstract.");
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1150.0, 760.0])
            .with_min_inner_size([850.0, 620.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Mathematical playground",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(Playground::default()))
        }),
    )
}
