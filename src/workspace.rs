//! Bounded 2D workspace. Only applied equations are persisted; geometry is rebuilt.
use crate::{
    curve::{self, CurveSampling},
    geometry::Geometry,
};
use serde::{Deserialize, Serialize};

pub const MAX_PLOTS: usize = 8;
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plot {
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
    pub plots: Vec<Plot>,
    pub selected: u64,
    pub sampling: CurveSampling,
}

impl Workspace {
    pub fn single(source: &str, sampling: CurveSampling) -> Result<Self, String> {
        let mut workspace = Self {
            plots: Vec::new(),
            selected: 0,
            sampling,
        };
        workspace.add()?;
        workspace.apply(source)?;
        Ok(workspace)
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
        if self.plots.is_empty() || self.plots.len() > MAX_PLOTS {
            return Err("A workspace must contain 1–8 equation rows.".into());
        }
        let mut ids = std::collections::HashSet::new();
        for plot in &self.plots {
            if plot.id == 0 || !ids.insert(plot.id) || plot.source.len() > 1024 {
                return Err("Invalid workspace equation identity or length.".into());
            }
            if !plot.source.is_empty() {
                crate::expression::Equation::planar(&plot.source)?;
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
            .map(|p| {
                if p.source.is_empty() {
                    Ok(Geometry::default())
                } else {
                    curve::sample(&p.source, sampling)
                }
            })
            .collect::<Result<Vec<_>, String>>()?;
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
            curve::sample(source, self.sampling)
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
        self.id == other.id
            && self.source == other.source
            && self.visible == other.visible
            && self.color == other.color
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
