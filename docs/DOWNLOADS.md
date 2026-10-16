# Download and run a CI build

Open https://github.com/Warsinger/math-visualization/actions and choose a successful **Rust checks** run for the revision you want. Under **Artifacts**, download the matching target (GitHub sign-in may be required). Builds from pull requests are previews of those changes. Artifacts expire after 14 days; they are CI builds, not signed releases or installers.

| Target / artifact suffix | Platform | Package inside the Actions download |
|---|---|---|
| `x86_64-pc-windows-msvc` | Windows x64 | `.zip` |
| `x86_64-unknown-linux-gnu` | Linux x64, built on Ubuntu 24.04 | `.tar.gz` |
| `aarch64-apple-darwin` | macOS Apple Silicon, built on macOS 15 | `.tar.gz` |

Extract the outer Actions ZIP first. It contains the platform archive and a `.sha256` checksum file. Verify the archive with `Get-FileHash -Algorithm SHA256 <archive>` in PowerShell or `shasum -a 256 <archive>` on macOS (`sha256sum` on Linux), comparing with the supplied checksum. Extract the platform archive, preserving Unix permissions with `tar -xzf <archive>.tar.gz`.

Each inner package contains the `math-playground` executable (`.exe` on Windows), this guide as `RUNNING.md`, and `build-info.json` identifying the target, source commit, and Rust version. Windows also includes the matching `math_playground.pdb` diagnostic symbols; keep it beside the executable when investigating a failure. Linux/macOS retain diagnostic symbols in the executable. No Cargo/Rust compiler is needed to run it. The executable is not an installer; launch it from the extracted directory:

- Windows: `.\math-playground.exe` (or double-click). The Microsoft Visual C++ v14 x64 runtime may be required if not installed. Unsigned builds may trigger Windows reputation prompts.
- Linux: `./math-playground`. Requires a graphical X11/Wayland session, OpenGL-capable drivers, and system libraries including X11, Wayland and xkbcommon. On Ubuntu 24.04, runtime packages include `libx11-6 libxi6 libxcursor1 libxrandr2 libxinerama1 libxkbcommon0 libxkbcommon-x11-0 libwayland-client0 libgl1`. Builds target Ubuntu 24.04's libc baseline, not every Linux distribution.
- macOS: `./math-playground` in Terminal. Requires Apple Silicon and a graphical desktop. This is an unsigned, unnotarized command-line executable that opens a window, not a `.app` bundle; macOS may require an explicit user decision to open downloaded software.

A working graphics driver is required on every platform. Automated CI tests include headless UI drawing, not interactive window/driver validation. Report startup or display problems with the target and revision from `build-info.json`.

## Build pipeline

Pushes to main, pull requests, and manual workflow dispatch run formatting once, then strict Clippy, debug-profile tests, release-profile tests, and an optimized release build on each native runner. Branch pushes use the PR event to avoid duplicate builds. Release tests reuse compatible compiled dependencies when building the packaged app, but test harnesses are separate binaries. Debug tests retain debug assertions/overflow checks; release tests exercise optimized behavior. Release builds keep limited debug information for backtrace locations (not full variable inspection), with ordinary release optimization and runtime-check settings unchanged. CI enables `RUST_BACKTRACE=1`.

The Rust cache restores dependency downloads and compiled dependencies per target and compiler, with manifest/lockfile and build-environment keys. Debug/release outputs remain distinct, and compiled files are not shared across platforms. The first run is cold; compatible later runs can reuse the saved cache. Compiler/dependency changes or cache eviction may require rebuilding; no checks are skipped on a cache hit.

Packaging verifies exact archive contents and writes a SHA-256 checksum before uploading. A failed platform does not cancel the other platform jobs; only verified packages are uploaded by that job. This scope covers downloadable CI artifacts. Release publishing, signing, installers, browser/WebAssembly builds, and hosting remain separate work.

The workflow uses checkout v6 and upload-artifact v6, which run on Node.js 24, addressing the previous checkout v4 Node.js 20 deprecation warning.
