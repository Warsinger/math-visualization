# Testing the playground

## Automated checks

Run the four commands in README or the VS Code **Verify** task before each meaningful change. rustfmt provides consistent layout; Clippy catches common mistakes; tests protect behavior; build ensures the GUI binary links. CI repeats these checks on Windows x64, Linux x64, and macOS Apple Silicon and builds release executables. Avoid accepting a green geometry test as proof that the GUI works.

## CI packages

`scripts/package.ps1 -Target <triple>` packages the matching `cargo +stable build --locked --release --target <triple>` output. It rejects missing/empty binaries and verifies that the archive contains exactly the executable, RUNNING.md and build-info.json, then generates a SHA-256 checksum. Unix tar archives retain executable permissions inside the outer Actions ZIP. Run the script with PowerShell 7; supported triples are listed in [download instructions](docs/DOWNLOADS.md).

For workflow changes, validate GitHub Actions syntax (for example with actionlint), run local Rust checks and package the native target. Confirm every matrix job passes in GitHub before treating the other platforms as verified. Download each platform artifact, compare the checksum, inspect its contents, and perform the UI protocol on an actual matching graphical desktop. CI compilation/headless tests do not establish driver compatibility. Signing/installers and release publishing are outside this workflow.

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
8. Resize to the current minimum 950×760. Confirm the field, controls, canvas, feedback, and formula remain usable.

## Next automation step

The widget construction now lives in a testable `draw` function with a headless drawing smoke test. Next, use egui's test tooling to inject clicks, keyboard submission, and pointer motion. Capture a deterministic default-view image with fixed parameters and viewport. Add screenshot comparison with a small tolerance for fonts/antialiasing and preserve reference images for review. For a depth renderer, add tests for image occlusion alongside existing camera/projection numeric tests. Input injection and image comparison are planned, not claimed as executed.

## Equation regression tests and manual checks

The equation extension adds parser precedence/right-associative powers, general evaluation, malformed/unknown syntax, byte/depth limits, non-real/nonfinite evaluation, plane residual/domain checks, sphere refinement, both cone halves, empty/even-root sampling, skipped invalid regions, sampling limits, edge-budget rejection, helix/surface switching, failure atomicity, and background worker completion. Headless egui tests construct both scene interfaces and check finite drawing bounds; they do not verify pixels, layout readability, or interaction. No new dependencies were needed.

Manual extension checklist:

1. Apply `plot x^2-y^2-z^2=0`. Confirm two cone halves along X, surface controls, and wireframe/no-depth label. Rotate/zoom with the existing camera.
2. Apply sphere `plot x^2+y^2+z^2=4`, saddle `plot z-x*y`, and plane `plot x=0.3`. Confirm they differ without any shape-template controls.
3. Change X/Y/Z half-extents and resolution independently; **Apply sampling** must rebuild. Confirm the next equation uses the applied sampling values. Increase resolution to compare approximation; do not assume extra resolution proves exactness.
4. While computing, confirm camera interaction remains possible and editing is disabled until completion. Invalid equations (`plot w`, `plot x=0=1`) must preserve the preceding scene.
5. `plot x^2=0` may return no crossings; `plot 1/(x-x)` skips undefined regions. Confirm empty-result feedback avoids claiming no solutions. `plot 1/(x-0.13)` may produce a false plane: this is the documented discontinuity limitation.
6. Return with `create a helix with five turns`; test radius slider and `set height 6` again.
7. Inspect default/minimum size with the example expander open. Check long equation text, error feedback, and sampling/camera controls are readable.

Automated checks are recorded in README. Native startup is a smoke check only; interactive manual review and image comparison remain outstanding.

## Function checks

Automated tests verify ln at e, sine/cosine/tangent at known radian angles, nested calls and power/unary precedence, required parentheses/one argument, unknown names, retained parser limits, log at positive/zero/negative inputs, overflow and invalid intermediates, and the tangent near-pole guard plus signs on either side. Scene tests mesh all four functions and a nested expression, verify finite geometry and skipped logarithm-domain regions, preserve geometry on malformed calls, and return to a helix.

Manual review: try `plot ln(x)=y`, `plot sin(x)=z`, `plot cos(x)=y`, `plot tan(x)=z`, and `plot ln(cos(x)+2)=y`. These are 3D surfaces extended along the unused coordinate, not 2D plots. Check log-domain feedback, rotate the surfaces, and narrow X to ±1 for a tangent branch without poles. With X at ±3, pole-crossing artifacts can occur despite near-pole sample exclusion. Check malformed `plot sin(x,y)` preserves the preceding scene and the new function examples remain readable. Pixel/interaction review remains pending.
