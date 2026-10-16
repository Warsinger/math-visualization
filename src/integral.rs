//! Bounded composite Simpson estimates; no symbolic or continuity claims.
use crate::expression::Equation;

pub struct Area {
    pub signed: f64,
    pub total: f64,
    pub points: Vec<[f64; 2]>,
}

pub fn integrate(source: &str, a: f64, b: f64) -> Result<Area, String> {
    if !a.is_finite() || !b.is_finite() || a.abs().max(b.abs()) > 1e6 {
        return Err("Integration bounds must be finite and within ±1,000,000.".into());
    }
    let equation = Equation::explicit_y(source)?;
    let value = |x| {
        equation
            .evaluate([x, 0.0, 0.0])
            .ok_or("Function is undefined/nonfinite within the sampled interval.")
    };
    if a == b {
        return Ok(Area {
            signed: 0.0,
            total: 0.0,
            points: vec![[a, value(a)?]],
        });
    }
    let (low, high) = (a.min(b), a.max(b));
    let mut previous: Option<(f64, f64)> = None;
    // At most 16,352 evaluations including every independent-grid cross-check.
    for n in [32, 64, 128, 256, 512, 1024, 2048, 4096] {
        let mut area = estimate(&equation, low, high, n)?;
        let (signed, total) = (area.signed, area.total);
        if previous.is_some_and(|(s, t)| {
            (s - signed).abs() <= 1e-6 * (1.0 + signed.abs())
                && (t - total).abs() <= 1e-6 * (1.0 + total.abs())
        }) {
            // n+2 shares no interior grid phase with a power-of-two grid except
            // its midpoint. Check both signed and absolute area: cancellation
            // alone must not hide unresolved oscillation.
            let check = estimate(&equation, low, high, n + 2)?;
            if (check.signed - signed).abs() <= 1e-6 * (1.0 + signed.abs())
                && (check.total - total).abs() <= 1e-6 * (1.0 + total.abs())
            {
                if b < a {
                    area.signed = -area.signed;
                }
                return Ok(area);
            }
        }
        previous = Some((signed, total));
    }
    Err("Integral did not converge within the sampling budget; possible discontinuity or unresolved detail. Narrow the interval.".into())
}

fn estimate(equation: &Equation, low: f64, high: f64, n: usize) -> Result<Area, String> {
    let h = (high - low) / n as f64;
    let mut area = Area {
        signed: 0.0,
        total: 0.0,
        points: Vec::with_capacity(n + 1),
    };
    for i in 0..=n {
        let x = if i == n { high } else { low + i as f64 * h };
        let y = equation
            .evaluate([x, 0.0, 0.0])
            .ok_or("Function is undefined/nonfinite within the sampled interval.")?;
        let weight = if i == 0 || i == n {
            1.0
        } else if i % 2 == 0 {
            2.0
        } else {
            4.0
        };
        area.signed += weight * y;
        area.total += weight * y.abs();
        area.points.push([x, y]);
    }
    area.signed *= h / 3.0;
    area.total *= h / 3.0;
    if !area.signed.is_finite() || !area.total.is_finite() {
        return Err("Integral estimate overflowed; choose a smaller interval.".into());
    }
    Ok(area)
}

#[derive(Default)]
pub struct Cache {
    key: Option<(String, u64, u64)>,
    value: Option<Result<Area, String>>,
}
impl Cache {
    pub fn get(&mut self, source: &str, a: f64, b: f64) -> &Result<Area, String> {
        let key = (source.to_owned(), a.to_bits(), b.to_bits());
        if self.key.as_ref() != Some(&key) {
            self.value = Some(integrate(source, a, b));
            self.key = Some(key);
        }
        self.value.as_ref().expect("cache initialized")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oscillation_aliases_are_resolved_or_reported_unresolved() {
        for source in ["cos(402.1238596594935*x)", "sin(402.1238596594935*x)"] {
            if let Ok(area) = integrate(source, 0.0, 1.0) {
                assert!(area.signed.abs() < 1e-5, "{source}: signed {}", area.signed);
                assert!(
                    (area.total - 2.0 / std::f64::consts::PI).abs() < 1e-4,
                    "{source}: total {}",
                    area.total
                );
            }
        }
    }
    #[test]
    fn known_integrals_distinguish_signed_total_orientation_and_bounds() {
        for (s, a, b, signed, total) in [
            ("x^2", 0.0, 1.0, 1.0 / 3.0, 1.0 / 3.0),
            ("x", -1.0, 1.0, 0.0, 1.0),
            ("-2", 0.0, 3.0, -6.0, 6.0),
            ("sin(x)", 0.0, std::f64::consts::PI, 2.0, 2.0),
            ("x", 1.0, 0.0, -0.5, 0.5),
        ] {
            let r = integrate(s, a, b).unwrap();
            assert!((r.signed - signed).abs() < 1e-5, "{s}");
            assert!((r.total - total).abs() < 1e-5, "{s}");
            assert_eq!(r.points.first().unwrap()[0], a.min(b));
            assert_eq!(r.points.last().unwrap()[0], a.max(b));
        }
        assert_eq!(integrate("x", 1.0, 1.0).unwrap().signed, 0.0);
        let mut cache = Cache::default();
        assert!((cache.get("x", 0.0, 1.0).as_ref().unwrap().signed - 0.5).abs() < 1e-6);
        assert!((cache.get("x", 0.0, 2.0).as_ref().unwrap().signed - 2.0).abs() < 1e-6);
    }
    #[test]
    fn undefined_discontinuous_overflowing_and_implicit_inputs_are_rejected() {
        for (s, a, b) in [
            ("1/x", -1.0, 1.0),
            ("1/(x-0.13)", -1.0, 1.0),
            ("ln(x)", -1.0, 1.0),
            ("tan(x)", -2.0, 2.0),
            ("x^2+y^2=1", -1.0, 1.0),
            ("1e308*x", 1.0, 2.0),
            ("x", f64::NAN, 1.0),
        ] {
            assert!(integrate(s, a, b).is_err(), "{s}");
        }
    }
}
