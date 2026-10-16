# Local browser app

The browser build uses the same Rust evaluator, geometry and egui controls as the desktop app. It runs entirely in the browser; the local server only serves static files. No backend or cloud account is needed.

## Build and run

Prerequisites: Rust 1.95 or newer with Cargo/rustup, PowerShell 7 (`pwsh`), Node.js 22 or newer, and a desktop browser with JavaScript, WebAssembly and WebGL enabled. Install Rust's native prerequisites from the main README if also building the desktop app.

From the repository root, run once:

```powershell
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
```

The CLI version must match the pinned `wasm-bindgen` crate. The script checks this before building. An official prebuilt CLI of the same version can also be used through `-Bindgen /path/to/wasm-bindgen`.

Build and serve:

```powershell
pwsh -File scripts/build-web.ps1
node scripts/serve-web.mjs
```

Open **http://127.0.0.1:8080/**. Stop the server with **Ctrl+C** in its terminal. The server binds only to loopback and needs port 8080 free. Opening the HTML through `file://` is unsupported. After changing Rust or HTML, rerun the build script and reload the page; there is no automatic rebuild. Generated files live in ignored `dist/web/`, so a fresh checkout must build first. After dependencies and tooling are installed, running the app requires no internet connection.

## Behavior and limits

The build prepares a matched JS/Wasm pair under `dist/web/assets/<hash>/` and includes Pages hosting headers. `node scripts/verify-web.mjs` checks that artifact. Local serving remains loopback-only; see [DEPLOYMENT.md](DEPLOYMENT.md) for Cloudflare publishing and rollback.

- Typed requests, Apply/Enter, object controls, Apply sampling, camera controls and the existing applicable exploration tools share the native UI. The browser entry selects a dark theme so plot feedback remains legible.
- History is **session-only**: requests, outcomes, timestamps, restore, and clear work in memory. Reloading or closing the page loses it. The history panel states this explicitly. The browser never opens the desktop history file. Persistent browser storage is [#55](https://github.com/Warsinger/math-visualization/issues/55).
- Browser computations run synchronously under the existing parser/sampling budgets, using a candidate scene that replaces the current scene only on success. Expensive plots can briefly pause input and repaint; a computing message may not paint before the calculation finishes. Native calculations still use their background thread. Workers/cancellation and performance guarantees are [#56](https://github.com/Warsinger/math-visualization/issues/56).
- Loading and failure text appears above the canvas. If startup fails or the app stops unexpectedly, inspect the browser's developer **Console** and **Network** panels, confirm JS/Wasm requests succeed, and reload. Startup errors go to the console rather than native log files or dialogs. Wider recovery and browser automation are [#57](https://github.com/Warsinger/math-visualization/issues/57).
- This is a desktop-sized layout (minimum 950 × 790 CSS pixels including the status bar). Small-screen/touch support is [#25](https://github.com/Warsinger/math-visualization/issues/25). Hosting/deployment is [#54](https://github.com/Warsinger/math-visualization/issues/54).

## Initial validation record

Tested on 2026-10-16 using Windows, Chrome 155.0.8059.85 (installed executable version), and a 1847 × 1244 browser viewport. Source: branch `codex/issue-9-local-browser`, initial implementation commit containing this document, based on `88f725a`. The PR records its exact commit SHA. This is the initial tested baseline, not a claim of multi-browser or multi-platform support.

Manual checks through the documented server:

| Check | Observed result |
| --- | --- |
| Startup and helix command | Initial helix renders; `set radius 2` updates the helix and controls. |
| Typed input and dimensions | Apply and Enter work; `x^2` renders a 2D parabola, `x^2+y^2+z^2=1` selects a 3D sphere. |
| Sampling | 2D sample count 64 and X extent 3 apply and record history; 3D cells/axis 20 applies and updates the slice segment count. |
| Camera | 2D pan, 3D orbit/tilt, wheel zoom and Reset view change/restore framing. |
| Point/tangent/integral | Coordinates and residual appear near the curve; tangent near x=1 has slope about 2; `x^2` on [-1,1] shows shaded area and integral approximately 0.666667. |
| Cross-section | Sphere's Z=0 plane and yellow intersection render; camera movement preserves them. |
| Errors | `sqrt(x,2)` reports an arity error and keeps the valid sphere, slice and sampling intact. |
| History | Success/error rows and UTC times appear; restoring a 2D entry from 3D restores its bounds and samples; reload resets history to zero entries. |
| Diagnostics/server | No captured console warnings/errors during these checks. HTML/JS/Wasm return correct MIME types; missing files return 404, POST returns 405, and an encoded parent-path request returns 403. |

Shared debug/release Rust tests, native release build, formatting, and strict native/Wasm Clippy checks pass locally. Browser UI checks are manual; they do not establish exhaustive mathematical parity, sustained responsiveness, mobile behavior, other browsers/operating systems, or automated browser coverage. Native interactive rendering was not repeated for this target adaptation; native headless UI and workflow tests remain in the regression suite.
