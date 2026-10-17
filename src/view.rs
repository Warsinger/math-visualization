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

pub struct CanvasOutput {
    pub rect: egui::Rect,
    pub selected: Option<[f32; 3]>,
    pub tangent: Option<Result<math_playground::derivative::Tangent, String>>,
}

#[derive(Default)]
pub struct Overlays<'a> {
    pub tangent_source: Option<&'a str>,
    pub area: Option<&'a math_playground::integral::Area>,
    pub slice: Option<&'a math_playground::slice::Slice>,
    pub axis_markers: bool,
    pub axis_extent: Option<[f32; 3]>,
}

// Prefer 1/2/5 steps rather than rounding arbitrary fractional coordinates.
fn ticks(extent: f32) -> (f32, Vec<f32>) {
    let raw = extent / 5.0;
    let power = 10.0_f32.powf(raw.log10().floor());
    let fraction = raw / power;
    let step = power
        * if fraction <= 1.0 {
            1.0
        } else if fraction <= 2.0 {
            2.0
        } else if fraction <= 5.0 {
            5.0
        } else {
            10.0
        };
    let count = (extent / step + 1e-5).floor() as i32;
    (step, (-count..=count).map(|i| i as f32 * step).collect())
}
fn tick_label(value: f32, step: f32) -> String {
    if value.abs() < step * 1e-4 {
        return "0".into();
    }
    let precision = (-step.log10()).ceil().clamp(0.0, 6.0) as usize;
    let mut text = format!("{value:.precision$}");
    if text.contains('.') {
        text = text.trim_end_matches('0').trim_end_matches('.').into();
    }
    text
}

pub fn canvas(
    ui: &mut egui::Ui,
    geometry: &Geometry,
    camera: &mut Camera,
    height: f32,
    curve: Option<CurveSampling>,
    overlays: Overlays<'_>,
) -> CanvasOutput {
    let Overlays {
        tangent_source,
        area,
        slice,
        axis_markers,
        axis_extent,
    } = overlays;
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
        for axis in 0..2 {
            for value in ticks(settings.extent[axis]).1 {
                let mut a = [0.0; 3];
                let mut b = a;
                a[axis] = value;
                b[axis] = value;
                a[1 - axis] = -settings.extent[1 - axis];
                b[1 - axis] = settings.extent[1 - axis];
                let line = [a, b];
                painter.line_segment(line.map(project), Stroke::new(0.5, Color32::from_gray(45)));
            }
        }
    }
    let extents = curve
        .map(|s| [s.extent[0], s.extent[1], bound])
        .or(axis_extent)
        .unwrap_or([bound; 3]);
    let mut label_rects = Vec::new();
    for (end, label, color) in [
        (
            [extents[0], 0.0, 0.0],
            "x",
            Color32::from_rgb(240, 120, 120),
        ),
        (
            [0.0, extents[1], 0.0],
            "y",
            Color32::from_rgb(130, 200, 150),
        ),
        (
            [0.0, 0.0, extents[2]],
            "z",
            Color32::from_rgb(140, 170, 245),
        ),
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
        let start = end.map(|v| -v);
        painter.line_segment([project(start), project(end)], Stroke::new(1.0, color));
        label_rects.push(painter.text(
            project(end),
            egui::Align2::LEFT_BOTTOM,
            label,
            egui::FontId::proportional(14.0),
            color,
        ));
    }
    if axis_markers {
        let mut origin_drawn = false;
        for (axis, extent) in
            extents
                .into_iter()
                .enumerate()
                .take(if curve.is_some() { 2 } else { 3 })
        {
            let mut end = [0.0; 3];
            end[axis] = extent;
            let direction = project(end) - project(end.map(|v| -v));
            // Near end-on axes have no room for meaningful distinct labels.
            if direction.length() < 40.0 {
                continue;
            }
            let normal = egui::vec2(-direction.y, direction.x).normalized();
            let (step, values) = ticks(extent);
            for value in values {
                if value == 0.0 && origin_drawn {
                    continue;
                }
                let mut world = [0.0; 3];
                world[axis] = value;
                let point = project(world);
                let galley = painter.layout_no_wrap(
                    tick_label(value, step),
                    egui::FontId::proportional(10.0),
                    Color32::GRAY,
                );
                let center = point + normal * (8.0 + galley.size().y / 2.0);
                let bounds = egui::Rect::from_center_size(center, galley.size());
                if !rect.contains_rect(bounds)
                    || label_rects.iter().any(|r| r.expand(3.0).intersects(bounds))
                {
                    continue;
                }
                painter.line_segment(
                    [point - normal * 3.0, point + normal * 3.0],
                    Stroke::new(1.0, Color32::GRAY),
                );
                painter.galley(bounds.min, galley, Color32::GRAY);
                label_rects.push(bounds);
                origin_drawn |= value == 0.0;
            }
        }
    }
    if let (Some(area), Some(settings)) = (area, curve) {
        for point in [area.points.first(), area.points.last()]
            .into_iter()
            .flatten()
        {
            let x = point[0] as f32;
            painter.line_segment(
                [
                    project([x, -settings.extent[1], 0.0]),
                    project([x, settings.extent[1], 0.0]),
                ],
                Stroke::new(1.0, Color32::GRAY),
            );
        }
        let draw = |a: [f64; 2], b: [f64; 2]| {
            let positive = a[1] + b[1] >= 0.0;
            let color = if positive {
                Color32::from_rgba_unmultiplied(70, 210, 160, 65)
            } else {
                Color32::from_rgba_unmultiplied(240, 100, 130, 65)
            };
            let point = |p: [f64; 2]| {
                project([
                    p[0] as f32,
                    p[1].clamp(
                        -f64::from(settings.extent[1]),
                        f64::from(settings.extent[1]),
                    ) as f32,
                    0.0,
                ])
            };
            painter.add(egui::Shape::convex_polygon(
                vec![point([a[0], 0.0]), point(a), point(b), point([b[0], 0.0])],
                color,
                Stroke::NONE,
            ));
        };
        for pair in area.points.windows(2) {
            let [a, b] = [pair[0], pair[1]];
            if a[1].signum() != b[1].signum() && a[1] != 0.0 && b[1] != 0.0 {
                let t = a[1].abs() / (a[1].abs() + b[1].abs());
                let zero = [a[0] + t * (b[0] - a[0]), 0.0];
                draw(a, zero);
                draw(zero, b);
            } else {
                draw(a, b);
            }
        }
    }
    if let Some(slice) = slice {
        painter.add(egui::Shape::convex_polygon(
            slice.plane.map(project).to_vec(),
            Color32::from_rgba_unmultiplied(255, 190, 70, 18),
            Stroke::new(1.0, Color32::from_rgb(210, 150, 60)),
        ));
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
    let tangent = selected
        .zip(tangent_source)
        .map(|(point, source)| math_playground::derivative::tangent(source, f64::from(point[0])));
    if let (Some(Ok(tangent)), Some(settings)) = (&tangent, curve)
        && let Some(line) = tangent.segment(settings.extent)
    {
        painter.line_segment(line.map(project), Stroke::new(2.0, Color32::YELLOW));
    }
    if let Some(slice) = slice {
        for line in &slice.geometry.lines {
            painter.line_segment(line.map(project), Stroke::new(2.5, Color32::YELLOW));
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
    CanvasOutput {
        rect,
        selected,
        tangent,
    }
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
    fn tick_labels_are_distinct_accurate_and_never_negative_zero() {
        for extent in [0.1, 0.13, 0.37, 1.234, 3.0, 9.9, 20.0] {
            let (step, values) = ticks(extent);
            assert!(values.len() <= 11);
            let labels: Vec<_> = values.iter().map(|v| tick_label(*v, step)).collect();
            let unique: std::collections::HashSet<_> = labels.iter().collect();
            assert_eq!(unique.len(), labels.len());
            for (value, text) in values.iter().zip(labels) {
                assert!((*value - text.parse::<f32>().unwrap()).abs() < step * 0.001);
                assert_ne!(text, "-0");
                assert!(value.abs() <= extent + 1e-5);
            }
        }
        assert_eq!(tick_label(0.04, 0.02), "0.04");
        assert_eq!(tick_label(-0.0, 0.02), "0");
    }

    #[test]
    fn markers_toggle_in_both_dimensions_without_changing_geometry() {
        let geometry = Geometry {
            bound: 3.0,
            ..Default::default()
        };
        for planar in [true, false] {
            let mut positions = Vec::new();
            for (visible, yaw) in [(false, 0.6), (true, 0.6), (true, 1.1)] {
                let ctx = egui::Context::default();
                let mut camera = Camera {
                    yaw,
                    ..Default::default()
                };
                let mut out = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            Pos2::ZERO,
                            egui::vec2(650.0, 350.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        canvas(
                            ui,
                            &geometry,
                            &mut camera,
                            300.0,
                            planar.then(CurveSampling::default),
                            Overlays {
                                axis_markers: visible,
                                ..Default::default()
                            },
                        );
                    },
                );
                out.textures_delta.clear();
                let labels: Vec<_> = out
                    .shapes
                    .iter()
                    .filter_map(|s| match &s.shape {
                        egui::Shape::Text(t) => Some((t.galley.job.text.clone(), t.pos)),
                        _ => None,
                    })
                    .collect();
                assert!(labels.iter().any(|(s, _)| s == "x"));
                assert!(labels.iter().any(|(s, _)| s == "y"));
                assert_eq!(labels.iter().any(|(s, _)| s == "z"), !planar);
                let numbers: Vec<_> = labels
                    .iter()
                    .filter(|(s, _)| s.parse::<f32>().is_ok())
                    .cloned()
                    .collect();
                assert_eq!(!numbers.is_empty(), visible);
                assert!(numbers.iter().filter(|(s, _)| s == "0").count() <= 1);
                positions.push(numbers);
            }
            if !planar {
                assert_ne!(positions[1], positions[2]);
            }
            assert!(geometry.lines.is_empty());
        }
    }
    #[test]
    fn tangent_follows_the_same_hover_point_and_clears_when_inactive() {
        let ctx = egui::Context::default();
        let geometry = Geometry {
            lines: vec![
                [[-1.0, 1.0, 0.0], [0.0, 0.0, 0.0]],
                [[0.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
            ],
            bound: 2.0,
            ..Default::default()
        };
        let mut camera = Camera::default();
        let settings = CurveSampling::default();
        let mut rect = egui::Rect::NOTHING;
        for (x, source) in [
            (None, None),
            (Some(-0.5), Some("x^2")),
            (Some(0.5), Some("x^2")),
            (Some(0.5), None),
            (None, Some("x^2")),
            (Some(0.0), Some("0.00001*(x^2)^0.5")),
        ] {
            let pointer = x.map_or(Pos2::new(799.0, 599.0), |x: f32| {
                camera.project_2d([x, x.abs(), 0.0], rect.center(), 17.0)
            });
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                events: vec![egui::Event::PointerMoved(pointer)],
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                let result = canvas(
                    ui,
                    &geometry,
                    &mut camera,
                    200.0,
                    Some(settings),
                    Overlays {
                        tangent_source: source,
                        ..Default::default()
                    },
                );
                rect = result.rect;
                assert_eq!(result.selected.is_some(), x.is_some());
                match (x, source, result.tangent) {
                    (Some(x), Some("x^2"), Some(Ok(tangent))) => {
                        assert_eq!(tangent.x, f64::from(result.selected.unwrap()[0]));
                        assert!((tangent.x - f64::from(x)).abs() < 1e-4);
                        assert!((tangent.slope - 2.0 * f64::from(x)).abs() < 1e-4);
                    }
                    (Some(0.0), Some(_), Some(Err(_))) => {}
                    (None, _, None) | (_, None, None) => {}
                    _ => panic!("unexpected tangent state"),
                }
            });
            output.textures_delta.clear();
        }
    }
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
                    let output = canvas(
                        ui,
                        &geometry,
                        &mut camera,
                        200.0,
                        Some(CurveSampling::default()),
                        Overlays::default(),
                    );
                    rect = output.rect;
                    selected = output.selected;
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
