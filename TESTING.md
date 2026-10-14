# Testing the playground

## Automated checks

Run the four commands in README or the VS Code **Verify** task before each meaningful change. rustfmt provides consistent layout; Clippy catches common mistakes; tests protect behavior; build ensures the GUI binary links. CI repeats them on Windows. Avoid accepting a green geometry test as proof that the GUI works.

Current tests cover radius and endpoints over the sampled curve, sampling density, default creation, numeric and word commands, fractional turns, pitch/turn ordering, invalid and unsupported syntax, ambiguous parameters, and failure atomicity. Camera tests verify center/vertical mapping and a quarter-turn rotation. Integration tests simulate a control edit followed by a command and verify both use one scene. All 9 tests passed during initial setup. Future regressions should become focused tests before fixes.

## Manual UI/render protocol

Launch with `cargo run`; record Rust version, OS, graphics adapter, window size, observed outcome, and any screenshot when reviewing a release.

1. At 1150×760, confirm the initial five-turn helix is visible, axes/labels readable, and controls do not overlap the canvas.
2. Apply the initial example, then `set turns to 8`. Confirm Turns shows 8, Height remains 4, and Pitch becomes 0.5.
3. Type Radius 2 through its numeric control. Apply `set height 6`; radius must remain 2.
4. Apply `set pitch 0.5 and turns 4`, then reverse their order. Both give Height 2 and Pitch 0.5.
5. Apply `set radius 3 and turns -1`; confirm a visible error and unchanged controls/curve. Try unsupported `animate helix`.
6. Drag horizontally and vertically; scroll to zoom; confirm input outside the canvas does not rotate/zoom it. Reset view and inspect framing.
7. Check fractional Turns 3.5, bounds on each slider, Enter submission, and the request-examples expander.
8. Resize to 850×620. Confirm the field, controls, canvas, feedback, and formula remain usable.

## Next automation step

Once the UI settles, extract widget construction into a testable UI function and use egui's test tooling to inject clicks, keyboard submission, and pointer motion. Capture a deterministic default-view image with fixed parameters and viewport. Add screenshot comparison with a small tolerance for fonts/antialiasing and preserve reference images for review. For a depth renderer, add camera/projection numeric tests and test image occlusion. These UI/image checks are planned, not claimed as executed by the initial scaffold.
