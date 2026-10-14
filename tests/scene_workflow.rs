use math_playground::{
    command,
    scene::{Helix, Scene},
};

#[test]
fn surfaces_sampling_and_helix_switching_preserve_valid_state() {
    use math_playground::geometry::Sampling;
    let mut scene = Scene::default();
    scene.request("plot x^2-y^2-z^2=0").unwrap();
    let source = scene.surface.as_ref().unwrap().source.clone();
    assert!(!scene.geometry.lines.is_empty());
    let before = scene.geometry.lines.clone();
    assert!(scene.request("plot x + unknown").is_err());
    assert_eq!(scene.geometry.lines, before);
    assert!(scene.request("set radius 2").is_err());
    let sampling = Sampling {
        extent: [2.0, 3.0, 4.0],
        resolution: 16,
    };
    scene.plot(&source, sampling).unwrap();
    assert_eq!(scene.surface.as_ref().unwrap().sampling, sampling);
    scene.request("plot x=0.3").unwrap();
    assert_eq!(scene.surface.as_ref().unwrap().sampling, sampling);
    let before = scene.geometry.lines.clone();
    assert!(
        scene
            .plot(
                "x",
                Sampling {
                    resolution: 100,
                    ..sampling
                }
            )
            .is_err()
    );
    assert_eq!(scene.geometry.lines, before);
    scene.request("create a helix with three turns").unwrap();
    assert!(scene.surface.is_none());
    assert_eq!(scene.helix.turns, 3.0);
    assert_eq!(scene.geometry.lines.len(), 288);
}

#[test]
fn commands_and_controls_share_state_and_errors_are_atomic() {
    let mut scene = Scene::default();
    // The controls submit a candidate through the same boundary as commands.
    scene
        .replace(Helix {
            radius: 2.0,
            ..scene.helix
        })
        .unwrap();
    let requested = command::parse("set turns to eight", scene.helix).unwrap();
    scene.replace(requested).unwrap();
    assert_eq!(scene.helix.radius, 2.0);
    assert_eq!(scene.helix.turns, 8.0);
    assert_eq!(scene.helix.pitch(), 0.5);
    let before = scene.helix;
    let result = command::parse("set radius 3 and turns -1", scene.helix)
        .and_then(|candidate| scene.replace(candidate));
    assert!(result.is_err());
    assert_eq!(scene.helix, before);
    assert_eq!(scene.helix.points().len(), 769);
}

#[test]
fn create_replaces_the_single_selected_object_with_defaults() {
    let old = Helix {
        radius: 3.0,
        height: 10.0,
        turns: 8.0,
    };
    let created = command::parse("create a helix with two turns", old).unwrap();
    assert_eq!(
        created,
        Helix {
            turns: 2.0,
            ..Helix::default()
        }
    );
}
