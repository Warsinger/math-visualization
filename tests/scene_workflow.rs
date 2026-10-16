use math_playground::{
    command,
    scene::{Helix, Scene},
};

#[test]
fn explicit_plot_mode_keeps_2d_and_3d_meanings_separate_and_history_restores_them() {
    use math_playground::{
        curve::CurveSampling,
        history::{History, Outcome, Snapshot},
        scene::PlotMode,
    };
    let mut scene = Scene::default();
    scene.request_in_mode("x^2", PlotMode::TwoD).unwrap();
    let planar = scene.geometry.lines.clone();
    for text in ["plot x^2", "y=x^2", "plot x^2=y"] {
        scene.request_in_mode(text, PlotMode::TwoD).unwrap();
        assert_eq!(scene.geometry.lines, planar);
    }
    let settings = CurveSampling {
        extent: [3.0, 8.0],
        samples: 512,
    };
    scene.plot_curve("x^2", settings).unwrap();
    let snapshot = Snapshot::capture(&scene);
    let mut history = History::default();
    history.record(
        "x^2".into(),
        Outcome::Success {
            snapshot: snapshot.clone(),
        },
    );
    let loaded = History::decode(&serde_json::to_vec(&history).unwrap()).unwrap();
    scene.request("plot y=x^2").unwrap();
    assert!(scene.curve.is_none());
    assert!(scene.surface.is_some());
    assert!(
        scene
            .geometry
            .lines
            .iter()
            .flatten()
            .any(|p| p[2].abs() > 0.1)
    );
    if let Outcome::Success { snapshot } = &loaded.entries[0].outcome {
        snapshot.restore(&mut scene).unwrap();
    } else {
        panic!("expected saved curve")
    }
    assert_eq!(scene.curve.as_ref().unwrap().sampling, settings);
    let before = scene.geometry.lines.clone();
    assert!(
        scene
            .request_in_mode("plot x^2+y^2=1", PlotMode::TwoD)
            .is_err()
    );
    assert_eq!(scene.geometry.lines, before);
    scene
        .request_in_mode("create a helix with five turns", PlotMode::TwoD)
        .unwrap();
    assert!(scene.curve.is_none() && scene.surface.is_none());
    snapshot.restore(&mut scene).unwrap();
    assert!(scene.curve.is_some());
}

#[test]
fn function_equations_mesh_and_preserve_scene_on_invalid_calls() {
    let mut scene = Scene::default();
    for request in [
        "plot ln(x)=y",
        "plot sin(x)=z",
        "plot cos(x)=y",
        "plot tan(x)=z",
        "plot ln(cos(x)+2)=y",
    ] {
        scene.request(request).unwrap();
        assert!(!scene.geometry.lines.is_empty(), "{request}");
        assert!(
            scene
                .geometry
                .lines
                .iter()
                .flatten()
                .flatten()
                .all(|v| v.is_finite())
        );
        if request == "plot ln(x)=y" {
            assert!(scene.geometry.skipped_tetrahedra > 0);
            assert!(scene.geometry.lines.iter().flatten().all(|p| p[0] > 0.0));
        }
    }
    let before = scene.geometry.lines.clone();
    assert!(scene.request("plot sin(x,y)").is_err());
    assert_eq!(scene.geometry.lines, before);
    scene.request("create a helix with five turns").unwrap();
    assert!(scene.surface.is_none());
}

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
