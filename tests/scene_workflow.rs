use math_playground::{
    command,
    scene::{Helix, Scene},
};

#[test]
fn implicit_products_share_plotting_history_and_exploration_semantics() {
    use math_playground::{
        derivative::tangent,
        expression::Equation,
        history::{History, Outcome, Snapshot},
        integral::integrate,
        scene::PlotMode,
    };
    for (implicit, explicit, modes) in [
        (
            "y=2x",
            "y=2*x",
            vec![PlotMode::Auto, PlotMode::TwoD, PlotMode::ThreeD],
        ),
        (
            "x^2+2y^2=1",
            "x^2+2*y^2=1",
            vec![PlotMode::Auto, PlotMode::TwoD, PlotMode::ThreeD],
        ),
        (
            "x^2+y^2+2z^2=1",
            "x^2+y^2+2*z^2=1",
            vec![PlotMode::Auto, PlotMode::ThreeD],
        ),
        (
            "2x^2",
            "2*x^2",
            vec![PlotMode::Auto, PlotMode::TwoD, PlotMode::ThreeD],
        ),
    ] {
        for mode in modes {
            let mut scene = Scene::default();
            scene.request_in_mode(explicit, mode).unwrap();
            let expected = scene.geometry.lines.clone();
            scene.request_in_mode(implicit, mode).unwrap();
            assert_eq!(scene.geometry.lines, expected, "{implicit}");
            let saved = Snapshot::capture(&scene);
            assert!(saved.request().contains(implicit));
            let mut history = History::default();
            history.record(
                implicit.into(),
                Outcome::Success {
                    snapshot: saved.clone(),
                },
            );
            let restored = History::decode(&serde_json::to_vec(&history).unwrap()).unwrap();
            scene.replace(Helix::default()).unwrap();
            if let Outcome::Success { snapshot } = &restored.entries[0].outcome {
                snapshot.restore(&mut scene).unwrap();
            } else {
                panic!("expected success");
            }
            assert_eq!(Snapshot::capture(&scene), saved);
            assert_eq!(scene.geometry.lines, expected);
            for bad in ["xy", "2e(x)", "x 2", "sin x", "x=2=3"] {
                assert!(scene.request_in_mode(bad, mode).is_err(), "{bad}");
                assert_eq!(Snapshot::capture(&scene), saved);
                assert_eq!(scene.geometry.lines, expected);
            }
        }
    }
    let residual = Equation::planar("y=2x").unwrap();
    assert_eq!(residual.evaluate([3.0, 6.0, 0.0]), Some(0.0));
    let a = tangent("2x^2", 2.0).unwrap();
    let b = tangent("2*x^2", 2.0).unwrap();
    assert_eq!((a.x, a.y, a.slope), (b.x, b.y, b.slope));
    let a = integrate("2x^2", -1.0, 2.0).unwrap();
    let b = integrate("2*x^2", -1.0, 2.0).unwrap();
    assert_eq!((a.signed, a.total), (b.signed, b.total));
}

#[test]
fn extended_functions_plot_and_restore_through_shared_evaluator() {
    use math_playground::{
        curve::CurveSampling,
        expression::Equation,
        geometry::Sampling,
        history::{Outcome, Snapshot, Store},
        scene::PlotMode,
    };
    let mut scene = Scene::default();
    for source in [
        "sqrt(x)",
        "cbrt(x)",
        "log(abs(x)+1,2)",
        "atan2(sin(x),cos(x))",
        "sinh(x)",
        "asin(x)",
        "acosh(x)",
        "min(abs(x),1)",
        "max(x,0)",
        "sign(x)",
        "sec(x)",
    ] {
        scene.request_in_mode(source, PlotMode::Auto).unwrap();
        assert!(scene.curve.is_some());
        assert!(!scene.geometry.lines.is_empty(), "{source}");
        assert!(
            scene
                .geometry
                .lines
                .iter()
                .flatten()
                .flatten()
                .all(|v| v.is_finite())
        );
        let saved = Snapshot::capture(&scene);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.json");
        let mut store = Store::open(Some(path.clone()));
        store.record(
            source.into(),
            Outcome::Success {
                snapshot: saved.clone(),
            },
        );
        let loaded = Store::open(Some(path));
        assert!(loaded.warning.is_none());
        if let Outcome::Success { snapshot } = &loaded.history.entries[0].outcome {
            snapshot.restore(&mut scene).unwrap();
        } else {
            panic!("successful snapshot expected");
        }
        assert_eq!(Snapshot::capture(&scene), saved);
    }
    for source in [
        "sqrt(x)=z",
        "log(abs(x)+1,2)+y=z",
        "atan2(y,x)=z",
        "tanh(x)=z",
        "min(x,z)=y",
    ] {
        scene.request_in_mode(source, PlotMode::Auto).unwrap();
        assert!(scene.surface.is_some());
        assert!(!scene.geometry.lines.is_empty(), "{source}");
        assert!(
            scene
                .geometry
                .lines
                .iter()
                .flatten()
                .flatten()
                .all(|v| v.is_finite())
        );
    }
    scene.plot("sqrt(x)=z", Sampling::default()).unwrap();
    assert!(scene.geometry.skipped_tetrahedra > 0);
    assert!(scene.geometry.lines.iter().flatten().all(|p| p[0] >= 0.0));
    let before = scene.geometry.lines.clone();
    for source in ["log02(x)", "atan2(x)", "sqrt(x,2)", "max(x,)"] {
        assert!(scene.request_in_mode(source, PlotMode::Auto).is_err());
        assert_eq!(scene.geometry.lines, before);
    }
    let mut planar = Scene::default();
    planar
        .plot_curve("cbrt(x)", CurveSampling::default())
        .unwrap();
    let p = planar
        .geometry
        .lines
        .iter()
        .flatten()
        .find(|p| p[0] < -1.0)
        .unwrap();
    assert!(
        Equation::planar("cbrt(x)")
            .unwrap()
            .evaluate(p.map(f64::from))
            .unwrap()
            .abs()
            < 1e-5
    );
}

#[test]
fn extended_functions_work_with_tangents_and_integrals_without_claiming_smooth_corners() {
    use math_playground::{derivative::tangent, integral::integrate};
    for (source, x, slope) in [
        ("sqrt(x)", 4.0, 0.25),
        ("cbrt(x)", -8.0, 1.0 / 12.0),
        ("log2(x)", 1.0, 1.0 / std::f64::consts::LN_2),
        ("tanh(x)", 0.0, 1.0),
    ] {
        let t = tangent(source, x).unwrap();
        assert!((t.slope - slope).abs() < 1e-4, "{source}: {t:?}");
    }
    for source in [
        "abs(x)",
        "0.00001*abs(x)",
        "min(x,-x)",
        "max(x,-x)",
        "sign(x)",
        "sqrt(x)",
        "cbrt(x)",
        "atan2(x,-1)",
        "csc(x)",
    ] {
        assert!(tangent(source, 0.0).is_err(), "{source}");
    }
    for (source, a, b, expected) in [
        ("sqrt(x)", 0.0, 1.0, 2.0 / 3.0),
        ("abs(x)", -1.0, 1.0, 1.0),
        ("max(x,0)", -1.0, 1.0, 0.5),
        ("min(x,0)", -1.0, 1.0, -0.5),
    ] {
        let area = integrate(source, a, b).unwrap();
        assert!((area.signed - expected).abs() < 1e-5, "{source}");
    }
    for source in ["sqrt(x)", "log2(x)", "csc(x)", "sign(x)", "atan2(x,-1)"] {
        assert!(integrate(source, -1.0, 1.0).is_err(), "{source}");
    }
}

#[test]
fn automatic_dimension_selection_and_implicit_history() {
    use math_playground::{
        history::{History, Outcome, Snapshot},
        scene::PlotMode,
    };
    let mut scene = Scene::default();
    for source in ["x^2+y^2=1", "x=2", "plot sin(x)+cos(y)=0", "y=x^2", "x^2"] {
        scene.request_in_mode(source, PlotMode::Auto).unwrap();
        assert!(scene.curve.is_some(), "{source}");
    }
    scene.request_in_mode("x^2+y^2=1", PlotMode::Auto).unwrap();
    let saved = Snapshot::capture(&scene);
    let mut history = History::default();
    history.record("circle".into(), Outcome::Success { snapshot: saved });
    let loaded = History::decode(&serde_json::to_vec(&history).unwrap()).unwrap();
    scene
        .request_in_mode("x^2+y^2+z^2=1", PlotMode::Auto)
        .unwrap();
    assert!(scene.surface.is_some());
    scene
        .request_in_mode("plot y=x^2", PlotMode::ThreeD)
        .unwrap();
    assert!(scene.surface.is_some());
    if let Outcome::Success { snapshot } = &loaded.entries[0].outcome {
        snapshot.restore(&mut scene).unwrap();
    } else {
        panic!("expected saved circle")
    }
    assert!(scene.curve.is_some());
    assert!(!scene.geometry.lines.is_empty());
}

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
            .request_in_mode("plot x^2+z^2=1", PlotMode::TwoD)
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
