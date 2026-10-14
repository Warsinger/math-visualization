use crate::{
    command,
    expression::Equation,
    geometry::{self, Geometry, Sampling},
};
use std::f32::consts::TAU;

/// One selected helix. Height is total height; pitch is height / turns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Helix {
    pub radius: f32,
    pub height: f32,
    pub turns: f32,
}

impl Default for Helix {
    fn default() -> Self {
        Self {
            radius: 1.0,
            height: 4.0,
            turns: 5.0,
        }
    }
}

impl Helix {
    pub fn validate(self) -> Result<Self, String> {
        for (name, value, low, high) in [
            ("radius", self.radius, 0.1, 5.0),
            ("height", self.height, 0.1, 20.0),
            ("turns", self.turns, 0.25, 20.0),
        ] {
            if !value.is_finite() || !(low..=high).contains(&value) {
                return Err(format!("{name} must be between {low} and {high}."));
            }
        }
        Ok(self)
    }

    pub fn pitch(self) -> f32 {
        self.height / self.turns
    }

    /// Uniform samples of (r cos(2πnt), r sin(2πnt), h(t - 1/2)).
    pub fn points(self) -> Vec<[f32; 3]> {
        let segments = (self.turns * 96.0).ceil() as usize;
        (0..=segments)
            .map(|i| {
                let t = i as f32 / segments as f32;
                let angle = TAU * self.turns * t;
                [
                    self.radius * angle.cos(),
                    self.radius * angle.sin(),
                    self.height * (t - 0.5),
                ]
            })
            .collect()
    }
}

/// Both UI controls and commands replace this same validated scene state.
#[derive(Clone)]
pub struct Scene {
    pub helix: Helix,
    pub surface: Option<Surface>,
    pub geometry: Geometry,
}

#[derive(Clone)]
pub struct Surface {
    pub source: String,
    pub sampling: Sampling,
}

impl Default for Scene {
    fn default() -> Self {
        let mut scene = Self {
            helix: Helix::default(),
            surface: None,
            geometry: Geometry::default(),
        };
        scene.replace(scene.helix).expect("default helix is valid");
        scene
    }
}

impl Scene {
    pub fn replace(&mut self, helix: Helix) -> Result<(), String> {
        let helix = helix.validate()?;
        let points = helix.points();
        self.geometry = Geometry {
            lines: points.windows(2).map(|p| [p[0], p[1]]).collect(),
            bound: helix.radius.hypot(helix.height / 2.0).max(1.0),
            skipped_tetrahedra: 0,
        };
        self.helix = helix;
        self.surface = None;
        Ok(())
    }

    pub fn plot(&mut self, source: &str, sampling: Sampling) -> Result<(), String> {
        let equation = Equation::parse(source)?;
        let geometry = geometry::mesh(&equation, sampling)?;
        self.geometry = geometry;
        self.surface = Some(Surface {
            source: source.into(),
            sampling,
        });
        Ok(())
    }

    pub fn request(&mut self, text: &str) -> Result<(), String> {
        let text = text.trim();
        if let Some(source) = text.strip_prefix("plot ") {
            let sampling = self
                .surface
                .as_ref()
                .map_or(Sampling::default(), |s| s.sampling);
            self.plot(source.trim(), sampling)
        } else {
            if self.surface.is_some() && !text.to_lowercase().starts_with("create a helix with ") {
                return Err("For this surface use 'plot ...' or its sampling controls. Create a helix to use helix commands.".into());
            }
            self.replace(command::parse(text, self.helix)?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_has_correct_radius_height_and_endpoints() {
        let h = Helix::default();
        let points = h.points();
        assert_eq!(points.len(), 481);
        for p in &points {
            assert!((p[0].hypot(p[1]) - h.radius).abs() < 1e-5);
        }
        assert_eq!(points[0], [1.0, 0.0, -2.0]);
        let last = points.last().unwrap();
        assert!((last[0] - 1.0).abs() < 1e-5);
        assert!(last[1].abs() < 1e-5);
        assert_eq!(last[2], 2.0);
    }

    #[test]
    fn failed_edit_preserves_scene() {
        let mut scene = Scene::default();
        let before = scene.helix;
        assert!(
            scene
                .replace(Helix {
                    radius: f32::NAN,
                    ..before
                })
                .is_err()
        );
        assert_eq!(scene.helix, before);
    }
}
