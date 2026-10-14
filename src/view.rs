use eframe::egui::{self, Color32, Pos2, Stroke};
use math_playground::geometry::Geometry;

pub struct Camera {
    pub yaw: f32,
    pub tilt: f32,
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: 0.6,
            tilt: 0.35,
            zoom: 1.0,
        }
    }
}

impl Camera {
    /// Rotate around vertical Z, then tilt. Orthographic projection is deliberate.
    fn project(&self, [x, y, z]: [f32; 3], center: Pos2, scale: f32) -> Pos2 {
        let horizontal = x * self.yaw.cos() - y * self.yaw.sin();
        let depth = x * self.yaw.sin() + y * self.yaw.cos();
        let vertical = z * self.tilt.cos() - depth * self.tilt.sin();
        center + egui::vec2(horizontal, -vertical) * scale
    }
}

pub fn canvas(ui: &mut egui::Ui, geometry: &Geometry, camera: &mut Camera, height: f32) {
    let (response, painter) = ui.allocate_painter(
        egui::vec2(ui.available_width().max(50.0), height),
        egui::Sense::drag(),
    );
    let rect = response.rect;
    painter.rect_filled(rect, 8.0, Color32::from_rgb(15, 22, 34));
    if response.dragged() {
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
    let scale = rect.width().min(rect.height()) * 0.40 / bound * camera.zoom;
    let project = |point| camera.project(point, rect.center(), scale);
    for (end, label, color) in [
        ([bound, 0.0, 0.0], "x", Color32::from_rgb(240, 120, 120)),
        ([0.0, bound, 0.0], "y", Color32::from_rgb(130, 200, 150)),
        ([0.0, 0.0, bound], "z", Color32::from_rgb(140, 170, 245)),
    ] {
        painter.line_segment([project([0.0; 3]), project(end)], Stroke::new(1.0, color));
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
    painter.text(
        rect.left_top() + egui::vec2(14.0, 14.0),
        egui::Align2::LEFT_TOP,
        "Orthographic wireframe · no depth occlusion",
        egui::FontId::proportional(14.0),
        Color32::GRAY,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_preserves_center_and_maps_vertical_up() {
        let camera = Camera {
            yaw: 0.0,
            tilt: 0.0,
            zoom: 1.0,
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
        };
        let point = camera.project([0.0, 2.0, 0.0], Pos2::ZERO, 10.0);
        assert!((point.x + 20.0).abs() < 1e-5);
        assert!(point.y.abs() < 1e-5);
    }
}
