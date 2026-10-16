mod view;

use eframe::egui;
use math_playground::{
    curve::CurveSampling,
    geometry::Sampling,
    history::{Outcome, Snapshot, Store, truncate},
    scene::{PlotMode, Scene},
};
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
    history: Store,
    history_open: bool,
    pending_history_request: Option<String>,
    plot_mode: PlotMode,
    curve_sampling: CurveSampling,
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
            history: Store::open(None),
            history_open: false,
            pending_history_request: None,
            plot_mode: PlotMode::Auto,
            curve_sampling: CurveSampling::default(),
        }
    }
}

impl Playground {
    fn restore_snapshot(&mut self, snapshot: Snapshot) {
        self.plot_mode = if matches!(snapshot, Snapshot::Curve { .. }) {
            PlotMode::TwoD
        } else {
            PlotMode::ThreeD
        };
        self.request = snapshot.request();
        self.start(move |scene| snapshot.restore(scene));
    }
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
                        if self.plot_mode != PlotMode::Auto {
                            self.plot_mode = if self.scene.curve.is_some() {
                                PlotMode::TwoD
                            } else {
                                PlotMode::ThreeD
                            };
                        }
                        if let Some(c) = &self.scene.curve {
                            self.curve_sampling = c.sampling;
                        }
                        self.error = false;
                        self.feedback = if self.scene.curve.is_some() {
                            format!(
                                "2D plot: {} segments; {} sampling regions skipped. Empty output does not prove no real values exist.",
                                self.scene.geometry.lines.len(),
                                self.scene.geometry.skipped_tetrahedra
                            )
                        } else if self.scene.geometry.lines.is_empty() {
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
        if self.pending.is_none()
            && let Some(request) = self.pending_history_request.take()
        {
            let outcome = if self.error {
                Outcome::Error {
                    message: truncate(&self.feedback, 1024),
                }
            } else {
                Outcome::Success {
                    snapshot: Snapshot::capture(&self.scene),
                }
            };
            self.history.record(request, outcome);
        }
    }

    fn history_ui(&mut self, ui: &mut egui::Ui) -> Option<egui::Rect> {
        if !self.history_open {
            return None;
        }
        let mut restore = None;
        let mut retry = None;
        let panel = egui::Panel::right("history_panel")
            .default_size(300.0)
            .min_size(260.0)
            .max_size((ui.available_width()-600.0).clamp(260.0,360.0))
            .resizable(true)
            .show(ui, |ui| {
                ui.style_mut().wrap_mode=Some(egui::TextWrapMode::Wrap);
                ui.horizontal(|ui| {
                    ui.heading("History");
                    if ui.button("Hide").clicked() { self.history_open=false; }
                });
                ui.label("Newest first · up to 200 entries · timestamps in UTC");
            ui.small("Restore uses saved plot settings. Edit/retry fills the input; current settings apply when submitted.");
                if let Some(path) = &self.history.path {
                    ui.label(format!("Storage: {}", path.display()));
                }
                if let Some(warning) = &self.history.warning {
                    ui.colored_label(egui::Color32::YELLOW, warning);
                }
                ui.add_enabled_ui(self.pending.is_none(), |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Add current plot").clicked() {
                            let snapshot = Snapshot::capture(&self.scene);
                            self.history
                                .record(snapshot.request(), Outcome::Success { snapshot });
                        }
                        if ui.button("Clear history").clicked() {
                            self.history.clear();
                        }
                    });
                    if self.history.history.entries.is_empty() {
                        ui.label("No history yet. Apply a request or add the current plot.");
                    }
                    egui::ScrollArea::vertical()
                        .max_height(ui.available_height())
                        .show(ui, |ui| {
                            for entry in self.history.history.entries.iter().rev() {
                                ui.separator();
                                let date =
                                    chrono::DateTime::from_timestamp(entry.timestamp as i64, 0)
                                        .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                                        .unwrap_or_else(|| "Unknown time".into());
                                ui.small(date);
                                ui.label(&entry.request);
                                match &entry.outcome {
                                    Outcome::Success { snapshot } => {
                                        ui.colored_label(egui::Color32::LIGHT_GREEN, "Succeeded");
                                        ui.small(if matches!(snapshot,Snapshot::Curve {..}){"2D curve"}else{"3D plot"});
                                        if let Snapshot::Surface { sampling, .. } = snapshot {
                                            ui.small(format!(
                                                "Bounds ±{:?}; {} cells/axis",
                                                sampling.extent, sampling.resolution
                                            ));
                                        }
                                        ui.horizontal(|ui| {
                                            if ui.button("Restore").clicked() {
                                                restore = Some(snapshot.clone());
                                            }
                                            if ui.button("Edit/retry").clicked() {
                                                self.plot_mode=if matches!(snapshot,Snapshot::Curve {..}){PlotMode::TwoD}else{PlotMode::ThreeD};
                                                retry = Some(snapshot.request());
                                            }
                                        });
                                    }
                                    Outcome::Error { message } => {
                                        ui.colored_label(
                                            egui::Color32::LIGHT_RED,
                                            format!("Failed: {message}"),
                                        );
                                        if ui.button("Edit/retry").clicked() {
                                            retry = Some(entry.request.clone());
                                        }
                                    }
                                }
                            }
                        });
                });
            });
        if let Some(request) = retry {
            self.request = request;
        }
        if let Some(snapshot) = restore {
            self.restore_snapshot(snapshot);
        }
        Some(panel.response.rect)
    }
}

impl Playground {
    fn draw(&mut self, ui: &mut egui::Ui) -> (egui::Rect, Option<egui::Rect>) {
        self.finish();
        if self.pending.is_some() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
        let history_rect = self.history_ui(ui);
        let mut canvas_rect = egui::Rect::NOTHING;
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Mathematical playground");
            ui.label("Cartesian curves, helices and implicit surfaces.");
            ui.add_enabled_ui(self.pending.is_none(),|ui| {
                ui.horizontal(|ui| {
                    ui.label("Input mode:");
                    ui.selectable_value(&mut self.plot_mode,PlotMode::Auto,"Auto");
                    ui.selectable_value(&mut self.plot_mode,PlotMode::TwoD,"2D · XY");
                    ui.selectable_value(&mut self.plot_mode,PlotMode::ThreeD,"3D · implicit");
                });
                ui.small("Auto: equations with z use 3D; otherwise 2D. Bare x^2 means y=x^2. Overrides apply on the next request. Helices are 3D.");
            });
            ui.separator();
            ui.add_enabled_ui(self.pending.is_none(), |ui| {
                ui.horizontal(|ui| {
                    let response = ui.add(egui::TextEdit::singleline(&mut self.request)
                        .font(egui::FontId::monospace(16.0))
                        .desired_width((ui.available_width() - 85.0).max(100.0))
                        .hint_text("plot x^2 - y^2 - z^2 = 0"));
                    let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button("Apply").clicked() || enter {
                        let request = self.request.clone();
                        let mode=self.plot_mode;
                        self.pending_history_request=Some(request.clone());
                        self.start(move |scene| scene.request_in_mode(&request,mode));
                    }
                });
            });
            let color = if self.error { egui::Color32::LIGHT_RED } else { egui::Color32::LIGHT_GREEN };
            ui.colored_label(color, &self.feedback);
            ui.horizontal(|ui| {
                if ui.selectable_label(self.history_open,"History").clicked() { self.history_open = !self.history_open; }
                ui.small(format!("{} entries",self.history.history.entries.len()));
            });
            if let Some(warning)=&self.history.warning { ui.colored_label(egui::Color32::YELLOW,warning); }
            ui.collapsing("Request examples and equation syntax", |ui| {
                ui.label("create a helix with five turns · set radius to 2 and height to 6");
                ui.label("plot x^2 - y^2 - z^2 = 0 · plot x^2+y^2+z^2=4 · plot z-x*y");
                ui.label("plot ln(x)=y · plot sin(x)=z · plot cos(x)=y · plot tan(x)=z");
                ui.label("Use x/y/z, numbers, + - * / ^, parentheses, ln/sin/cos/tan. Trig uses radians; multiplication is explicit.");
                ui.label("Sampling can miss thin features, singularities and even roots; discontinuities can mislead.");
            });
            ui.separator();
            let canvas_height = (ui.available_height() - 120.0).max(120.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(255.0);
                    ui.add_enabled_ui(self.pending.is_none(), |ui| {
                        if let Some(curve)=&self.scene.curve {
                            let source=curve.source.clone();
                            ui.heading("Selected · 2D curve");
                            for (axis,extent) in ["X ±","Y ±"].into_iter().zip(&mut self.curve_sampling.extent) {
                                ui.add(egui::Slider::new(extent,0.1..=20.0).text(axis));
                            }
                            ui.add(egui::Slider::new(&mut self.curve_sampling.samples,64..=4096).text("Samples"));
                            ui.label(format!("Apply sampling to rebuild. Explicit functions use sample intervals; implicit curves use a {}×{} grid.",math_playground::curve::grid_cells(self.curve_sampling),math_playground::curve::grid_cells(self.curve_sampling)));
                            if ui.button("Apply sampling").clicked() {
                                let settings=self.curve_sampling;
                                self.pending_history_request=Some(format!("plot {source}"));
                                self.start(move |scene|scene.plot_curve(&source,settings));
                            }
                        } else if let Some(surface) = &self.scene.surface {
                            let source = surface.source.clone();
                            ui.heading("Selected · Implicit surface");
                            for (axis, extent) in ["X", "Y", "Z"].into_iter().zip(&mut self.sampling.extent) {
                                ui.add(egui::Slider::new(extent, 0.1..=10.0).text(format!("{axis} ±")));
                            }
                            ui.add(egui::Slider::new(&mut self.sampling.resolution, 8..=40).text("Cells / axis"));
                            ui.label("Domain is symmetric about zero. Click values to type. Apply sampling to rebuild.");
                            if ui.button("Apply sampling").clicked() {
                                let sampling = self.sampling;
                                self.pending_history_request=Some(format!("plot {source}"));
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
                    if self.scene.curve.is_none() {
                        ui.add(egui::Slider::new(&mut self.camera.yaw, -std::f32::consts::PI..=std::f32::consts::PI).text("Orbit"));
                        ui.add(egui::Slider::new(&mut self.camera.tilt, -1.5..=1.5).text("Tilt"));
                    }
                    ui.label(if self.scene.curve.is_some(){"Drag to pan. Scroll to zoom."}else{"Drag to rotate. Scroll to zoom."});
                });
                canvas_rect=view::canvas(ui, &self.scene.geometry, &mut self.camera, canvas_height,self.scene.curve.as_ref().map(|c|c.sampling));
            });
            ui.separator();
            if let Some(curve)=&self.scene.curve {
                ui.label(format!("2D Cartesian: {}",curve.source));
                ui.label("Uniform numerical samples; undefined/sharp intervals may be skipped, and discontinuities can still mislead.");
            } else if let Some(surface) = &self.scene.surface {
                ui.label(format!("Equation: {}", surface.source));
                ui.label("Approximate wireframe; no depth occlusion. One equation generally defines a surface, not a space curve.");
            } else {
                ui.label("x = r cos(2πnt)    y = r sin(2πnt)    z = h(t − ½)    0 ≤ t ≤ 1");
                ui.label("Height stays fixed when turns change. Pitch = height / turns. Units are abstract.");
            }
        });
        (canvas_rect, history_rect)
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
            Ok(Box::new(Playground {
                history: Store::local(),
                ..Default::default()
            }))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_scene_types_generate_finite_headless_ui_shapes() {
        let mut app = Playground::default();
        for (request, mode) in [
            ("create a helix with five turns", PlotMode::ThreeD),
            ("plot x^2-y^2-z^2=0", PlotMode::ThreeD),
            ("x^2", PlotMode::TwoD),
        ] {
            app.scene.request_in_mode(request, mode).unwrap();
            app.plot_mode = mode;
            let ctx = egui::Context::default();
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1150.0, 760.0),
                )),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                app.draw(ui);
            });
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
        let mut app = Playground {
            pending_history_request: Some("plot x^2-y^2-z^2=0".into()),
            ..Default::default()
        };
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
        assert_eq!(app.history.history.entries.len(), 1);
        assert!(matches!(
            app.history.history.entries[0].outcome,
            Outcome::Success { .. }
        ));
        let before = app.scene.geometry.lines.clone();
        let (tx, rx) = mpsc::channel();
        tx.send(Err("Bad equation".into())).unwrap();
        app.pending = Some(rx);
        app.pending_history_request = Some("plot bad".into());
        app.finish();
        assert!(app.error);
        assert_eq!(app.scene.geometry.lines, before);
        assert_eq!(app.history.history.entries.len(), 2);
        assert!(matches!(
            app.history.history.entries[1].outcome,
            Outcome::Error { .. }
        ));
        app.finish();
        assert_eq!(app.history.history.entries.len(), 2);
    }

    #[test]
    fn history_panel_renders_success_and_failure_without_filesystem_access() {
        let mut app = Playground {
            history_open: true,
            ..Default::default()
        };
        app.history.record(
            "create a helix with five turns".into(),
            Outcome::Success {
                snapshot: Snapshot::capture(&app.scene),
            },
        );
        app.history.record(
            "bad request".into(),
            Outcome::Error {
                message: "Unsupported".into(),
            },
        );
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            app.draw(ui);
        });
        output.textures_delta.clear();
        assert!(!output.shapes.is_empty());
        assert_eq!(app.history.history.entries.len(), 2);
    }

    #[test]
    fn docked_history_never_overlaps_canvas_and_hiding_restores_width() {
        for size in [egui::vec2(950.0, 760.0), egui::vec2(1150.0, 760.0)] {
            for mode in [PlotMode::TwoD, PlotMode::ThreeD] {
                let mut app = Playground {
                    history_open: true,
                    ..Default::default()
                };
                if mode == PlotMode::TwoD {
                    app.scene
                        .plot_curve("x^2", CurveSampling::default())
                        .unwrap();
                    app.plot_mode = mode;
                }
                let ctx = egui::Context::default();
                let input = || egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    ..Default::default()
                };
                let mut layout = (egui::Rect::NOTHING, None);
                for _ in 0..2 {
                    let mut output = ctx.run_ui(input(), |ui| {
                        layout = app.draw(ui);
                    });
                    output.textures_delta.clear();
                }
                let (canvas, history) = layout;
                let history = history.expect("history is docked");
                assert!(
                    !canvas.intersects(history),
                    "{canvas:?} overlaps {history:?}"
                );
                assert!(canvas.width() > 250.0);
                app.history_open = false;
                let mut output = ctx.run_ui(input(), |ui| {
                    layout = app.draw(ui);
                });
                output.textures_delta.clear();
                assert!(layout.1.is_none());
                assert!(layout.0.width() > canvas.width() + 200.0);
            }
        }
    }

    #[test]
    fn restored_curve_updates_input_mode_and_controls_without_adding_history() {
        let mut app = Playground::default();
        let sampling = CurveSampling {
            extent: [3.0, 9.0],
            samples: 128,
        };
        let snapshot = Snapshot::Curve {
            equation: "x^2".into(),
            sampling,
        };
        app.restore_snapshot(snapshot);
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
        assert_eq!(app.plot_mode, PlotMode::TwoD);
        assert_eq!(app.curve_sampling, sampling);
        assert!(app.history.history.entries.is_empty());
    }
}
