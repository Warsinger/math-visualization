#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
mod layout;
#[cfg(not(target_arch = "wasm32"))]
mod startup;
mod view;
#[cfg(target_arch = "wasm32")]
mod web;

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
    history_width: f32,
    #[cfg(not(target_arch = "wasm32"))]
    layout_checked: bool,
    pending_history_request: Option<String>,
    plot_mode: PlotMode,
    curve_sampling: CurveSampling,
    tangent_enabled: bool,
    area_enabled: bool,
    area_bounds: [f64; 2],
    area_cache: math_playground::integral::Cache,

    slice_enabled: bool,
    slice_axis: usize,
    slice_position: f32,
    slice_cache: math_playground::slice::Cache,
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
            history: Store::session(),
            history_open: false,
            history_width: 300.0,
            #[cfg(not(target_arch = "wasm32"))]
            layout_checked: false,
            pending_history_request: None,
            plot_mode: PlotMode::Auto,
            curve_sampling: CurveSampling::default(),
            tangent_enabled: false,
            area_enabled: false,
            area_bounds: [-1.0, 1.0],
            area_cache: Default::default(),

            slice_enabled: false,
            slice_axis: 2,
            slice_position: 0.0,
            slice_cache: Default::default(),
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
        let task = move || {
            let result = work(&mut candidate).map(|()| candidate);
            let _ = tx.send(result);
        };
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(task);
        #[cfg(target_arch = "wasm32")]
        task();
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
        let max_width = (ui.available_width() - 600.0).clamp(0.0, 360.0);
        // A monitor smaller than the normal minimum must still retain a canvas.
        if max_width < 120.0 {
            return None;
        }
        let panel = egui::Panel::right("history_panel")
            .default_size(self.history_width.clamp(120.0,max_width))
            .min_size(260.0_f32.min(max_width))
            .max_size(max_width)
            .resizable(true)
            .show(ui, |ui| {
                ui.style_mut().wrap_mode=Some(egui::TextWrapMode::Wrap);
                ui.horizontal(|ui| {
                    ui.heading("History");
                    if ui.button("Hide").clicked() { self.history_open=false; }
                });
                ui.label("Newest first · up to 200 entries · timestamps in UTC");
                #[cfg(target_arch = "wasm32")]
                ui.small("Browser session only: history resets when this page reloads.");
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
                                ui.label(egui::RichText::new(&entry.request).font(egui::FontId::monospace(14.0)));
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
        self.history_width = panel.response.rect.width().clamp(260.0, 360.0);
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
                        .font(egui::FontId::monospace(14.0))
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
                ui.label("Functions: ln, log/logN, sqrt/cbrt/abs, sin/cos/tan/sec/csc/cot, asin/acos/atan/atan2, sinh/cosh/tanh/asinh/acosh/atanh, min/max/sign.");
                ui.label("Products: 2x, 2(x+1), 2sin(x), x y, 2pi or 2π. Names stay whole: use x*y, not xy; functions require parentheses, e.g. sin(x), not sin x.");
                ui.label("Multiplication and division share precedence, left to right: 1/2x = (1/2)*x. Use 1/(2x) for a denominator product. Powers bind first: 2x^2 = 2*(x^2).");
                ui.label("Scientific notation: 2e3 = 2000; 2e-3 = 0.002. For Euler's constant use 2*e or 2 e; 2e and 2e(x) are invalid. Put numeric coefficients first or use '*': x*2, not x 2.");
                ui.label("Examples: log(2x,2), cbrt(x), atan2(2y,3x). Trig uses radians. Separate function arguments with commas.");
                ui.label("Sampling can miss thin features, singularities and even roots; discontinuities can mislead.");
            });
            ui.separator();
            let canvas_height = (ui.available_height() - 210.0).max(120.0);
            let mut inspected_point = None;
            let mut inspection_equation = None;
            let mut tangent = None;
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(255.0);
                    ui.add_enabled_ui(self.pending.is_none(), |ui| {
                        if let Some(curve)=&self.scene.curve {
                            let source=curve.source.clone();
                            let applied_extent=f64::from(curve.sampling.extent[0]);
                            ui.heading("Selected · 2D curve");
                            for (axis,extent) in ["X ±","Y ±"].into_iter().zip(&mut self.curve_sampling.extent) {
                                ui.add(egui::Slider::new(extent,0.1..=20.0).text(axis));
                            }
                            ui.add(egui::Slider::new(&mut self.curve_sampling.samples,64..=4096).text("Samples"));
                            ui.label(format!("Apply sampling to rebuild. Explicit functions use sample intervals; implicit curves use a {}×{} grid.",math_playground::curve::grid_cells(self.curve_sampling),math_playground::curve::grid_cells(self.curve_sampling)));
                            if ui.button("Apply sampling").clicked() {
                                let settings=self.curve_sampling;
                                self.pending_history_request=Some(format!("plot {source}"));
                                let source = source.clone();
                                self.start(move |scene|scene.plot_curve(&source,settings));
                            }
                            ui.checkbox(&mut self.tangent_enabled, "Explore tangent");
                            if self.tangent_enabled {
                                if math_playground::expression::Equation::explicit_y(&source).is_ok() {
                                    ui.label("Hover along the curve to explore the tangent at the inspected point.");
                                    ui.small("One-sided finite differences; convergence is approximate, not symbolic proof.");
                                } else { ui.label("Tangent exploration requires explicit y=f(x)."); }
                            }
                            ui.checkbox(&mut self.area_enabled,"Explore integral / area");
                            if self.area_enabled {
                                if math_playground::expression::Equation::explicit_y(&self.scene.curve.as_ref().unwrap().source).is_ok() {
                                    for (label,bound) in ["From a","To b"].into_iter().zip(&mut self.area_bounds) {
                                        *bound=bound.clamp(-applied_extent,applied_extent);
                                        ui.add(egui::Slider::new(bound,-applied_extent..=applied_extent).text(label));
                                    }
                                    let current_source=&self.scene.curve.as_ref().unwrap().source;
                                    match self.area_cache.get(current_source,self.area_bounds[0],self.area_bounds[1]) {
                                        Ok(area) => {
                                            ui.label(format!("Signed integral ≈ {:.6}",area.signed));
                                            ui.label(format!("Total area ≈ {:.6}",area.total));
                                        }
                                        Err(error) => {ui.label(error);}
                                    }
                                    ui.small("Numerical Simpson estimate; no continuity proof. Green above / red below x-axis. Reversing bounds negates integral, not total area.");
                                } else {ui.label("Integration requires explicit y=f(x).");}
                            }
                        } else if let Some(surface) = &self.scene.surface {
                            let source = surface.source.clone();
                            let applied_sampling = surface.sampling;
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
                            ui.checkbox(&mut self.slice_enabled, "Explore cross-section");
                            if self.slice_enabled {
                                ui.horizontal(|ui| {
                                    for (axis, label) in ["X", "Y", "Z"].into_iter().enumerate() {
                                        ui.selectable_value(&mut self.slice_axis, axis, label);
                                    }
                                });
                                let extent = applied_sampling.extent[self.slice_axis];
                                self.slice_position = self.slice_position.clamp(-extent, extent);
                                ui.add(egui::Slider::new(&mut self.slice_position, -extent..=extent).text("Plane position"));
                                let current_source = self.scene.surface.as_ref().unwrap().source.clone();
                                match self.slice_cache.get(&current_source, applied_sampling, self.slice_axis, self.slice_position) {
                                    Ok(slice) => { ui.label(format!("{} intersection segments; {} undefined regions", slice.geometry.lines.len(), slice.geometry.skipped_tetrahedra)); }
                                    Err(error) => { ui.label(error); }
                                }
                                ui.small("Yellow: numerical intersection. Empty may mean missed/degenerate features. No depth occlusion.");
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
                    ui.label("Hover near a line to inspect coordinates and residual.");
                });
                inspection_equation = if let Some(curve) = &self.scene.curve {
                    math_playground::expression::Equation::planar(&curve.source).ok()
                } else if let Some(surface) = &self.scene.surface {
                    math_playground::expression::Equation::parse(&surface.source).ok()
                } else { None };
                let tangent_source = if self.tangent_enabled { self.scene.curve.as_ref().map(|curve|curve.source.as_str()) } else { None };
                let area=if self.area_enabled {
                    self.scene.curve.as_ref().and_then(|curve|self.area_cache.get(&curve.source,self.area_bounds[0],self.area_bounds[1]).as_ref().ok())
                } else {None};
                let slice = if self.slice_enabled {
                    self.scene.surface.as_ref().and_then(|surface| self.slice_cache.get(&surface.source, surface.sampling, self.slice_axis, self.slice_position).as_ref().ok())
                } else { None };
                let output=view::canvas(ui, &self.scene.geometry, &mut self.camera, canvas_height,self.scene.curve.as_ref().map(|c|c.sampling), view::Overlays {tangent_source,area,slice});
                canvas_rect=output.rect;
                inspected_point=output.selected;
                tangent=output.tangent;
            });
            ui.allocate_ui(egui::vec2(ui.available_width(), 85.0), |ui| {
                view::inspection_readout(ui, inspected_point, inspection_equation.as_ref());
                if self.tangent_enabled && self.scene.curve.is_some() {
                    match &tangent {
                        Some(Ok(value)) => { ui.label(format!("Numerical f'(x) ≈ {:.6} at the inspected x",value.slope)); }
                        Some(Err(error)) => { ui.label(error); }
                        None => { ui.label("Tangent: hover near the curve to select a point."); }
                    }
                }
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
        #[cfg(not(target_arch = "wasm32"))]
        if !self.layout_checked {
            layout::fit_to_monitor(ui.ctx());
            self.layout_checked = true;
        }
        self.draw(ui);
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        layout::Layout {
            history_open: self.history_open,
            history_width: self.history_width,
            ..Default::default()
        }
        .save(storage);
    }
    fn persist_egui_memory(&self) -> bool {
        false
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    let simulate_error = std::env::args_os()
        .skip(1)
        .any(|arg| arg == "--simulate-startup-error");
    match run(simulate_error) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            startup::report(&error.to_string());
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run(simulate_error: bool) -> eframe::Result {
    if simulate_error {
        return Err(eframe::Error::AppCreation(Box::new(std::io::Error::other(
            "Simulated startup failure requested with --simulate-startup-error. Remove this option to start normally.",
        ))));
    }
    let options = eframe::NativeOptions {
        persistence_path: layout::storage_path(),
        persist_window: true,
        window_builder: Some(Box::new(layout::sanitize_window)),
        viewport: egui::ViewportBuilder::default()
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon/app-256.png"))
                    .expect("bundled application icon must be a valid PNG"),
            )
            .with_inner_size([1150.0, 760.0])
            .with_min_inner_size([950.0, 760.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Mathematical playground",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            let saved_layout = layout::Layout::load(cc.storage);
            Ok(Box::new(Playground {
                history: Store::local(),
                history_open: saved_layout.history_open,
                history_width: saved_layout.history_width,
                ..Default::default()
            }))
        }),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    web::launch();
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn restored_panel_width_is_clamped_and_small_windows_keep_canvas_space() {
        for (width, expected_panel) in [(1150.0, true), (950.0, true), (650.0, false)] {
            let ctx = egui::Context::default();
            let mut app = Playground {
                history_open: true,
                history_width: 340.0,
                ..Default::default()
            };
            for _ in 0..2 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 760.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        let available = ui.available_width();
                        let panel = app.history_ui(ui);
                        assert_eq!(panel.is_some(), expected_panel);
                        assert!(ui.available_width() >= available.min(600.0) - 2.0);
                        if let Some(panel) = panel {
                            assert!(panel.width() <= 360.0);
                        }
                    },
                );
                output.textures_delta.clear();
            }
            assert!(app.history_open);
        }
    }

    #[test]
    fn exploration_controls_leave_bottom_inspection_visible_at_minimum_size() {
        let mut app = Playground {
            tangent_enabled: true,
            area_enabled: true,
            history_open: true,
            ..Default::default()
        };
        app.scene
            .plot_curve("x^2", CurveSampling::default())
            .unwrap();
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(950.0, 760.0),
                )),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                app.draw(ui);
            });
            output.textures_delta.clear();
            let text = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t)
                        if t.galley.job.text.contains("Approximate segment point;") =>
                    {
                        Some(t)
                    }
                    _ => None,
                })
                .expect("bottom inspection readout must remain visible");
            assert!(
                text.pos.y + text.galley.size().y <= 760.0,
                "readout extends below window"
            );
        }
    }

    #[test]
    fn simulated_startup_failure_returns_before_window_creation() {
        let error = run(true).unwrap_err().to_string();
        assert!(error.contains("Simulated startup failure"));
        assert!(error.contains("--simulate-startup-error"));
    }

    #[test]
    fn all_scene_types_generate_finite_headless_ui_shapes() {
        let mut app = Playground {
            tangent_enabled: true,
            area_enabled: true,

            slice_enabled: true,
            ..Default::default()
        };
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
                    tangent_enabled: true,
                    area_enabled: true,

                    slice_enabled: true,
                    ..Default::default()
                };
                if mode == PlotMode::TwoD {
                    app.scene
                        .plot_curve("x^2", CurveSampling::default())
                        .unwrap();
                    app.plot_mode = mode;
                } else {
                    app.scene
                        .plot("x^2+y^2+z^2=1", Sampling::default())
                        .unwrap();
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
