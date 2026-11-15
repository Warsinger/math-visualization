use crate::{
    expression::Equation,
    scene::{Helix, Scene},
    workspace::PlotKind,
};

/// Display the applied selection, including selections hidden in the workspace.
pub fn selected(scene: &Scene) -> Option<String> {
    if scene.spatial_active() {
        let plot = scene.spatial_workspace.as_ref()?.selected();
        return match plot.object {
            PlotKind::Helix { parameters } => Some(helix(parameters)),
            PlotKind::Surface => equation(&plot.source, false),
            PlotKind::Curve => equation(&plot.source, true).map(|eq| format!("{eq}   (z = 0)")),
        };
    }
    if let Some(curve) = &scene.curve {
        equation(&curve.source, true)
    } else if let Some(surface) = &scene.surface {
        equation(&surface.source, false)
    } else {
        Some(helix(scene.helix))
    }
}

fn helix(h: Helix) -> String {
    format!(
        "x = r cos(2πnt)\ny = r sin(2πnt)\nz = h(t − ½)   ·   0 ≤ t ≤ 1\nr = {}   ·   h = {}   ·   n = {}",
        h.radius, h.height, h.turns,
    )
}

fn equation(source: &str, planar: bool) -> Option<String> {
    let source = source.trim();
    if source.is_empty() {
        return None;
    }
    let mut formatted = String::new();
    let mut chars = source.chars().peekable();
    const SUPERSCRIPTS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    while let Some(ch) = chars.next() {
        match ch {
            '^' => {
                let mut probe = chars.clone();
                let mut digits = String::new();
                while probe.peek().is_some_and(char::is_ascii_digit) {
                    digits.push(probe.next().unwrap());
                }
                let next = probe.find(|ch| !ch.is_whitespace());
                if digits.is_empty() || matches!(next, Some('.' | 'e' | 'E' | '^')) {
                    formatted.push('^');
                } else {
                    for digit in digits.bytes() {
                        formatted.push(SUPERSCRIPTS[usize::from(digit - b'0')]);
                        chars.next();
                    }
                }
            }
            '*' => formatted.push('×'),
            '-' => formatted.push('−'),
            _ => formatted.push(ch),
        }
    }
    Some(if source.contains('=') {
        formatted
    } else if planar && Equation::planar_shorthand(source) {
        format!("y = {formatted}")
    } else {
        format!("{formatted} = 0")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::PlotMode;

    #[test]
    fn shows_applied_shorthand_with_the_correct_dimension() {
        let mut scene = Scene::default();
        assert!(selected(&scene).unwrap().contains("n = 5"));
        scene.request_in_mode("x^2", PlotMode::TwoD).unwrap();
        assert_eq!(selected(&scene).as_deref(), Some("y = x²"));
        scene.request_in_mode("x^2+y^2-1", PlotMode::TwoD).unwrap();
        assert_eq!(selected(&scene).as_deref(), Some("x²+y²−1 = 0"));
        scene.request_in_mode("x^2", PlotMode::ThreeD).unwrap();
        assert_eq!(selected(&scene).as_deref(), Some("x² = 0"));
    }

    #[test]
    fn leaves_complex_power_grouping_visible() {
        assert_eq!(equation("x^2^3", true).as_deref(), Some("y = x^2³"));
        assert_eq!(equation("y=x^2e-3/2", true).as_deref(), Some("y=x^2e−3/2"));
    }
}
