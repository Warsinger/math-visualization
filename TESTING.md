# Testing the playground

## Local browser checks

See [BROWSER.md](docs/BROWSER.md) for build/serve commands, the tested browser baseline, manual results and known limits. Build the Wasm target and run its lint with `cargo clippy --locked --target wasm32-unknown-unknown -- -D warnings`; then use the local browser protocol there. Native CI does not currently run browser UI tests.

## 2D Cartesian checks

Automated tests cover equality of bare/shorthand/explicit/reversed parabola forms, zero Z coordinates, clipping and domains, constant functions, invalid variables/implicit forms and sampling limits, unchanged 3D cylinders, history JSON/restoration and mode/control synchronization, planar camera projection, headless drawing, and non-overlapping history layout at minimum/default sizes in both modes.

Manual review: use **Auto** or select **2D · XY** and apply `x^2`, `plot y=x^2`, and `x^2=y`; verify the same flat parabola. Try `sin(x)`, `ln(x)` and `y=4`. Pan, zoom and Reset view; confirm Orbit/Tilt are absent. Edit X/Y limits and sample count, then Apply sampling. Restore the plot from History after switching to a 3D surface; verify mode and controls return. Select **3D · implicit**, apply `plot y=x^2`, and rotate its cylinder. Confirm switching the input selector without Apply leaves the current plot/view unchanged. Test `x^2+y^2=1`, `x^2/4+y^2=1` and `x=2` in Auto; verify circles/ellipses/vertical lines. Test `x^2+y^2+z^2=1` automatically uses 3D and forced 2D rejects z without changing the scene. Test `y=x^2` in Auto versus forced 3D. Inspect grid labels and the docked history at 950×760 and 1150×760. Discontinuity gaps are heuristic; do not treat `1/x` or `tan(x)` screenshots as continuity proofs.

## History checks

Automated tests use isolated temporary directories or an in-memory store, never the user's real history. They cover JSON round trips, repeated replacement, the 200-entry limit, clearing, invalid/oversized/future-version input, corrupt-file preservation, unavailable/unwritable storage, UTF-8 truncation, helix/sampling restoration and exactly-once worker success/error recording. Headless drawing covers a history panel containing both outcomes. A layout regression checks that docked history does not overlap the canvas at 950×760 and 1150×760, and hiding it restores canvas width.

Manual protocol for issue #3:

1. Submit a helix edit, a surface request, and an invalid request. Open History; confirm it docks beside the canvas without covering it, resize/hide it, and confirm timestamp/order, success/error labels and useful error text.
2. Change current helix/plot and bounds; Restore the earlier entry. Confirm it restores the original parameters or sampling settings, with no duplicate history row and no change in camera.
3. Change sliders, choose Add current plot, and restore that row. Apply sampling on a surface and confirm it records the successful settings.
4. Choose Edit/retry on an error or success; confirm it fills the input and requires Apply/Enter. The current sampling applies to submitted text; Restore instead uses saved sampling.
5. Restart the app and confirm history remains. Check the displayed storage path. Clear history and restart to verify it stays empty.
6. With the app closed, test a deliberately malformed history file after backing up any useful history. Confirm a visible warning, usable plotting/session history, and unchanged file contents. Restore/move the file and restart for recovery. Do not run this against valuable history without a backup.

Pixel-level/interactive validation remains manual; headless tests do not establish window layout or input ergonomics.

## Automated checks

Run the four commands in README or the VS Code **Verify** task before each meaningful change. rustfmt provides consistent layout; Clippy catches common mistakes; tests protect behavior; build ensures the GUI binary links. CI repeats these checks on Windows x64, Linux x64, and macOS Apple Silicon and builds release executables. Avoid accepting a green geometry test as proof that the GUI works.

## CI packages

`scripts/package.ps1 -Target <triple>` packages the matching `cargo +stable build --locked --release --target <triple>` output. It rejects missing/empty binaries and verifies that the archive contains exactly the executable, RUNNING.md, build-info.json, and (on Windows) the matching math_playground.pdb diagnostic symbols, then generates a SHA-256 checksum. Windows packaging rejects missing symbols. Linux/macOS embed diagnostic symbols in the executable. Unix tar archives retain executable permissions inside the outer Actions ZIP. Run the script with PowerShell 7; supported triples are listed in [download instructions](docs/DOWNLOADS.md).

CI keeps the default debug-profile tests and also runs `cargo +stable test --locked --release --target <triple> --all-targets` on every platform. Release builds retain limited debug information without changing their optimization/debug-assertion/overflow-check settings. Test harnesses do not replace interactive verification of the packaged executable. Formatting runs once; platform-specific lint and both test profiles always run, including after cache restoration. To verify caching, compare a cold run with a rerun of the same PR revision and inspect restore/save logs; cache hits are not proof that tests ran, so check the test steps too.

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

## Coverage reports

CI's **Coverage (Linux debug)** job uses [cargo-llvm-cov 0.9.1](https://github.com/taiki-e/cargo-llvm-cov) with stable Rust's llvm-tools-preview. It runs all automated test targets once with instrumentation on Linux x64 using the default debug profile (including this project's dev opt-level=1). Report generation reuses that run for HTML, LCOV and JSON; it does not run tests three times. Instrumented compilation uses a separate cache because it cannot reuse ordinary compiled objects safely.

The Actions run summary lists line, function and region coverage per source file. Download **coverage-linux-debug**, extract it and open **html/index.html** to inspect uncovered source lines. Reports are retained for 14 days. Dependencies, generated target code and integration-test source are excluded. Inline unit-test code is included in source-file percentages; percentages are a guide to gaps rather than a pure production-code denominator. Stable collection does not enable branch coverage or doctest instrumentation. There currently are no doctests.

Coverage is a separate Linux/debug measurement; the ordinary three-platform debug/release tests still run. No paid service, upload to an external coverage service, percentage threshold, or additional branch-protection rule is introduced. Collection/report failures fail the job and appear in PR checks. Native GUI event loops, real mouse/keyboard behavior, GPU/driver behavior and pixels are not validated by headless drawing coverage. We do not exclude the main/view modules to conceal those gaps or add interaction tests just to raise a score.

To reproduce on a supported development machine:

```sh
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --version 0.9.1 --locked
cargo llvm-cov --locked --all-targets --no-report
cargo llvm-cov report --html --output-dir dist/coverage
```

Start review with uncovered parser/error paths, geometry budgets/domain boundaries, scene failure atomicity, persistence recovery and worker completion. Compare the per-file results and inspect source locations before adding a focused behavioral regression test. GUI ergonomics remain a separate manual decision.

## Shared browser frontend

Run npm ci --ignore-scripts (Node 24+), node --test scripts/*.test.mjs, the documented Rust/Wasm build, then node scripts/verify-web.mjs. New packaging tests verify real module/style references, a single content-addressed Rust/frontend release, shared-CSS/frontend-only cache invalidation and preservation of valid output on size errors. Shared status tests ensure initialization resolution is not renderer readiness and late callbacks cannot clear failures. Real rendering/resizing/egui/history checks remain manual.
