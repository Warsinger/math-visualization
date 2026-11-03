//! Bounded plot workspace. Spatial workspaces contain surfaces, helices and XY curves. Only applied equations are persisted; geometry is rebuilt.
use crate::{
    curve::{self, CurveSampling},
    geometry::{self, Geometry, Sampling},
    scene::Helix,
};
use serde::{Deserialize, Serialize};

pub const MAX_PLOTS: usize = 8;
pub const MAX_SPATIAL_LINES: usize = 300_000;
const COLORS: [[u8; 3]; MAX_PLOTS] = [
    [100, 190, 255],
    [255, 170, 80],
    [120, 220, 145],
    [220, 140, 245],
    [255, 120, 150],
    [100, 225, 220],
    [230, 220, 100],
    [190, 190, 210],
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlotKind {
    #[default]
    Curve,
    Surface,
    Helix {
        parameters: Helix,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plot {
    #[serde(default)]
    pub object: PlotKind,
    pub id: u64,
    pub source: String,
    pub visible: bool,
    pub color: [u8; 3],
    #[serde(skip)]
    pub geometry: Geometry,
    #[serde(skip)]
    pub draft: String,
    #[serde(skip)]
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spatial: Option<Sampling>,
    pub plots: Vec<Plot>,
    pub selected: u64,
    pub sampling: CurveSampling,
}

impl Workspace {
    pub fn single(source: &str, sampling: CurveSampling) -> Result<Self, String> {
        let mut workspace = Self {
            spatial: None,
            plots: Vec::new(),
            selected: 0,
            sampling,
        };
        workspace.add()?;
        workspace.apply(source)?;
        Ok(workspace)
    }
    pub fn spatial_single(
        object: PlotKind,
        source: &str,
        sampling: Sampling,
    ) -> Result<Self, String> {
        let mut workspace = Self {
            spatial: Some(sampling),
            plots: Vec::new(),
            selected: 0,
            sampling: CurveSampling::default(),
        };
        workspace.add()?;
        workspace.selected_mut().object = object;
        let request = if matches!(object, PlotKind::Helix { .. }) {
            workspace.selected_request()
        } else {
            source.into()
        };
        workspace.apply(&request)?;
        Ok(workspace)
    }
    fn build(
        object: PlotKind,
        source: &str,
        sampling: CurveSampling,
        spatial: Option<Sampling>,
    ) -> Result<Geometry, String> {
        if let PlotKind::Helix { parameters } = object {
            let h = parameters.validate()?;
            let points = h.points();
            return Ok(Geometry {
                lines: points.windows(2).map(|p| [p[0], p[1]]).collect(),
                bound: h.radius.hypot(h.height / 2.0).max(1.0),
                skipped_tetrahedra: 0,
            });
        }
        if source.is_empty() {
            return Ok(Geometry::default());
        }
        match object {
            PlotKind::Surface => geometry::mesh(
                &crate::expression::Equation::parse(source)?,
                spatial.ok_or("Surface requires a 3D workspace.")?,
            ),
            PlotKind::Curve => {
                let settings = spatial.map_or(sampling, |s| CurveSampling {
                    extent: [s.extent[0], s.extent[1]],
                    samples: sampling.samples,
                });
                curve::sample(source, settings)
            }
            PlotKind::Helix { .. } => unreachable!(),
        }
    }
    pub fn rebuild_spatial(
        &mut self,
        sampling: Sampling,
        curve_samples: usize,
    ) -> Result<(), String> {
        let mut candidate = self.clone();
        candidate.spatial = Some(sampling);
        let curves = CurveSampling {
            samples: curve_samples,
            ..candidate.sampling
        };
        candidate.rebuild(curves)?;
        *self = candidate;
        Ok(())
    }
    pub fn change_object(&mut self, object: PlotKind, source: &str) -> Result<(), String> {
        let mut candidate = self.clone();
        candidate.selected_mut().object = object;
        candidate.apply(source)?;
        *self = candidate;
        Ok(())
    }
    pub fn selected_request(&self) -> String {
        let plot = self.selected();
        match plot.object {
            PlotKind::Helix { parameters: h } => format!(
                "create a helix with radius {} and height {} and turns {}",
                h.radius, h.height, h.turns
            ),
            _ => plot.source.clone(),
        }
    }
    pub fn selected(&self) -> &Plot {
        self.plots
            .iter()
            .find(|p| p.id == self.selected)
            .expect("validated selection")
    }
    pub fn selected_mut(&mut self) -> &mut Plot {
        self.plots
            .iter_mut()
            .find(|p| p.id == self.selected)
            .expect("validated selection")
    }
    pub fn validate(&self) -> Result<(), String> {
        self.sampling.validate()?;
        if let Some(sampling) = self.spatial {
            sampling.validate()?;
        }
        if self.plots.is_empty() || self.plots.len() > MAX_PLOTS {
            return Err("A workspace must contain 1–8 equation rows.".into());
        }
        let mut ids = std::collections::HashSet::new();
        for plot in &self.plots {
            if plot.id == 0 || !ids.insert(plot.id) || plot.source.len() > 1024 {
                return Err("Invalid workspace equation identity or length.".into());
            }
            if self.spatial.is_none() && plot.object != PlotKind::Curve {
                return Err("A 2D workspace only accepts planar equations.".into());
            }
            match plot.object {
                PlotKind::Helix { parameters } => {
                    parameters.validate()?;
                }
                PlotKind::Surface if !plot.source.is_empty() => {
                    crate::expression::Equation::parse(&plot.source)?;
                }
                PlotKind::Curve if !plot.source.is_empty() => {
                    crate::expression::Equation::planar(&plot.source)?;
                }
                _ => {}
            }
        }
        if !ids.contains(&self.selected) {
            return Err("Invalid workspace selection.".into());
        }
        Ok(())
    }
    pub fn rebuild(&mut self, sampling: CurveSampling) -> Result<(), String> {
        self.validate()?;
        sampling.validate()?;
        // Transactional: do not leave half the rows at the new sampling settings.
        let geometry = self
            .plots
            .iter()
            .map(|p| Self::build(p.object, &p.source, sampling, self.spatial))
            .collect::<Result<Vec<_>, String>>()?;
        if self.spatial.is_some()
            && geometry.iter().map(|g| g.lines.len()).sum::<usize>() > MAX_SPATIAL_LINES
        {
            return Err(
                "3D workspace exceeds 300,000 lines; reduce sampling or remove objects.".into(),
            );
        }
        for (plot, geometry) in self.plots.iter_mut().zip(geometry) {
            plot.geometry = geometry;
        }
        self.sampling = sampling;
        Ok(())
    }
    pub fn restore(&mut self) -> Result<(), String> {
        self.rebuild(self.sampling)?;
        for plot in &mut self.plots {
            plot.draft = plot.source.clone();
            plot.error = None;
        }
        Ok(())
    }
    pub fn apply(&mut self, source: &str) -> Result<(), String> {
        let source = source.trim();
        let result = if source.is_empty() || source.len() > 1024 {
            Err("Enter an equation of at most 1024 bytes.".into())
        } else {
            Self::build(self.selected().object, source, self.sampling, self.spatial).and_then(|g| {
                if self.spatial.is_some()
                    && self
                        .plots
                        .iter()
                        .filter(|p| p.id != self.selected)
                        .map(|p| p.geometry.lines.len())
                        .sum::<usize>()
                        + g.lines.len()
                        > MAX_SPATIAL_LINES
                {
                    Err(
                        "3D workspace exceeds 300,000 lines; reduce sampling or remove objects."
                            .into(),
                    )
                } else {
                    Ok(g)
                }
            })
        };
        let plot = self.selected_mut();
        plot.draft = source.into();
        match result {
            Ok(geometry) => {
                plot.source = source.into();
                plot.geometry = geometry;
                plot.error = None;
                Ok(())
            }
            Err(error) => {
                plot.error = Some(error.clone());
                Err(error)
            }
        }
    }
    pub fn add(&mut self) -> Result<(), String> {
        if self.plots.len() >= MAX_PLOTS {
            return Err("Up to eight equations per workspace.".into());
        }
        let id = self
            .plots
            .iter()
            .map(|p| p.id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("Equation ID limit reached.")?;
        let color = COLORS
            .iter()
            .find(|c| !self.plots.iter().any(|p| &p.color == *c))
            .copied()
            .unwrap_or(COLORS[0]);
        self.plots.push(Plot {
            object: if self.spatial.is_some() {
                PlotKind::Surface
            } else {
                PlotKind::Curve
            },
            id,
            source: String::new(),
            visible: true,
            color,
            geometry: Geometry::default(),
            draft: String::new(),
            error: None,
        });
        self.selected = id;
        Ok(())
    }
    pub fn remove(&mut self, id: u64) {
        self.plots.retain(|p| p.id != id);
        if self.plots.is_empty() {
            self.add().expect("empty workspace can add");
        }
        if !self.plots.iter().any(|p| p.id == self.selected) {
            self.selected = self.plots[0].id;
        }
    }
}

impl PartialEq for Plot {
    fn eq(&self, other: &Self) -> bool {
        self.object == other.object
            && self.id == other.id
            && self.source == other.source
            && self.visible == other.visible
            && self.color == other.color
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spatial_objects_share_bounds_and_rebuild_without_losing_drafts() {
        let settings = Sampling {
            extent: [2.0, 3.0, 4.0],
            resolution: 12,
        };
        let mut w =
            Workspace::spatial_single(PlotKind::Surface, "x^2+y^2+z^2=1", settings).unwrap();
        w.add().unwrap();
        w.selected_mut().object = PlotKind::Curve;
        w.apply("x^2+y^2=1").unwrap();
        assert!(
            w.selected()
                .geometry
                .lines
                .iter()
                .flatten()
                .all(|p| p[2] == 0.0)
        );
        let helix = Helix {
            radius: 1.5,
            height: 5.0,
            turns: 2.0,
        };
        w.add().unwrap();
        w.selected_mut().object = PlotKind::Helix { parameters: helix };
        w.apply("helix").unwrap();
        let helix_lines = w.selected().geometry.lines.clone();
        w.plots[0].visible = false;
        w.plots[0].draft = "broken(".into();
        w.rebuild_spatial(
            Sampling {
                extent: [4.0; 3],
                resolution: 16,
            },
            256,
        )
        .unwrap();
        assert_eq!(w.plots[0].draft, "broken(");
        assert_eq!(w.selected().geometry.lines, helix_lines);
        assert!(!w.plots[0].geometry.lines.is_empty());
        assert_eq!(
            w.plots[1].geometry.lines,
            curve::sample(
                "x^2+y^2=1",
                CurveSampling {
                    extent: [4.0; 2],
                    samples: 256
                }
            )
            .unwrap()
            .lines
        );
        let applied = w.clone();
        assert!(
            w.rebuild_spatial(
                Sampling {
                    resolution: 100,
                    ..settings
                },
                256
            )
            .is_err()
        );
        assert_eq!(w, applied);
        assert_eq!(w.plots[0].geometry.lines, applied.plots[0].geometry.lines);
    }
    #[test]
    fn aggregate_budget_counts_hidden_objects_and_failed_edits_keep_geometry() {
        let mut w = Workspace::spatial_single(
            PlotKind::Surface,
            "z=0",
            Sampling {
                resolution: 8,
                ..Default::default()
            },
        )
        .unwrap();
        let first_lines = w.selected().geometry.lines.clone();
        assert!(w.change_object(PlotKind::Curve, "z=1").is_err());
        assert_eq!(w.selected().object, PlotKind::Surface);
        assert_eq!(w.selected().geometry.lines, first_lines);
        w.selected_mut().visible = false;
        // Synthetic cached geometry isolates aggregate accounting from mesher limits.
        w.selected_mut().geometry.lines = vec![[[0.0; 3]; 2]; MAX_SPATIAL_LINES];
        w.add().unwrap();
        assert!(w.apply("x=0").unwrap_err().contains("300,000"));
        assert!(w.selected().source.is_empty());
        assert!(w.selected().geometry.lines.is_empty());
    }
    #[test]
    fn spatial_json_roundtrips_types_and_rejects_invalid_helix_and_dimension() {
        let mut w = Workspace::spatial_single(
            PlotKind::Helix {
                parameters: Helix::default(),
            },
            "helix",
            Sampling::default(),
        )
        .unwrap();
        w.add().unwrap();
        w.selected_mut().object = PlotKind::Curve;
        w.apply("sin(x)").unwrap();
        let json = serde_json::to_string(&w).unwrap();
        let mut restored: Workspace = serde_json::from_str(&json).unwrap();
        restored.restore().unwrap();
        assert_eq!(w, restored);
        assert!(restored.plots.iter().all(|p| !p.geometry.lines.is_empty()));
        restored.plots[0].object = PlotKind::Helix {
            parameters: Helix {
                radius: -1.0,
                ..Default::default()
            },
        };
        assert!(restored.validate().is_err());
        restored = w;
        restored.spatial = None;
        assert!(restored.validate().is_err());
        let legacy = r#"{"plots":[{"id":1,"source":"x^2","visible":true,"color":[100,190,255]}],"selected":1,"sampling":{"extent":[5,5],"samples":1024}}"#;
        let mut old: Workspace = serde_json::from_str(legacy).unwrap();
        old.restore().unwrap();
        assert_eq!(old.selected().object, PlotKind::Curve);
        assert!(old.spatial.is_none());
    }
    #[test]
    fn editing_and_sampling_keep_other_equations_independent() {
        let mut w = Workspace::single("y=x^2", CurveSampling::default()).unwrap();
        let first = w.selected().id;
        let lines = w.selected().geometry.lines.clone();
        w.add().unwrap();
        w.apply("x^2+y^2=1").unwrap();
        assert_ne!(w.plots[0].color, w.plots[1].color);
        let circle = w.selected().geometry.lines.clone();
        assert!(w.apply("bad(").is_err());
        assert_eq!(w.selected().source, "x^2+y^2=1");
        assert_eq!(w.selected().geometry.lines, circle);
        assert!(w.selected().error.is_some());
        assert_eq!(w.plots[0].geometry.lines, lines);
        let invalid = CurveSampling {
            samples: 0,
            ..w.sampling
        };
        assert!(w.rebuild(invalid).is_err());
        assert_eq!(w.selected().geometry.lines, circle);
        let settings = CurveSampling {
            extent: [2.0, 3.0],
            samples: 256,
        };
        w.rebuild(settings).unwrap();
        for plot in &w.plots {
            assert_eq!(
                plot.geometry.lines,
                curve::sample(&plot.source, settings).unwrap().lines
            );
        }
        assert_eq!(w.plots[0].id, first);
    }
    #[test]
    fn row_limits_removal_and_selection_are_valid() {
        let mut w = Workspace::single("x", CurveSampling::default()).unwrap();
        for _ in 1..MAX_PLOTS {
            w.add().unwrap();
        }
        assert!(w.add().is_err());
        let selected = w.selected;
        w.remove(selected);
        assert_eq!(w.selected, w.plots[0].id);
        while w.plots.len() > 1 {
            w.remove(w.selected);
        }
        w.remove(w.selected);
        w.validate().unwrap();
        assert!(w.selected().source.is_empty());
        assert!(w.apply("").is_err());
        assert!(w.selected().error.is_some());
    }
    #[test]
    fn serialized_workspace_contains_applied_state_only_and_validates_ids() {
        let mut w = Workspace::single("sin(x)", CurveSampling::default()).unwrap();
        w.add().unwrap();
        w.apply("x^2+y^2=1").unwrap();
        w.selected_mut().visible = false;
        w.selected_mut().draft = "unapplied_draft".into();
        w.selected_mut().error = Some("transient_error".into());
        let text = serde_json::to_string(&w).unwrap();
        for transient in ["geometry", "unapplied_draft", "transient_error"] {
            assert!(!text.contains(transient));
        }
        let mut restored: Workspace = serde_json::from_str(&text).unwrap();
        restored.restore().unwrap();
        assert_eq!(restored, w);
        assert_eq!(restored.selected().draft, restored.selected().source);
        assert!(!restored.plots[0].geometry.lines.is_empty());
        restored.plots[1].id = restored.plots[0].id;
        assert!(restored.validate().is_err());
        restored = w.clone();
        restored.selected = 999;
        assert!(restored.validate().is_err());
        restored = w;
        restored.plots.clear();
        assert!(restored.validate().is_err());
    }
}
