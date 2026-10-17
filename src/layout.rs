//! Narrow, versioned layout preferences stored through eframe's native storage.
use eframe::egui;
use serde::{Deserialize, Serialize};
pub const KEY: &str = "workspace_layout_v1";
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub version: u32,
    pub history_open: bool,
    pub history_width: f32,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            version: 1,
            history_open: false,
            history_width: 300.0,
        }
    }
}
impl Layout {
    pub fn load(storage: Option<&dyn eframe::Storage>) -> Self {
        storage
            .and_then(|s| s.get_string(KEY))
            .and_then(|s| Self::decode(&s))
            .unwrap_or_default()
    }
    fn decode(text: &str) -> Option<Self> {
        if text.len() > 1024 {
            return None;
        }
        let value: Self = serde_json::from_str(text).ok()?;
        if value.version != 1 || !value.history_width.is_finite() {
            return None;
        }
        Some(Self {
            history_width: value.history_width.clamp(260.0, 360.0),
            ..value
        })
    }
    pub fn save(&self, storage: &mut dyn eframe::Storage) {
        if let Ok(text) = serde_json::to_string(self) {
            storage.set_string(KEY, text);
        }
    }
}
pub fn storage_path() -> Option<std::path::PathBuf> {
    directories::BaseDirs::new().map(|b| b.data_local_dir().join("MathPlayground/workspace-v1.ron"))
}
// eframe clamps persisted window size/monitor position before this hook. Reject
// nonfinite values without enlarging the already monitor-clamped size. The
// normal UI minimum is applied only once the active monitor is known.
pub fn sanitize_window(mut viewport: egui::ViewportBuilder) -> egui::ViewportBuilder {
    let size = viewport
        .inner_size
        .filter(|s| s.is_finite() && s.x > 0.0 && s.y > 0.0)
        .unwrap_or(egui::vec2(64.0, 64.0));
    viewport.inner_size = Some(size.min(egui::vec2(16384.0, 16384.0)));
    viewport.min_inner_size = Some(egui::vec2(950.0, 760.0).min(size));
    if viewport
        .position
        .is_some_and(|p| !p.is_finite() || p.x.abs() > 100_000.0 || p.y.abs() > 100_000.0)
    {
        viewport.position = None;
    }
    viewport
}

pub fn monitor_sizes(size: egui::Vec2, monitor: egui::Vec2) -> (egui::Vec2, egui::Vec2) {
    // Leave space for decorations/taskbars on screens smaller than the normal UI minimum.
    let maximum = (monitor - egui::vec2(40.0, 80.0)).max(egui::vec2(320.0, 240.0));
    let minimum = egui::vec2(950.0, 760.0).min(maximum);
    (minimum, size.max(minimum).min(maximum))
}
pub fn fit_to_monitor(ctx: &egui::Context) {
    // Context commands need a write lock: release input's read lock first.
    let viewport = ctx.input(|input| input.viewport().clone());
    if let Some(monitor) = viewport
        .monitor_size
        .filter(|s| s.is_finite() && s.x > 0.0 && s.y > 0.0)
    {
        let size = viewport
            .inner_rect
            .map_or(egui::vec2(1150.0, 760.0), |r| r.size());
        let (minimum, fitted) = monitor_sizes(size, monitor);
        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(minimum));
        if viewport.maximized != Some(true)
            && viewport.fullscreen != Some(true)
            && (size - fitted).length() > 1.0
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(fitted));
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_native_frame_can_issue_monitor_commands_without_reentrant_locking() {
        let ctx = egui::Context::default();
        let mut input = egui::RawInput::default();
        let viewport = input.viewports.get_mut(&egui::ViewportId::ROOT).unwrap();
        viewport.monitor_size = Some(egui::vec2(800.0, 600.0));
        viewport.inner_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(800.0, 600.0),
        ));
        let mut output = ctx.run_ui(input, |ui| fit_to_monitor(ui.ctx()));
        let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
        assert!(commands.iter().any(|c| matches!(c,egui::ViewportCommand::MinInnerSize(size) if *size==egui::vec2(760.0,520.0))));
        assert!(commands.iter().any(
            |c| matches!(c,egui::ViewportCommand::InnerSize(size) if *size==egui::vec2(760.0,520.0))
        ));
        output.textures_delta.clear();
    }
    #[derive(Default)]
    struct Memory(std::collections::HashMap<String, String>);
    impl eframe::Storage for Memory {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.into(), value);
        }
        fn remove_string(&mut self, key: &str) {
            self.0.remove(key);
        }
        fn flush(&mut self) {}
    }
    #[test]
    fn settings_roundtrip_validation_and_history_separation() {
        let mut storage = Memory::default();
        let expected = Layout {
            history_open: true,
            history_width: 340.0,
            ..Default::default()
        };
        expected.save(&mut storage);
        assert_eq!(Layout::load(Some(&storage)), expected);
        assert_eq!(storage.0.len(), 1);
        assert!(storage.0.contains_key(KEY));
        for text in [
            "bad",
            r#"{"version":2,"history_open":true,"history_width":320}"#,
            r#"{"version":1,"history_open":true,"history_width":null}"#,
        ] {
            assert!(Layout::decode(text).is_none());
        }
        let small =
            Layout::decode(r#"{"version":1,"history_open":true,"history_width":-100}"#).unwrap();
        assert_eq!(small.history_width, 260.0);
        assert_eq!(Layout::load(None), Layout::default());
    }
    #[test]
    fn invalid_and_small_window_values_are_sanitized() {
        let viewport = sanitize_window(
            egui::ViewportBuilder::default()
                .with_inner_size([f32::NAN, 0.0])
                .with_position([f32::INFINITY, 0.0]),
        );
        assert_eq!(viewport.inner_size, Some(egui::vec2(64.0, 64.0)));
        assert!(viewport.position.is_none());
        assert_eq!(
            sanitize_window(egui::ViewportBuilder::default().with_inner_size([64.0, 64.0]))
                .inner_size,
            Some(egui::vec2(64.0, 64.0))
        );
    }
    #[test]
    fn startup_hook_preserves_framework_monitor_clamp_before_first_frame() {
        for monitor in [
            egui::vec2(800.0, 600.0),
            egui::vec2(640.0, 480.0),
            egui::vec2(1920.0, 1080.0),
        ] {
            for requested in [
                egui::vec2(1150.0, 760.0),
                egui::vec2(3000.0, 2000.0),
                egui::vec2(64.0, 64.0),
            ] {
                // eframe's viewport_builder clamps restored/default inner_size
                // before invoking NativeOptions.window_builder.
                let framework_size = requested.min(monitor);
                let before_creation = sanitize_window(
                    egui::ViewportBuilder::default()
                        .with_inner_size(framework_size)
                        .with_min_inner_size([950.0, 760.0]),
                );
                assert_eq!(before_creation.inner_size, Some(framework_size));
                assert!(before_creation.min_inner_size.unwrap().x <= framework_size.x);
                assert!(before_creation.min_inner_size.unwrap().y <= framework_size.y);
                let (_, after_first_frame) = monitor_sizes(framework_size, monitor);
                assert!(after_first_frame.x <= monitor.x && after_first_frame.y <= monitor.y);
            }
        }
    }
    #[test]
    fn smaller_monitor_reduces_minimum_and_keeps_window_size_usable() {
        let (minimum, size) = monitor_sizes(egui::vec2(1500.0, 1200.0), egui::vec2(800.0, 600.0));
        assert_eq!(minimum, egui::vec2(760.0, 520.0));
        assert_eq!(size, minimum);
        let (minimum, size) = monitor_sizes(egui::vec2(640.0, 480.0), egui::vec2(1920.0, 1080.0));
        assert_eq!(minimum, egui::vec2(950.0, 760.0));
        assert_eq!(size, minimum);
    }
}
