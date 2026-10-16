//! Bounded numerical slope estimates for explicit Cartesian functions.
use crate::expression::Equation;

#[derive(Clone, Copy, Debug)]
pub struct Tangent {
    pub x: f64,
    pub y: f64,
    pub slope: f64,
}

/// Compare one-sided slopes at successively smaller steps. Agreement is a
/// numerical heuristic, never a symbolic differentiability/continuity proof.
pub fn tangent(source: &str, x: f64) -> Result<Tangent, String> {
    let equation = Equation::explicit_y(source)?;
    let value = |x| equation.evaluate([x, 0.0, 0.0]);
    let y = value(x).ok_or("Function is undefined or nonfinite at this x.")?;
    let mut h = 1e-3 * x.abs().max(1.0);
    let mut previous_sides: Option<(f64, f64, f64)> = None;
    let mut previous_estimate: Option<(f64, f64, f64)> = None;
    let mut contractions = 0;
    let mut plateau_steps = 0;
    let mut persistent_disagreement = false;
    let mut best_estimate: Option<(f64, f64)> = None;
    for _ in 0..12 {
        if let (Some(a), Some(b)) = (value(x - h), value(x + h)) {
            let left = (y - a) / h;
            let right = (b - y) / h;
            // A scale-based allowance for cancellation in function differences.
            // This is a noise heuristic, not a bound on arbitrary expression error.
            let function_scale = a.abs().max(b.abs()).max(y.abs());
            // Representing x±h perturbs the input, even when f(x) is zero.
            let input_effect = x.abs().max(h) * left.abs().max(right.abs());
            let noise = (8.0 * f64::EPSILON) * (function_scale / h + input_effect / h);
            if let Some((old_left, old_right, old_noise)) = previous_sides {
                // Extrapolate each side separately to remove leading O(h)
                // curvature error without cancelling a corner's slope jump.
                let extrapolated_left = 2.0 * left - old_left;
                let extrapolated_right = 2.0 * right - old_right;
                let slope = extrapolated_left * 0.5 + extrapolated_right * 0.5;
                let gap = (extrapolated_left - extrapolated_right).abs();
                let allowance = 2.0 * noise + old_noise;
                if slope.is_finite() && gap.is_finite() && allowance.is_finite() {
                    if let Some((old_slope, old_gap, old_allowance)) = previous_estimate {
                        let resolved = gap > allowance && old_gap > old_allowance;
                        if resolved && (gap - old_gap).abs() <= 0.1 * gap.max(old_gap) {
                            plateau_steps += 1;
                            persistent_disagreement |= plateau_steps >= 2;
                        } else {
                            plateau_steps = 0;
                        }
                        if gap <= 0.75 * old_gap + allowance {
                            contractions += 1;
                        } else {
                            contractions = 0;
                        }
                        let tolerance = 1e-4 * (1.0 + slope.abs());
                        let within_noise = gap <= allowance && old_gap <= old_allowance;
                        if gap <= tolerance
                            && (slope - old_slope).abs() <= tolerance
                            && (within_noise || contractions >= 2)
                        {
                            let uncertainty = gap + allowance + (slope - old_slope).abs();
                            if best_estimate.is_none_or(|(_, best)| uncertainty < best) {
                                best_estimate = Some((slope, uncertainty));
                            }
                        }
                    }
                    previous_estimate = Some((slope, gap, allowance));
                } else {
                    previous_estimate = None;
                    contractions = 0;
                    plateau_steps = 0;
                }
            }
            previous_sides = Some((left, right, noise));
        } else {
            previous_sides = None;
            previous_estimate = None;
            contractions = 0;
            plateau_steps = 0;
        }
        h *= 0.5;
    }
    // Inspect the entire bounded refinement sequence: a small early gap is not
    // evidence of convergence. Later roundoff must neither erase a resolved
    // plateau nor discard a useful earlier estimate of a smooth derivative.
    if !persistent_disagreement && let Some((slope, _)) = best_estimate {
        return Ok(Tangent { x, y, slope });
    }
    Err("No reliable finite slope: domain boundary, corner, discontinuity, or numerical instability.".into())
}

impl Tangent {
    /// Clip the infinite tangent to the applied sampling rectangle before drawing.
    pub fn segment(self, extent: [f32; 2]) -> Option<[[f32; 3]; 2]> {
        let [ex, ey] = extent.map(f64::from);
        let (mut low, mut high) = (-ex, ex);
        if self.slope.abs() > f64::EPSILON {
            let a = self.x + (-ey - self.y) / self.slope;
            let b = self.x + (ey - self.y) / self.slope;
            low = low.max(a.min(b));
            high = high.min(a.max(b));
        } else if self.y.abs() > ey {
            return None;
        }
        if low > high {
            return None;
        }
        let line =
            [low, high].map(|x| [x as f32, (self.y + self.slope * (x - self.x)) as f32, 0.0]);
        line.iter().flatten().all(|v| v.is_finite()).then_some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn smooth_zero_crossings_survive_late_roundoff() {
        for (source, x, expected) in [
            ("ln(x)", 1.0, 1.0_f64),
            ("ln(x)", 0.9999, 1.0 / 0.9999),
            ("ln(x)", 1.0001, 1.0 / 1.0001),
            ("ln(x+1)", 0.0, 1.0),
            ("sin(x)", std::f64::consts::PI, -1.0),
            ("sin(x-1)", 1.0, 1.0),
            ("x^3", 0.0, 0.0),
            ("0.000000000001*ln(x)", 1.0, 1e-12),
        ] {
            let result = tangent(source, x).unwrap_or_else(|e| panic!("{source} at {x}: {e}"));
            let tolerance = if expected == 0.0 {
                1e-8
            } else {
                1e-6 * expected.abs()
            };
            assert!(
                (result.slope - expected).abs() < tolerance,
                "{source} at {x}: {result:?}"
            );
        }
    }
    #[test]
    fn scaled_corners_retain_persistent_one_sided_disagreement() {
        for scale in [1e-12, 1e-8, 1e-5, 1.0, 1e6] {
            for source in [
                format!("{scale}*(x^2)^0.5"),
                format!("-{scale}*(x^2)^0.5"),
                format!("x^2+{scale}*(x^2)^0.5"),
            ] {
                assert!(tangent(&source, 0.0).is_err(), "accepted corner: {source}");
            }
        }
        assert!(tangent("1+0.00001*((x-0.25)^2)^0.5", 0.25).is_err());
    }

    #[test]
    fn small_smooth_slopes_and_flat_points_remain_available() {
        for scale in [1e-12, 1e-8, 1e-5, 1.0] {
            for (source, x, expected) in [
                (format!("{scale}*x"), 0.0, scale),
                (format!("{scale}*x^2"), 0.0, 0.0),
                (format!("{scale}*x^2"), 2.0, 4.0 * scale),
            ] {
                let t = tangent(&source, x).unwrap_or_else(|e| panic!("{source}: {e}"));
                assert!(
                    (t.slope - expected).abs() <= scale * 1e-6,
                    "{source}: {t:?}"
                );
            }
        }
        let t = tangent("1+0.00001*x", 2.0).unwrap();
        assert!((t.slope - 1e-5).abs() < 1e-8);
    }
    #[test]
    fn known_slopes_update_point_and_clip_line() {
        for (source, x, expected) in [
            ("x^2", 2.0, 4.0),
            ("sin(x)", 0.0, 1.0),
            ("ln(x)", 2.0, 0.5),
            ("4", 1.0, 0.0),
            ("y=x^3", -2.0, 12.0),
        ] {
            let t = tangent(source, x).unwrap();
            assert!((t.slope - expected).abs() < 1e-4, "{source}: {t:?}");
            let line = t.segment([5.0, 5.0]).unwrap();
            assert!(
                line.iter()
                    .all(|p| p[0].abs() <= 5.0001 && p[1].abs() <= 5.0001)
            );
            for p in line {
                assert!((f64::from(p[1]) - t.y - t.slope * (f64::from(p[0]) - t.x)).abs() < 1e-4);
            }
        }
        assert_ne!(
            tangent("x^2", 1.0).unwrap().slope,
            tangent("x^2", 2.0).unwrap().slope
        );
        assert!(tangent("10", 0.0).unwrap().segment([5.0, 5.0]).is_none());
    }
    #[test]
    fn rejects_domains_corners_vertical_slopes_and_implicit_curves() {
        for (source, x) in [
            ("ln(x)", 0.0),
            ("1/x", 0.0),
            ("(x^2)^0.5", 0.0),
            ("x^0.5", 0.0),
            ("x^2+y^2=1", 0.0),
            ("x^2", f64::NAN),
            ("x^2", f64::INFINITY),
        ] {
            assert!(tangent(source, x).is_err(), "{source} at {x}");
        }
    }
}
