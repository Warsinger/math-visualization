# Mathematical playground

A Rust desktop explorer for helices and numerical implicit surfaces. End users edit through typed requests, sliders, and numeric inputs. Start with [SPEC.md](SPEC.md) for the project scope and learning roadmap, and [TESTING.md](TESTING.md) for regression checks.

Track features for later work in [BACKLOG.md](BACKLOG.md).

## Windows setup

1. Install [official Rust via rustup](https://rust-lang.org/tools/install/) with the default stable MSVC toolchain. This project needs Rust 1.95 or newer. Rustup supplies Cargo; select the default profile for rustfmt and Clippy.
2. Install [Visual Studio Build Tools](https://rust-lang.github.io/rustup/installation/windows-msvc.html) with **Desktop development with C++**, MSVC x64/x86 tools, and a Windows SDK. Rust uses the Microsoft linker even though this app contains Rust source only.
3. Restart VS Code and terminals after installation so they see the updated PATH. Verify `rustc --version`, `cargo --version`, and `rustup show`.
4. Open `D:\eelwe\dev\3dviz` in VS Code. Install the recommended rust-analyzer extension; CodeLLDB is optional for debugging. No extension is installed by this project automatically.

In the project terminal:

```powershell
cargo run
```

The first build downloads dependencies and takes longer. Normal app use needs no service, credentials, or internet. A functioning graphics driver with OpenGL support is required by the Glow backend.

## Explore

- Apply `create a helix with five turns` or `set radius to 2 and height to 6`.
- Try `set pitch to 0.5 and turns to 8`.
- Adjust Radius, Height, and Turns; click their numeric values for precise entry.
- Drag the canvas to rotate, scroll over it to zoom, or use camera sliders.
- Reset view restores camera framing.

The parser is intentionally limited to the documented grammar in SPEC.md. `create` replaces the one helix with default unspecified values; `set` preserves unspecified values. Total height stays fixed when turns change. Unsupported requests report errors without altering geometry. Taper, animation, and broader language are planned later.

## Plot an equation

Apply `plot x^2 - y^2 - z^2 = 0` to plot the double cone along X, or try `plot x^2+y^2+z^2=4` (sphere), `plot z-x*y` (saddle), or `plot x=0.3` (plane). These use a general arithmetic evaluator, not recognized shape templates. `plot F` means `F=0`; `plot left=right` evaluates `left-right=0`.

Use lowercase Cartesian variables `x`, `y`, `z` (at most three), finite decimal/scientific numbers, `+ - * / ^`, unary signs, and parentheses. Multiplication must be explicit (`2*x`). Powers associate right: `x^2^3` means `x^(2^3)`; `-x^2` means `-(x^2)`. No functions, constants such as pi, polar coordinates, inequalities, equation systems, or executable code. Expressions are limited to 256 bytes and 64 parsing levels; constant-only equations are rejected.

Surface controls set symmetric domain half-extents independently for X/Y/Z (0.1–10) and grid resolution (8–40 cells per axis, default 24). Click numbers for precise input, then **Apply sampling**. A new equation reuses the active surface's applied sampling settings. `create a helix with five turns` returns to the helix; helix `set` commands are rejected while a surface is selected. Camera behavior is shared.

Marching tetrahedra approximates real zero crossings on the finite sampled domain. It is **not an exact real-solution solver**: thin features, singularities, tangencies/even-multiplicity roots (for example `x^2=0`) may be missed. Discontinuities can produce false surfaces (for example `1/(x-0.13)=0`). Undefined/non-real/nonfinite samples cause adjoining tetrahedra to be skipped and are counted in feedback. Empty geometry does not prove no real solutions exist. A single equation generally describes a surface, not an arbitrary space curve.

Rendering is an orthographic triangle-edge wireframe with no depth occlusion, lighting, or hidden-line removal. Surface calculation runs on one background worker, with editing disabled until it finishes; camera interaction remains available. Geometry is cached until an applied edit. Work is bounded to 68,921 grid evaluations, 384,000 tetrahedra and 120,000 output edges; complexity over the edge cap is rejected without replacing the prior scene. These are work limits, not a fixed elapsed-time guarantee on every computer. Repeated/shared triangle edges are currently retained.

Formula typography remains pending in BACKLOG.md; equations currently appear as plain text.

## Review and verify

Use Terminal → Run Task → **Verify** in VS Code, or:

```powershell
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --locked
```

If starting from a copy without Cargo.lock, run `cargo build` once to generate it before using `--locked`, and keep that file with the application. CI is configured in `.github/workflows/ci.yml` for a future GitHub repository. It does not run until the project is pushed there. No repository commit/push is performed by setup.

## Setup validation status

Verified on October 14, 2026:

- Installed official stable Rust 1.99.0 (MSVC), Cargo, rustfmt, and Clippy in the user profile through rustup.
- Installed Microsoft Visual Studio 2022 Build Tools with the C++ workload and recommended SDK prerequisites.
- `cargo check --all-targets`, formatting check, strict Clippy, all 9 unit/integration tests, and a full Windows build passed.
- The executable remained running during a four-second startup smoke check. This establishes startup, not visual correctness or interaction quality.
- Cargo.lock was generated and is included. CI configuration exists but has not run remotely.
- Manual canvas/layout/input verification and automated screenshot checks remain outstanding; use TESTING.md.

Restart VS Code to pick up the new Cargo PATH. During setup checks, Cargo was invoked via `%USERPROFILE%\.cargo\bin\cargo.exe` because the already-open shell had its old PATH.

Equation extension validation: formatting check, strict Clippy, all **20 tests**, and the Windows build passed on October 14, 2026. Tests include worker completion/failure and headless drawing for helix and surface scenes. The equation-enabled executable also passed a four-second startup smoke check. Manual visual/interaction review and screenshot comparison remain pending. No dependency changes were needed. The original helix baseline is commit `66fbd78`; the next checkpoint records equation plotting and the project backlog before mathematical functions are added.
