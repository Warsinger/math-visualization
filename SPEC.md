# Mathematical playground — initial project specification

## Purpose

Learn Rust and 3D visualization by building an approachable mathematical playground inspired by the curiosity of Numberphile curve and shape demonstrations. End users explore through typed requests and direct controls; they never need to write code. The developer learns the math and engineering underneath through small, testable milestones.

## First release scope

A Windows desktop app starts with one selected helix of radius 1, height 4, and five turns. A large interactive canvas displays sampled 3D geometry. Requests and sliders/numeric entry edit the same scene. Users orbit, tilt, zoom, and reset the camera. The app shows its parameter interpretation and helpful errors without changing the last valid scene.

The helix is a line with a color progression. Projection is orthographic; crossing lines have no occlusion. A small deterministic parser supports documented phrases without accounts, network access at runtime, or AI credentials. The next implemented milestone adds general bounded implicit equations as triangle-edge wireframes, described below. Full conversational understanding, multiple objects, saving, export, taper, animation, shaded meshes, and browser deployment are later work.

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

- `src/scene.rs`: canonical selected helix/surface, validated edits, and cached geometry.
- `src/command.rs`: text → candidate helix → validation. Parsing never mutates state.
- `src/lib.rs`: exposes pure logic for tests and later alternate interfaces.
- `src/main.rs`: widgets and feedback; both edit paths call `Scene::replace`.
- `src/expression.rs`: bounded arithmetic parser and real-valued equation evaluation.
- `src/geometry.rs`: renderer-independent world-space lines, sampling settings, and marching tetrahedra.
- `src/view.rs`: shared camera rotation/projection and shape-independent canvas painting.
- `tests/scene_workflow.rs`: verifies combined control/command behavior.

The helix is x = r cos(2πnt), y = r sin(2πnt), z = h(t − 1/2), with 0 ≤ t ≤ 1. Sampling uses 96 segments per turn, capped indirectly by the validated turn bound (maximum 1,920 segments). Camera math rotates around the Z axis and then tilts before discarding depth. Both shapes cache renderer-independent world-space line segments, regenerated only on applied edits. A later GPU renderer can consume this geometry without changing expression interpretation or scene validation.

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

## Implemented extension: persistent chronological history

Issue #3 adds local history for repeatable exploration/manual testing, distinct from saved/shareable scenes in issue #16. The History window lists up to 200 entries newest first, with UTC timestamps, request text and success/error status. Successful requests and applied sampling changes capture a renderer-independent snapshot (helix parameters or surface equation plus sampling settings). Restore rebuilds through the existing background-worker/validated-scene path, preserving the current plot on failure. Restores are not recorded as new entries. Edit/retry only copies text to input; current scene settings apply on submission. Add current plot explicitly captures slider-adjusted states; slider ticks and camera movements are not logged. Clear history persists an empty list when saving is available.

`src/history.rs` separates schema, snapshot reconstruction and storage from egui. Serde/serde_json encode a version-1 JSON document; directories resolves per-user data storage; tempfile handles same-directory replacement; chrono formats UTC timestamps without using a platform clock service. Windows uses LocalAppData/MathPlayground/history.json, Linux uses XDG_DATA_HOME (fallback ~/.local/share), and macOS uses ~/Library/Application Support. The UI displays the resolved path. No cloud account or upload is involved.

Loading is bounded to 2 MB and validates version, count, text sizes, timestamps, parameters and expression syntax without meshing all entries. Saves sync a temporary file before replacement. Save failures keep session history and show a warning. Unreadable, damaged, oversized or unknown-version files are never overwritten during that session; recovery requires moving/repairing the file and restarting. History remains bounded in memory. This does not provide multi-instance merging or guaranteed power-loss durability; simultaneous instances are last-writer-wins. Geometry and camera are excluded. Tests cover disk round trips/replacement, retention, clearing, invalid/corrupt/future data, unavailable storage, context-independent restoration and completion/error recording.

## Reference documentation

- [eframe App API](https://docs.rs/eframe/0.36.2/eframe/trait.App.html)
- [eframe dependency features and Rust requirement](https://docs.rs/crate/eframe/0.36.2/source/Cargo.toml)
- [Rust installation](https://rust-lang.org/tools/install/)
- [Windows MSVC prerequisites](https://rust-lang.github.io/rustup/installation/windows-msvc.html)

## Implemented extension: general Cartesian implicit equations

The app accepts `plot x^2-y^2-z^2=0` through a general evaluator. An equation is one scalar function F(x,y,z)=0; a left/right equality is normalized by subtraction. Input is data in a restricted arithmetic language, never executable source code. Real finite results alone participate in extraction. One or two variables are allowed and naturally produce surfaces extended along unused axes. Variable-free equations are rejected. Polar coordinates and systems of equations are outside this pass.

Grammar: lowercase x/y/z, finite decimal/scientific numeric literals, unary +/- and binary +, -, *, /, ^, parentheses, and one-argument function calls `ln(expression)`, `sin(expression)`, `cos(expression)`, `tan(expression)`. Nested calls and arithmetic arguments are supported. Powers are right-associative and bind more strongly than unary signs; multiplication is explicit. No other named functions, pi, assignments, or inequalities. Input is capped at 256 bytes and parser recursion at 64 levels, including function arguments. These limits keep evaluation cost bounded without introducing another dependency.

Functions use standard-library f64 math. Trig arguments are radians; ln accepts strictly positive arguments only. Every evaluation node must return a finite real result; invalid intermediate values propagate as missing samples rather than being hidden by later operations. Tangent excludes samples when the absolute cosine of its argument is at most 1e-12. This numerical guard excludes near-pole samples but cannot prevent a grid edge from spanning a discontinuity; false surfaces across poles remain possible. Large angles have floating-point argument-reduction limits. No inverse trig or other functions are included in this pass. Function tests cover known values, nested expressions, precedence, syntax, resource bounds, domain edges, masked invalid intermediates, pole-side signs, and scene meshing.

The adjustable finite Cartesian domain is [-a,a]×[-b,b]×[-c,c], with independent half-extents from 0.1 to 10. Resolution is 8–40 cells along each axis, default 24. The grid is evaluated once; six consistently oriented tetrahedra per cell interpolate sign-crossing edges and produce one or two triangles. Exact-zero samples count as nonnegative; degenerate triangles are omitted. Nonfinite corners exclude that tetrahedron. Linear interpolation uses scaled values to avoid overflow when subtracting large opposite signs.

Maximum work: 68,921 evaluations, 384,000 tetrahedra, 120,000 output triangle edges. Excess output is an error rather than a silently truncated surface. Bounded work does not guarantee a hardware-independent runtime. Surface work runs on one background thread. Existing cached geometry remains usable by the camera while it runs; scene edits are temporarily disabled, preventing stale worker results from overwriting newer edits. Only successful completion replaces the scene, including sampling metadata. Apply sampling explicitly rebuilds; typing a new equation uses the last applied surface settings. There is no cancellation in this pass. Helix controls keep using the same validated scene boundary and regenerate their small geometry immediately.

This is numerical sampling, not an exact real-solution solver. Thin features, singularities, tangencies/even roots may vanish; discontinuities can yield spurious crossings; boundary-clipped surfaces are incomplete. An empty result is not proof that the real solution set is empty. A single scalar equation generally defines a surface and cannot serve as a general arbitrary-space-curve interface. The display explicitly identifies wireframe projection without depth occlusion. Shared/repeated triangle edges are retained; it is a correctness-first initial renderer.

Acceptance: the cone, sphere, plane, and saddle work via the same evaluator; both cone halves appear; camera controls remain shared; independent domain extents and resolution rebuild the current equation; invalid input/settings preserve scene and geometry; returning to a helix retains the initial command/control behavior. Automated tests cover precedence, invalid/non-real evaluation, plane residuals, sphere refinement, cone halves, bounds/output cap, switching, worker completion/failure, and finite headless UI shapes. Full visual/interaction review remains a manual acceptance step in TESTING.md. Rich formula typography remains pending in [issue #4](https://github.com/Warsinger/math-visualization/issues/4). [GitHub Issues](https://github.com/Warsinger/math-visualization/issues) is the sole feature tracker; the milestones above describe the learning progression, not assigned issue priorities or milestones.
