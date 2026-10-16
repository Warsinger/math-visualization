//! Explicit y=f(x) sampling in the XY plane, with bounded work and clipping.
use crate::{
    expression::Equation,
    geometry::{Geometry, Point},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveSampling {
    pub extent: [f32; 2],
    pub samples: usize,
}
impl Default for CurveSampling {
    fn default() -> Self {
        Self {
            extent: [5.0, 5.0],
            samples: 1024,
        }
    }
}
impl CurveSampling {
    pub fn validate(self) -> Result<Self, String> {
        if self
            .extent
            .iter()
            .any(|v| !v.is_finite() || !(0.1..=20.0).contains(v))
        {
            return Err("2D X/Y half-extents must be 0.1–20.".into());
        }
        if !(64..=4096).contains(&self.samples) {
            return Err("2D samples must be 64–4096.".into());
        }
        Ok(self)
    }
}

pub fn sample(source: &str, settings: CurveSampling) -> Result<Geometry, String> {
    let s = settings.validate()?;
    let equation = Equation::explicit_y(source)?;
    let mut geometry = Geometry {
        bound: s.extent[0].hypot(s.extent[1]),
        ..Default::default()
    };
    let value = |x: f32| {
        equation
            .evaluate([f64::from(x), 0.0, 0.0])
            .filter(|v| v.abs() <= f64::from(f32::MAX))
            .map(|y| [x, y as f32, 0.0])
    };
    let mut previous = value(-s.extent[0]);
    for i in 1..=s.samples {
        let x = -s.extent[0] + 2.0 * s.extent[0] * (i as f32 / s.samples as f32);
        let current = value(x);
        if let (Some(a), Some(b)) = (previous, current) {
            // A midpoint check avoids many false bridges, but is not a proof of continuity.
            if let Some(mid) = value((a[0] + b[0]) * 0.5) {
                let linear = f64::from(a[1]) * 0.5 + f64::from(b[1]) * 0.5;
                if (f64::from(mid[1]) - linear).abs() <= f64::from(s.extent[1]) * 0.05 {
                    if let Some(line) = clip_y(a, b, s.extent[1]) {
                        geometry.lines.push(line);
                    }
                } else {
                    geometry.skipped_tetrahedra += 1;
                }
            } else {
                geometry.skipped_tetrahedra += 1;
            }
        } else {
            geometry.skipped_tetrahedra += 1;
        }
        previous = current;
    }
    Ok(geometry)
}

fn clip_y(a: Point, b: Point, extent: f32) -> Option<[Point; 2]> {
    let (ay, by) = (f64::from(a[1]), f64::from(b[1]));
    let limit = f64::from(extent);
    if (ay > limit && by > limit) || (ay < -limit && by < -limit) {
        return None;
    }
    let at = |y: f64| {
        let t = (y - ay) / (by - ay);
        [
            ((1.0 - t) * f64::from(a[0]) + t * f64::from(b[0])) as f32,
            y as f32,
            0.0,
        ]
    };
    let start = if ay.abs() > limit {
        at(ay.clamp(-limit, limit))
    } else {
        a
    };
    let end = if by.abs() > limit {
        at(by.clamp(-limit, limit))
    } else {
        b
    };
    Some([start, end])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shorthand_explicit_and_reversed_parabola_are_planar_and_equal() {
        let s = CurveSampling::default();
        let bare = sample("x^2", s).unwrap();
        assert!(!bare.lines.is_empty());
        assert_eq!(bare.lines, sample("y=x^2", s).unwrap().lines);
        assert_eq!(bare.lines, sample("x^2=y", s).unwrap().lines);
        for p in bare.lines.iter().flatten() {
            assert_eq!(p[2], 0.0);
            assert!(p[1].abs() <= s.extent[1]);
            assert!((p[1] - p[0] * p[0]).abs() < 0.001);
        }
    }
    #[test]
    fn domains_clipping_and_limits() {
        let s = CurveSampling::default();
        assert!(
            sample("ln(x)", s)
                .unwrap()
                .lines
                .iter()
                .flatten()
                .all(|p| p[0] > 0.0)
        );
        assert!(
            sample("1/x", s)
                .unwrap()
                .lines
                .iter()
                .all(|p| p[0][0] * p[1][0] > 0.0)
        );
        assert!(
            sample("y=4", s)
                .unwrap()
                .lines
                .iter()
                .flatten()
                .all(|p| p[1] == 4.0)
        );
        assert!(sample("y=100", s).unwrap().lines.is_empty());
        for source in ["x^2+y^2=1", "y=z", "y=y+x", "x=0", "y=x=0"] {
            assert!(sample(source, s).is_err(), "{source}");
        }
        assert!(sample("x", CurveSampling { samples: 4097, ..s }).is_err());
        assert!(
            sample(
                "x",
                CurveSampling {
                    extent: [f32::NAN, 1.0],
                    ..s
                }
            )
            .is_err()
        );
    }
}
