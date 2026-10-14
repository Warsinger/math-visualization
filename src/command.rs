use crate::scene::Helix;

fn number(word: &str) -> Option<f32> {
    match word {
        "one" => Some(1.0),
        "two" => Some(2.0),
        "three" => Some(3.0),
        "four" => Some(4.0),
        "five" => Some(5.0),
        "six" => Some(6.0),
        "seven" => Some(7.0),
        "eight" => Some(8.0),
        "nine" => Some(9.0),
        "ten" => Some(10.0),
        _ => word.parse().ok(),
    }
}

/// A deliberately small grammar, not an AI interpreter. No partial mutations.
pub fn parse(text: &str, current: Helix) -> Result<Helix, String> {
    let normalized = text.trim().to_lowercase().replace([',', '='], " ");
    let words: Vec<_> = normalized.split_whitespace().collect();
    let (mut next, start) = if words.starts_with(&["create", "a", "helix", "with"]) {
        (Helix::default(), 4)
    } else if words.starts_with(&["set"]) {
        (current, 1)
    } else {
        return Err("Try: create a helix with five turns; or set radius to 2.".into());
    };
    let mut i = start;
    let mut seen = [false; 4];
    while i < words.len() {
        let (parameter, value, consumed) = if let Some(value) = number(words[i]) {
            (words.get(i + 1).copied().unwrap_or(""), value, 2)
        } else {
            let skip = usize::from(words.get(i + 1) == Some(&"to"));
            let value = words
                .get(i + 1 + skip)
                .and_then(|w| number(w))
                .ok_or_else(|| format!("Expected a number after '{}'.", words[i]))?;
            (words[i], value, 2 + skip)
        };
        let slot = match parameter {
            "radius" => 0,
            "height" => 1,
            "turns" => 2,
            "pitch" => 3,
            _ => {
                return Err(format!(
                    "Unknown parameter '{parameter}'. Use radius, height, turns, or pitch."
                ));
            }
        };
        if seen[slot] {
            return Err(format!("Specify {parameter} only once."));
        }
        seen[slot] = true;
        match slot {
            0 => next.radius = value,
            1 => next.height = value,
            2 => next.turns = value,
            _ => next.height = value, // temporarily store pitch; resolve after turns
        }
        i += consumed;
        if words.get(i) == Some(&"and") {
            i += 1;
        }
        if i == words.len() && words.last() == Some(&"and") {
            return Err("Expected a parameter after 'and'.".into());
        }
    }
    if !seen.iter().any(|s| *s) {
        return Err("Include at least one parameter.".into());
    }
    if seen[1] && seen[3] {
        return Err("Use height or pitch in one request, not both.".into());
    }
    if seen[3] {
        next.height *= next.turns;
    }
    next.validate()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_first_request_and_numeric_edits() {
        let h = parse("create a helix with five turns", Helix::default()).unwrap();
        assert_eq!(h.turns, 5.0);
        let h = parse("set radius to 2 and turns to 3.5", h).unwrap();
        assert_eq!((h.radius, h.turns), (2.0, 3.5));
    }

    #[test]
    fn pitch_uses_final_turn_count_regardless_of_order() {
        for text in ["set pitch 0.5 and turns 4", "set turns 4 and pitch 0.5"] {
            assert_eq!(parse(text, Helix::default()).unwrap().height, 2.0);
        }
    }

    #[test]
    fn rejects_ambiguous_invalid_and_unsupported_requests() {
        for text in [
            "",
            "set",
            "set radius NaN",
            "set turns 0",
            "set radius 2 bananas",
            "set radius 2 and",
            "set height 2 pitch 1",
            "set radius 2 radius 3",
            "animate helix",
        ] {
            assert!(parse(text, Helix::default()).is_err(), "{text}");
        }
    }
}
