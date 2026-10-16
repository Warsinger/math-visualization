//! Reusable axis-aligned surface slicing, independent of UI and rendering.
use crate::{
    curve::{self, CurveSampling},
    expression::Equation,
    geometry::{Geometry, Sampling},
};

pub struct Slice {
    pub geometry: Geometry,
    pub plane: [[f32; 3]; 4],
}

pub fn sample(
    source: &str,
    sampling: Sampling,
    axis: usize,
    position: f32,
) -> Result<Slice, String> {
    let settings = sampling.validate()?;
    if axis > 2 || !position.is_finite() || position.abs() > settings.extent[axis] {
        return Err("Slice axis/position must lie within the applied domain.".into());
    }
    let equation = Equation::parse(source)?;
    let axes: Vec<usize> = (0..3).filter(|i| *i != axis).collect();
    let extent = [settings.extent[axes[0]], settings.extent[axes[1]]];
    let world = |p: [f32; 3]| {
        let mut point = [0.0; 3];
        point[axis] = position;
        point[axes[0]] = p[0];
        point[axes[1]] = p[1];
        point
    };
    let mut geometry = curve::contour_field(
        CurveSampling {
            extent,
            samples: settings.resolution.pow(2),
        },
        |point| equation.evaluate(world(point).map(f64::from)),
    )?;
    for line in &mut geometry.lines {
        *line = line.map(world);
    }
    geometry.bound = settings.extent.iter().map(|v| v * v).sum::<f32>().sqrt();
    let plane = [
        [-extent[0], -extent[1], 0.0],
        [extent[0], -extent[1], 0.0],
        [extent[0], extent[1], 0.0],
        [-extent[0], extent[1], 0.0],
    ]
    .map(world);
    Ok(Slice { geometry, plane })
}

#[derive(Default)]
pub struct Cache {
    key: Option<(String, Sampling, usize, u32)>,
    value: Option<Result<Slice, String>>,
}
impl Cache {
    pub fn get(
        &mut self,
        source: &str,
        settings: Sampling,
        axis: usize,
        position: f32,
    ) -> &Result<Slice, String> {
        let key = (source.to_owned(), settings, axis, position.to_bits());
        if self.key.as_ref() != Some(&key) {
            self.value = Some(sample(source, settings, axis, position));
            self.key = Some(key);
        }
        self.value.as_ref().expect("cache is initialized")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "manual hardware-dependent slice timing probe; no latency threshold"]
    fn profile_complex_maximum_resolution_slices() {
        let source = "sin(cos(sin(ln(x^2+y^2+z^2+1))))+cos(sin(cos(x*y*z)))+ln((sin(x*y)+cos(y*z)+sin(z*x))^2+1)-0.5";
        let settings = Sampling {
            extent: [10.0; 3],
            resolution: 40,
        };
        let mut cache = Cache::default();
        let mut times = Vec::new();
        for i in 0..21 {
            let start = std::time::Instant::now();
            cache
                .get(source, settings, i % 3, (i as f32 - 10.0) * 0.9)
                .as_ref()
                .unwrap();
            times.push(start.elapsed());
        }
        times.sort();
        let start = std::time::Instant::now();
        for _ in 0..1000 {
            std::hint::black_box(cache.get(source, settings, 2, 9.0));
        }
        println!(
            "21 cache misses at 6561 evaluations: median {:?}, p95 {:?}, max {:?}; 1000 cached lookups {:?}",
            times[10],
            times[19],
            times[20],
            start.elapsed()
        );
    }
    #[test]
    fn sphere_slices_move_in_every_axis_and_preserve_world_coordinates() {
        for axis in 0..3 {
            for position in [0.0, 0.5] {
                let slice = sample("x^2+y^2+z^2=1", Sampling::default(), axis, position).unwrap();
                assert!(!slice.geometry.lines.is_empty());
                for p in slice.geometry.lines.iter().flatten() {
                    assert_eq!(p[axis], position);
                    assert!((p.iter().map(|v| v * v).sum::<f32>() - 1.0).abs() < 0.02);
                }
            }
        }
    }
    #[test]
    fn empty_coincident_undefined_and_invalid_slices_are_explicit() {
        let s = Sampling::default();
        assert!(
            sample("x^2+y^2+z^2=1", s, 2, 2.0)
                .unwrap()
                .geometry
                .lines
                .is_empty()
        );
        assert!(sample("z=0", s, 2, 0.0).is_err());
        assert!(
            sample("ln(z)+x", s, 2, 0.0)
                .unwrap()
                .geometry
                .skipped_tetrahedra
                > 0
        );
        assert!(sample("x", s, 3, 0.0).is_err());
        assert!(sample("x", s, 2, f32::NAN).is_err());
        assert!(sample("x", s, 2, 4.0).is_err());
        let mut cache = Cache::default();
        assert!(cache.get("z", s, 2, 0.0).is_err());
        assert!(cache.get("x+y", s, 2, 0.0).is_ok());
    }
}
