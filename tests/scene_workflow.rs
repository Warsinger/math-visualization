use math_playground::{
    command,
    scene::{Helix, Scene},
};

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
