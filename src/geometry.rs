use crate::expression::Equation;

pub type Point = [f32; 3];
/// Renderer-independent world-space lines (helix or triangle edges).
#[derive(Clone, Debug, Default)]
pub struct Geometry {
    pub lines: Vec<[Point; 2]>,
    pub bound: f32,
    pub skipped_tetrahedra: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Sampling {
    pub extent: [f32; 3],
    pub resolution: usize,
}
impl Default for Sampling {
    fn default() -> Self {
        Self {
            extent: [3.0; 3],
            resolution: 24,
        }
    }
}
impl Sampling {
    pub fn validate(self) -> Result<Self, String> {
        if !(8..=40).contains(&self.resolution) {
            return Err("Resolution must be 8–40 cells per axis.".into());
        }
        if self
            .extent
            .iter()
            .any(|v| !v.is_finite() || !(0.1..=10.0).contains(v))
        {
            return Err("Domain half-extents must be finite and between 0.1 and 10.".into());
        }
        Ok(self)
    }
}

/// Marching tetrahedra over a regular grid; sign crossings approximate F=0.
pub fn mesh(equation: &Equation, sampling: Sampling) -> Result<Geometry, String> {
    let s = sampling.validate()?;
    let n = s.resolution;
    let side = n + 1;
    let index = |x, y, z| (z * side + y) * side + x;
    let mut positions = Vec::with_capacity(side.pow(3));
    let mut values = Vec::with_capacity(side.pow(3));
    for z in 0..=n {
        for y in 0..=n {
            for x in 0..=n {
                let p = [x, y, z].map(|i| i as f32 / n as f32 * 2.0 - 1.0);
                let p = std::array::from_fn(|i| p[i] * s.extent[i]);
                positions.push(p);
                values.push(equation.evaluate(p.map(f64::from)));
            }
        }
    }
    let mut geometry = Geometry {
        bound: s.extent.iter().map(|v| v * v).sum::<f32>().sqrt(),
        ..Default::default()
    };
    // Consistent body diagonal on every cube prevents face triangulation cracks.
    let tetrahedra = [
        [0, 1, 2, 6],
        [0, 2, 3, 6],
        [0, 3, 7, 6],
        [0, 7, 4, 6],
        [0, 4, 5, 6],
        [0, 5, 1, 6],
    ];
    for z in 0..n {
        for y in 0..n {
            for x in 0..n {
                let cube = [
                    index(x, y, z),
                    index(x + 1, y, z),
                    index(x + 1, y + 1, z),
                    index(x, y + 1, z),
                    index(x, y, z + 1),
                    index(x + 1, y, z + 1),
                    index(x + 1, y + 1, z + 1),
                    index(x, y + 1, z + 1),
                ];
                for tet in tetrahedra {
                    let ids = tet.map(|i| cube[i]);
                    let Some(v) = ids.iter().map(|&i| values[i]).collect::<Option<Vec<_>>>() else {
                        geometry.skipped_tetrahedra += 1;
                        continue;
                    };
                    let p = ids.map(|i| positions[i]);
                    let inside: Vec<_> = (0..4).filter(|&i| v[i] < 0.0).collect();
                    let outside: Vec<_> = (0..4).filter(|&i| v[i] >= 0.0).collect();
                    let crossing = |a: usize, b: usize| {
                        // Scale before subtraction to avoid overflow of opposite huge values.
                        let scale = v[a].abs().max(v[b].abs());
                        let a_value = v[a] / scale;
                        let b_value = v[b] / scale;
                        let t = (a_value / (a_value - b_value)) as f32;
                        std::array::from_fn(|i| p[a][i] + (p[b][i] - p[a][i]) * t)
                    };
                    match inside.len() {
                        1 => triangle(
                            &mut geometry,
                            [
                                crossing(inside[0], outside[0]),
                                crossing(inside[0], outside[1]),
                                crossing(inside[0], outside[2]),
                            ],
                        )?,
                        3 => triangle(
                            &mut geometry,
                            [
                                crossing(outside[0], inside[0]),
                                crossing(outside[0], inside[1]),
                                crossing(outside[0], inside[2]),
                            ],
                        )?,
                        2 => {
                            let a = crossing(inside[0], outside[0]);
                            let b = crossing(inside[0], outside[1]);
                            let c = crossing(inside[1], outside[0]);
                            let d = crossing(inside[1], outside[1]);
                            triangle(&mut geometry, [a, b, c])?;
                            triangle(&mut geometry, [b, d, c])?;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(geometry)
}

fn triangle(g: &mut Geometry, p: [Point; 3]) -> Result<(), String> {
    let a: Point = std::array::from_fn(|i| p[1][i] - p[0][i]);
    let b: Point = std::array::from_fn(|i| p[2][i] - p[0][i]);
    let cross = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    if cross.iter().all(|v| *v == 0.0) {
        return Ok(());
    }
    if g.lines.len() + 3 > 120_000 {
        return Err(
            "Surface exceeds 120,000 wireframe edges. Reduce resolution or domain complexity."
                .into(),
        );
    }
    g.lines.extend([[p[0], p[1]], [p[1], p[2]], [p[2], p[0]]]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edge_budget_rejects_instead_of_delivering_truncated_geometry() {
        let mut g = Geometry {
            lines: vec![[[0.0; 3]; 2]; 119_999],
            ..Default::default()
        };
        assert!(triangle(&mut g, [[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]).is_err());
        assert_eq!(g.lines.len(), 119_999);
    }

    #[test]
    fn finer_sampling_improves_sphere_residual() {
        let eq = Equation::parse("x^2+y^2+z^2=4").unwrap();
        let residual = |n| {
            let g = mesh(
                &eq,
                Sampling {
                    resolution: n,
                    ..Default::default()
                },
            )
            .unwrap();
            g.lines
                .iter()
                .flatten()
                .map(|p| eq.evaluate(p.map(f64::from)).unwrap().abs())
                .fold(0.0_f64, f64::max)
        };
        assert!(residual(24) < residual(8));
    }
    fn sample(s: &str) -> Geometry {
        mesh(
            &Equation::parse(s).unwrap(),
            Sampling {
                resolution: 12,
                ..Default::default()
            },
        )
        .unwrap()
    }
    #[test]
    fn plane_vertices_are_on_plane_and_inside_domain() {
        let g = sample("x=0.37");
        assert!(!g.lines.is_empty());
        for p in g.lines.iter().flatten() {
            assert!((p[0] - 0.37).abs() < 1e-5);
            assert!(p.iter().all(|v| v.is_finite() && v.abs() <= 3.0));
        }
    }
    #[test]
    fn sphere_and_double_cone_are_general_expressions() {
        for source in ["x^2+y^2+z^2=4", "x^2-y^2-z^2=0"] {
            let g = sample(source);
            assert!(!g.lines.is_empty());
            let eq = Equation::parse(source).unwrap();
            for p in g.lines.iter().flatten() {
                assert!(eq.evaluate(p.map(f64::from)).unwrap().abs() < 0.3);
            }
        }
        let cone = sample("x^2-y^2-z^2");
        assert!(cone.lines.iter().flatten().any(|p| p[0] < -1.0));
        assert!(cone.lines.iter().flatten().any(|p| p[0] > 1.0));
    }
    #[test]
    fn sampling_failures_and_limits_are_explicit() {
        assert!(sample("x^2+y^2+z^2+1").lines.is_empty());
        assert!(sample("x^2").lines.is_empty()); // even root without sign change
        let invalid = sample("1/(x-x)");
        assert!(invalid.lines.is_empty());
        assert!(invalid.skipped_tetrahedra > 0);
        assert!(
            mesh(
                &Equation::parse("x").unwrap(),
                Sampling {
                    resolution: 41,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            Sampling {
                extent: [f32::NAN; 3],
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
}
