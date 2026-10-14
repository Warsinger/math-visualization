# Mathematical playground — initial project specification

## Purpose

Learn Rust and 3D visualization by building an approachable mathematical playground inspired by the curiosity of Numberphile curve and shape demonstrations. End users explore through typed requests and direct controls; they never need to write code. The developer learns the math and engineering underneath through small, testable milestones.

## First release scope

A Windows desktop app starts with one selected helix of radius 1, height 4, and five turns. A large interactive canvas displays sampled 3D geometry. Requests and sliders/numeric entry edit the same scene. Users orbit, tilt, zoom, and reset the camera. The app shows its parameter interpretation and helpful errors without changing the last valid scene.

The initial curve is a line, with a color progression and start marker. It is not yet a shaded tube or solid surface. Projection is orthographic; crossing lines have no occlusion. A small deterministic parser supports documented phrases without accounts, network access at runtime, or AI credentials. Full conversational understanding, multiple objects, saving, export, taper, animation, arbitrary equations, meshes, and browser deployment are later work.

## Experience

The request field appears above the canvas, with Apply and Enter submission. Examples are available in place. Selected-object controls sit beside the canvas. Each slider exposes its number for precise typed entry. Radius, total height, and turns are independent; fractional turns are supported. Pitch is displayed as height / turns. A pitch request sets height using the final requested turn count. A request that includes height and pitch together is rejected as ambiguous.

Bounds: radius 0.1–5, height 0.1–20, turns 0.25–20, in abstract units. Every edit must be finite and within these bounds. Changing turns through controls holds total height fixed. Creating a helix replaces the single object and resets unspecified parameters to defaults; setting parameters preserves unspecified values. Camera zoom and angles persist across object edits; Reset view restores the initial camera. The view fits the current bounding sphere at zoom 1, so its apparent scale changes as dimensions change; explicit world-scale camera behavior is a later design decision.

Supported examples:

- `create a helix with five turns`
- `create a helix with radius 2 and height 6 and turns 8`
- `set radius to 2 and turns to 3.5`
- `set pitch to 0.5 and turns to 8`

The grammar accepts parameter-before-number or number-before-parameter, optional `to`, optional `and`, commas, equals signs, digits and number words one–ten. Unsupported wording receives an error. This deliberately bounded first implementation must be described honestly as limited natural-language input. Future interpretation should produce structured validated edits through the existing boundary, never execute user-provided code.

## Architecture and choices

Rust 2024 with Cargo handles source, builds, and tests. eframe 0.36.2 supplies egui controls and a desktop window, using its Glow backend. It is the only direct runtime dependency. This keeps the initial code approachable and avoids starting with an engine, ECS, shader pipeline, or web/frontend split. The version and App API were checked against official crate documentation on October 14, 2026; eframe requires Rust 1.95 or later.

- `src/scene.rs`: canonical helix, validation, and pure geometry sampling.
- `src/command.rs`: text → candidate helix → validation. Parsing never mutates state.
- `src/lib.rs`: exposes pure logic for tests and later alternate interfaces.
- `src/main.rs`: widgets and feedback; both edit paths call `Scene::replace`.
- `src/view.rs`: camera rotation/projection and canvas painting.
- `tests/scene_workflow.rs`: verifies combined control/command behavior.

The helix is x = r cos(2πnt), y = r sin(2πnt), z = h(t − 1/2), with 0 ≤ t ≤ 1. Sampling uses 96 segments per turn, capped indirectly by the validated turn bound (maximum 1,920 segments). Camera math rotates around the Z axis and then tilts before discarding depth. For this bounded scene, regenerating a small polyline per redraw is acceptable. Cache geometry only when profiling shows a reason. A later GPU renderer can consume the same world-space points, leaving parsing and scene tests intact.

## Milestones and learning progression

1. **Review and launch:** inspect this spec; install Rust and Windows linker prerequisites; build the app. Learn Cargo, modules, structs, ownership, and Result through the existing code.
2. **Reliable helix exploration:** verify requests, precise controls, camera interaction, errors, and small-window layout. Learn floating-point tolerances, iterators, trigonometry, regression testing, and CI. This scaffold implements the intended behavior; validation status is recorded separately in README.
3. **Broader language:** add agreed phrases as test cases, structured commands, undo, and helpful ambiguity handling. Learn enums and separating interpretation from mutation. Evaluate optional AI only after a useful offline path exists.
4. **Taper:** define radius as a function of t; add controls, requests, mathematical invariants, and regression tests.
5. **Animation:** explicit play/pause and time parameters; use a deterministic clock in tests. Learn state evolution and repaint scheduling.
6. **Richer visualization:** tube meshes, perspective, depth testing, more shapes, multiple selection, persistence, and export as separate decisions. Learn vectors, matrices, GPU buffers, serialization, and performance measurement incrementally.

## Acceptance criteria for the helix milestone

- Launching opens the app with a five-turn helix and readable labels.
- The example first request succeeds; an eight-turn request visibly changes the geometry and controls.
- Radius/height/turns updates through either path are reflected in the same object and pitch display.
- Fractional turns work; pitch edits are order-independent with respect to turns.
- Invalid, nonfinite, out-of-range, duplicate, ambiguous, or unsupported requests leave the prior object intact and explain the failure.
- Drag/scroll and camera controls work; Reset view restores predictable framing.
- Geometry radius and endpoints agree with the formula within floating-point tolerance.
- Formatting, Clippy, unit/integration tests, and build pass locally and in CI.
- Manual UI checks in TESTING.md pass at default and minimum window sizes.

## Testing and engineering practice

Automate mathematical invariants, parser semantics, and shared-state workflows first. Tests should protect observable behavior, including failure atomicity, rather than duplicate implementation. CI checks formatting, treats lint warnings as failures, runs every test target, and compiles the app on Windows. Keep Cargo.lock under version control after resolution so everyone tests the same dependency graph. No commit or push is part of this setup.

UI verification begins with a repeatable manual checklist because geometry tests cannot prove readable layout or usable camera interaction. Add automated egui input tests and screenshot capture at a fixed viewport/camera once layout stabilizes. Keep image comparisons tolerant of antialiasing and platform fonts; use them alongside numerical geometry checks. Future timing, multi-object, persistence, or GPU features must add targeted regression tests before shipping. See TESTING.md for current commands and the initial UI protocol.

## Reference documentation

- [eframe App API](https://docs.rs/eframe/0.36.2/eframe/trait.App.html)
- [eframe dependency features and Rust requirement](https://docs.rs/crate/eframe/0.36.2/source/Cargo.toml)
- [Rust installation](https://rust-lang.org/tools/install/)
- [Windows MSVC prerequisites](https://rust-lang.github.io/rustup/installation/windows-msvc.html)
