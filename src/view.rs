use eframe::egui::{self, Color32, Pos2, Stroke};
use math_playground::curve::CurveSampling;
use math_playground::expression::Equation;
use math_playground::geometry::Geometry;

/// Snap to the closest displayed segment within eight logical pixels. In 3D,
/// screen distance wins; this wireframe renderer does not resolve occlusion.
fn pick(
    lines: &[[[f32; 3]; 2]],
    pointer: Pos2,
    project: impl Fn([f32; 3]) -> Pos2,
) -> Option<[f32; 3]> {
    let mut best = 8.0_f32.powi(2);
    let mut selected = None;
    for [a, b] in lines {
        let pa = project(*a);
        let pb = project(*b);
        let edge = pb - pa;
        let t = if edge.length_sq() > 1e-12 {
            ((pointer - pa).dot(edge) / edge.length_sq()).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let distance = pointer.distance_sq(pa + t * edge);
        if distance.is_finite() && distance < best {
            best = distance;
            selected = Some(std::array::from_fn(|i| a[i] + t * (b[i] - a[i])));
        }
    }
    selected
}

pub struct Camera {
    pub yaw: f32,
    pub tilt: f32,
    pub zoom: f32,
    pub pan: [f32; 2],
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: 0.6,
            tilt: 0.35,
            zoom: 1.0,
            pan: [0.0; 2],
        }
    }
}

impl Camera {
    fn project_2d(&self, [x, y, _]: [f32; 3], center: Pos2, scale: f32) -> Pos2 {
        center + egui::vec2(x + self.pan[0], -y - self.pan[1]) * scale
    }
    /// Rotate around vertical Z, then tilt. Orthographic projection is deliberate.
    fn project(&self, [x, y, z]: [f32; 3], center: Pos2, scale: f32) -> Pos2 {
        let horizontal = x * self.yaw.cos() - y * self.yaw.sin();
        let depth = x * self.yaw.sin() + y * self.yaw.cos();
        let vertical = z * self.tilt.cos() - depth * self.tilt.sin();
        center + egui::vec2(horizontal, -vertical) * scale
    }
}

pub fn canvas(
    ui: &mut egui::Ui,
    geometry: &Geometry,
    camera: &mut Camera,
    height: f32,
    curve: Option<CurveSampling>,
    tangent: Option<math_playground::derivative::Tangent>,
) -> (egui::Rect, Option<[f32; 3]>) {
    let (response, painter) = ui.allocate_painter(
        egui::vec2(ui.available_width().max(50.0), height),
        egui::Sense::drag(),
    );
    let rect = response.rect;
    painter.rect_filled(rect, 8.0, Color32::from_rgb(15, 22, 34));
    let base_scale = curve.map_or(
        rect.width().min(rect.height()) * 0.40 / geometry.bound.max(0.1),
        |s| (rect.width() / (2.0 * s.extent[0])).min(rect.height() / (2.0 * s.extent[1])) * 0.85,
    );
    if response.dragged() && curve.is_some() {
        let delta = ui.input(|i| i.pointer.delta());
        camera.pan[0] += delta.x / (base_scale * camera.zoom);
        camera.pan[1] -= delta.y / (base_scale * camera.zoom);
    } else if response.dragged() {
        // Per-frame pointer motion, not cumulative drag distance.
        let delta = ui.input(|i| i.pointer.delta());
        camera.yaw += delta.x * 0.01;
        camera.yaw = (camera.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        camera.tilt = (camera.tilt + delta.y * 0.01).clamp(-1.5, 1.5);
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        camera.zoom = (camera.zoom * (scroll * 0.002).exp()).clamp(0.3, 4.0);
    }
    // Bounding sphere keeps the initial view in frame at every rotation.
    let bound = geometry.bound.max(0.1);
    let scale = base_scale * camera.zoom;
    let project = |point| {
        if curve.is_some() {
            camera.project_2d(point, rect.center(), scale)
        } else {
            camera.project(point, rect.center(), scale)
        }
    };
    if let Some(settings) = curve {
        for i in -5..=5 {
            let x = i as f32 * settings.extent[0] / 5.0;
            let y = i as f32 * settings.extent[1] / 5.0;
            for line in [
                [[x, -settings.extent[1], 0.0], [x, settings.extent[1], 0.0]],
                [[-settings.extent[0], y, 0.0], [settings.extent[0], y, 0.0]],
            ] {
                painter.line_segment(line.map(project), Stroke::new(0.5, Color32::from_gray(45)));
            }
            painter.text(
                project([x, 0.0, 0.0]),
                egui::Align2::LEFT_TOP,
                format!("{x:.1}"),
                egui::FontId::proportional(10.0),
                Color32::GRAY,
            );
            if i != 0 {
                painter.text(
                    project([0.0, y, 0.0]),
                    egui::Align2::LEFT_BOTTOM,
                    format!("{y:.1}"),
                    egui::FontId::proportional(10.0),
                    Color32::GRAY,
                );
            }
        }
    }
    for (end, label, color) in [
        ([bound, 0.0, 0.0], "x", Color32::from_rgb(240, 120, 120)),
        ([0.0, bound, 0.0], "y", Color32::from_rgb(130, 200, 150)),
        ([0.0, 0.0, bound], "z", Color32::from_rgb(140, 170, 245)),
    ] {
        if curve.is_some() && label == "z" {
            continue;
        }
        let end = if let Some(s) = curve {
            if label == "x" {
                [s.extent[0], 0.0, 0.0]
            } else {
                [0.0, s.extent[1], 0.0]
            }
        } else {
            end
        };
        let start = if curve.is_some() {
            end.map(|v| -v)
        } else {
            [0.0; 3]
        };
        painter.line_segment([project(start), project(end)], Stroke::new(1.0, color));
        painter.text(
            project(end),
            egui::Align2::LEFT_BOTTOM,
            label,
            egui::FontId::proportional(14.0),
            color,
        );
    }
    for (i, pair) in geometry.lines.iter().enumerate() {
        let t = i as f32 / geometry.lines.len().max(1) as f32;
        let color = Color32::from_rgb((80.0 + 150.0 * t) as u8, (210.0 - 70.0 * t) as u8, 240);
        painter.line_segment(
            [project(pair[0]), project(pair[1])],
            Stroke::new(1.2, color),
        );
    }
    let selected = if response.dragged() {
        None
    } else {
        response
            .hover_pos()
            .and_then(|pointer| pick(&geometry.lines, pointer, project))
    };
    if let Some(point) = selected {
        painter.circle_filled(project(point), 4.0, Color32::YELLOW);
    }
    if let (Some(tangent), Some(settings)) = (tangent, curve) {
        if let Some(line) = tangent.segment(settings.extent) {
            painter.line_segment(line.map(project), Stroke::new(2.0, Color32::YELLOW));
        }
        if tangent.y.abs() <= f64::from(settings.extent[1]) {
            painter.circle_filled(
                project([tangent.x as f32, tangent.y as f32, 0.0]),
                4.0,
                Color32::YELLOW,
            );
        }
    }
    painter.text(
        rect.left_top() + egui::vec2(14.0, 14.0),
        egui::Align2::LEFT_TOP,
        if curve.is_some() {
            "2D Cartesian · XY plane · drag to pan"
        } else {
            "Orthographic wireframe · no depth occlusion"
        },
        egui::FontId::proportional(14.0),
        Color32::GRAY,
    );
    (rect, selected)
}

/// Ordinary layout content: immediate and independent of tooltip timers.
pub fn inspection_readout(ui: &mut egui::Ui, point: Option<[f32; 3]>, equation: Option<&Equation>) {
    if let Some(point) = point {
        ui.monospace(format!(
            "x = {:.6}, y = {:.6}, z = {:.6}",
            point[0], point[1], point[2]
        ));
        ui.label(match equation {
            Some(equation) => match equation.evaluate(point.map(f64::from)) {
                Some(value) => format!("Residual F = {value:.6e}"),
                None => "Residual F is undefined or nonfinite at this point.".into(),
            },
            None => "Parametric helix: no implicit equation residual.".into(),
        });
    } else {
        ui.label("Point inspection: hover near a plotted line.");
        ui.label("Coordinates and residual appear here immediately; pause dragging to inspect.");
    }
    ui.small("Approximate segment point; F = left minus right (shorthand f(x) minus y). Not proof of exactness.");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hover_readout_is_visible_immediately_without_tooltip_delay() {
        for (source, expected) in [
            (Some("y=0"), "Residual F ="),
            (Some("ln(y)"), "undefined or nonfinite"),
            (None, "Parametric helix"),
        ] {
            let ctx = egui::Context::default();
            let geometry = Geometry {
                lines: vec![[[-1.0, 0.0, 0.0], [1.0, 0.0, 0.0]]],
                bound: 1.0,
                ..Default::default()
            };
            let equation = source.map(|s| Equation::parse(s).unwrap());
            let mut camera = Camera::default();
            let mut rect = egui::Rect::NOTHING;
            for hover in [false, true, false] {
                let mut selected = None;
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        Pos2::ZERO,
                        egui::vec2(800.0, 600.0),
                    )),
                    time: Some(0.0),
                    events: vec![egui::Event::PointerMoved(if hover {
                        rect.center()
                    } else {
                        Pos2::new(799.0, 599.0)
                    })],
                    ..Default::default()
                };
                let mut output = ctx.run_ui(input, |ui| {
                    (rect, selected) = canvas(
                        ui,
                        &geometry,
                        &mut camera,
                        200.0,
                        Some(CurveSampling::default()),
                        None,
                    );
                    inspection_readout(ui, selected, equation.as_ref());
                });
                output.textures_delta.clear();
                let texts: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|s| match &s.shape {
                        egui::Shape::Text(t) => Some(t.galley.job.text.as_str()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(selected.is_some(), hover);
                assert!(
                    texts.iter().any(|s| s.contains(if hover {
                        expected
                    } else {
                        "Point inspection:"
                    })),
                    "{texts:?}"
                );
                if hover {
                    assert!(texts.iter().any(|s| s.contains("x = 0.000000")));
                }
            }
        }
    }
    #[test]
    fn picking_interpolates_world_coordinates_and_rejects_distant_points() {
        let lines = [[[0.0, 0.0, 2.0], [10.0, 0.0, 4.0]]];
        let project = |p: [f32; 3]| Pos2::new(p[0] * 10.0, p[1] * 10.0);
        assert_eq!(
            pick(&lines, Pos2::new(25.0, 2.0), project),
            Some([2.5, 0.0, 2.5])
        );
        assert_eq!(pick(&lines, Pos2::new(25.0, 9.0), project), None);
        assert_eq!(pick(&[], Pos2::ZERO, project), None);
        assert_eq!(
            pick(&[[[0.0, 0.0, 2.0], [0.0, 0.0, 4.0]]], Pos2::ZERO, project),
            Some([0.0, 0.0, 2.0])
        );
        let point = pick(&lines, Pos2::new(25.0, 2.0), project).unwrap();
        assert_eq!(
            Equation::parse("x-z")
                .unwrap()
                .evaluate(point.map(f64::from)),
            Some(0.0)
        );
        assert_eq!(
            Equation::parse("ln(y)")
                .unwrap()
                .evaluate(point.map(f64::from)),
            None
        );
    }
    #[test]
    fn planar_projection_is_xy_with_pan_and_ignores_orbit_and_depth() {
        let camera = Camera {
            yaw: 2.0,
            tilt: 1.0,
            pan: [1.0, -1.0],
            ..Default::default()
        };
        assert_eq!(
            camera.project_2d([2.0, 3.0, 99.0], Pos2::ZERO, 10.0),
            Pos2::new(30.0, -20.0)
        );
    }

    #[test]
    fn projection_preserves_center_and_maps_vertical_up() {
        let camera = Camera {
            yaw: 0.0,
            tilt: 0.0,
            zoom: 1.0,
            pan: [0.0; 2],
        };
        let center = Pos2::new(100.0, 100.0);
        assert_eq!(camera.project([0.0; 3], center, 10.0), center);
        assert_eq!(
            camera.project([2.0, 0.0, 3.0], center, 10.0),
            Pos2::new(120.0, 70.0)
        );
    }

    #[test]
    fn quarter_orbit_moves_y_axis_into_horizontal_view() {
        let camera = Camera {
            yaw: std::f32::consts::FRAC_PI_2,
            tilt: 0.0,
            zoom: 1.0,
            pan: [0.0; 2],
        };
        let point = camera.project([0.0, 2.0, 0.0], Pos2::ZERO, 10.0);
        assert!((point.x + 20.0).abs() < 1e-5);
        assert!(point.y.abs() < 1e-5);
    }
}
