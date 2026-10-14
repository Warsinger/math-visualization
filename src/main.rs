mod view;

use eframe::egui;
use math_playground::{geometry::Sampling, scene::Scene};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use view::Camera;

struct Playground {
    scene: Scene,
    camera: Camera,
    request: String,
    feedback: String,
    error: bool,
    sampling: Sampling,
    pending: Option<Receiver<Result<Scene, String>>>,
}

impl Default for Playground {
    fn default() -> Self {
        Self {
            scene: Scene::default(),
            camera: Camera::default(),
            request: "create a helix with five turns".into(),
            feedback: "Explore with a request or controls.".into(),
            error: false,
            sampling: Sampling::default(),
            pending: None,
        }
    }
}

impl Playground {
    fn start(&mut self, work: impl FnOnce(&mut Scene) -> Result<(), String> + Send + 'static) {
        let mut candidate = self.scene.clone();
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.error = false;
        self.feedback = "Computing bounded geometry…".into();
        std::thread::spawn(move || {
            let result = work(&mut candidate).map(|()| candidate);
            let _ = tx.send(result);
        });
    }
    fn finish(&mut self) {
        let result = self.pending.as_ref().map(|rx| rx.try_recv());
        match result {
            Some(Ok(result)) => {
                self.pending = None;
                match result {
                    Ok(scene) => {
                        self.scene = scene;
                        self.error = false;
                        self.feedback = if self.scene.geometry.lines.is_empty() {
                            format!(
                                "No crossings found; {} tetrahedra skipped. This does not prove there are no real solutions.",
                                self.scene.geometry.skipped_tetrahedra
                            )
                        } else {
                            format!(
                                "Updated: {} lines; {} tetrahedra skipped for undefined/nonfinite samples.",
                                self.scene.geometry.lines.len(),
                                self.scene.geometry.skipped_tetrahedra
                            )
                        };
                        if let Some(surface) = &self.scene.surface {
                            self.sampling = surface.sampling;
                        }
                    }
                    Err(error) => {
                        self.error = true;
                        self.feedback = error;
                    }
                }
            }
            Some(Err(TryRecvError::Disconnected)) => {
                self.pending = None;
                self.error = true;
                self.feedback = "Geometry worker stopped; previous scene preserved.".into();
            }
            _ => {}
        }
    }
}

impl Playground {
    fn draw(&mut self, ui: &mut egui::Ui) {
        self.finish();
        if self.pending.is_some() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Mathematical playground");
            ui.label("Helices and numerical implicit surfaces in Cartesian x, y, z.");
            ui.separator();
            ui.add_enabled_ui(self.pending.is_none(), |ui| {
                ui.horizontal(|ui| {
                    let response = ui.add(egui::TextEdit::singleline(&mut self.request)
                        .desired_width((ui.available_width() - 85.0).max(100.0))
                        .hint_text("plot x^2 - y^2 - z^2 = 0"));
                    let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button("Apply").clicked() || enter {
                        let request = self.request.clone();
                        self.start(move |scene| scene.request(&request));
                    }
                });
            });
            let color = if self.error { egui::Color32::LIGHT_RED } else { egui::Color32::LIGHT_GREEN };
            ui.colored_label(color, &self.feedback);
            ui.collapsing("Request examples and equation syntax", |ui| {
                ui.label("create a helix with five turns · set radius to 2 and height to 6");
                ui.label("plot x^2 - y^2 - z^2 = 0 · plot x^2+y^2+z^2=4 · plot z-x*y");
                ui.label("Use x/y/z, numbers, + - * / ^ and parentheses. Explicit multiplication; no functions.");
                ui.label("Sampling can miss thin features, singularities and even roots; discontinuities can mislead.");
            });
            ui.separator();
            let canvas_height = (ui.available_height() - 120.0).max(120.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(255.0);
                    ui.add_enabled_ui(self.pending.is_none(), |ui| {
                        if let Some(surface) = &self.scene.surface {
                            let source = surface.source.clone();
                            ui.heading("Selected · Implicit surface");
                            for (axis, extent) in ["X", "Y", "Z"].into_iter().zip(&mut self.sampling.extent) {
                                ui.add(egui::Slider::new(extent, 0.1..=10.0).text(format!("{axis} ±")));
                            }
                            ui.add(egui::Slider::new(&mut self.sampling.resolution, 8..=40).text("Cells / axis"));
                            ui.label("Domain is symmetric about zero. Click values to type. Apply sampling to rebuild.");
                            if ui.button("Apply sampling").clicked() {
                                let sampling = self.sampling;
                                self.start(move |scene| scene.plot(&source, sampling));
                            }
                        } else {
                            ui.heading("Selected object · Helix");
                            let mut edited = self.scene.helix;
                            ui.add(egui::Slider::new(&mut edited.radius, 0.1..=5.0).text("Radius"));
                            ui.add(egui::Slider::new(&mut edited.height, 0.1..=20.0).text("Height"));
                            ui.add(egui::Slider::new(&mut edited.turns, 0.25..=20.0).text("Turns"));
                            ui.label("Click a number to type a precise value.");
                            if edited != self.scene.helix {
                                match self.scene.replace(edited) {
                                    Ok(()) => { self.error = false; self.feedback = "Helix updated from controls.".into(); }
                                    Err(error) => { self.error = true; self.feedback = error; }
                                }
                            }
                            ui.label(format!("Pitch: {:.3} units / turn", self.scene.helix.pitch()));
                        }
                    });
                    ui.separator();
                    if ui.button("Reset view").clicked() { self.camera = Camera::default(); }
                    ui.add(egui::Slider::new(&mut self.camera.zoom, 0.3..=4.0).text("Zoom"));
                    ui.add(egui::Slider::new(&mut self.camera.yaw, -std::f32::consts::PI..=std::f32::consts::PI).text("Orbit"));
                    ui.add(egui::Slider::new(&mut self.camera.tilt, -1.5..=1.5).text("Tilt"));
                    ui.label("Drag to rotate. Scroll over canvas to zoom.");
                });
                view::canvas(ui, &self.scene.geometry, &mut self.camera, canvas_height);
            });
            ui.separator();
            if let Some(surface) = &self.scene.surface {
                ui.label(format!("Equation: {}", surface.source));
                ui.label("Approximate wireframe; no depth occlusion. One equation generally defines a surface, not a space curve.");
            } else {
                ui.label("x = r cos(2πnt)    y = r sin(2πnt)    z = h(t − ½)    0 ≤ t ≤ 1");
                ui.label("Height stays fixed when turns change. Pitch = height / turns. Units are abstract.");
            }
        });
    }
}

impl eframe::App for Playground {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1150.0, 760.0])
            .with_min_inner_size([950.0, 760.0]),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_scene_types_generate_finite_headless_ui_shapes() {
        let mut app = Playground::default();
        for request in ["create a helix with five turns", "plot x^2-y^2-z^2=0"] {
            app.scene.request(request).unwrap();
            let ctx = egui::Context::default();
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1150.0, 760.0),
                )),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| app.draw(ui));
            // No GPU backend in this test: intentionally discard texture uploads.
            output.textures_delta.clear();
            assert!(!output.shapes.is_empty());
            for shape in output.shapes {
                let bounds = shape.shape.visual_bounding_rect();
                if bounds.is_positive() {
                    assert!(bounds.min.is_finite() && bounds.max.is_finite());
                }
            }
        }
    }

    #[test]
    fn worker_result_is_applied_atomically_and_errors_keep_previous_scene() {
        let mut app = Playground::default();
        app.start(|scene| scene.request("plot x^2-y^2-z^2=0"));
        let result = app
            .pending
            .take()
            .unwrap()
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        let (tx, rx) = mpsc::channel();
        tx.send(result).unwrap();
        app.pending = Some(rx);
        app.finish();
        assert!(app.scene.surface.is_some());
        assert!(app.pending.is_none());
        assert!(!app.error);
        let before = app.scene.geometry.lines.clone();
        let (tx, rx) = mpsc::channel();
        tx.send(Err("Bad equation".into())).unwrap();
        app.pending = Some(rx);
        app.finish();
        assert!(app.error);
        assert_eq!(app.scene.geometry.lines, before);
    }
}
