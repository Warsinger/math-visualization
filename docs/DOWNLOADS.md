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

Packages also include `app-icon.png`. Windows embeds the application icon in the
executable and supplies it to the window. Linux window icons depend on the
desktop/window system (Wayland may require installed desktop-entry integration).
The macOS archive includes `app-icon.icns` for future bundle integration; the
current raw executable does not acquire a Finder/Dock icon just from that file.
See [icon assets](../assets/icon/README.md) in the source repository for exports.

## Build pipeline

Pushes to main, pull requests, and manual workflow dispatch run formatting once, then strict Clippy, debug-profile tests, release-profile tests, and an optimized release build on each native runner. Branch pushes use the PR event to avoid duplicate builds. Release tests reuse compatible compiled dependencies when building the packaged app, but test harnesses are separate binaries. Debug tests retain debug assertions/overflow checks; release tests exercise optimized behavior. Release builds keep limited debug information for backtrace locations (not full variable inspection), with ordinary release optimization and runtime-check settings unchanged. CI enables `RUST_BACKTRACE=1`.

The Rust cache restores dependency downloads and compiled dependencies per target and compiler, with manifest/lockfile and build-environment keys. Debug/release outputs remain distinct, and compiled files are not shared across platforms. The first run is cold; compatible later runs can reuse the saved cache. Compiler/dependency changes or cache eviction may require rebuilding; no checks are skipped on a cache hit.

Packaging verifies exact archive contents and writes a SHA-256 checksum before uploading. A failed platform does not cancel the other platform jobs; only verified packages are uploaded by that job. This scope covers downloadable CI artifacts. Release publishing, signing, installers, browser/WebAssembly builds, and hosting remain separate work.

The workflow uses checkout v6 and upload-artifact v6, which run on Node.js 24, addressing the previous checkout v4 Node.js 20 deprecation warning.
# Application logs and Windows launch diagnostics

Native builds append structured JSON Lines to the local data directory:
Windows `%LOCALAPPDATA%\MathPlayground\logs\application.jsonl`, Linux
`${XDG_DATA_HOME:-~/.local/share}/MathPlayground/logs/application.jsonl`, and
macOS `~/Library/Application Support/MathPlayground/logs/application.jsonl`.
Each record has an RFC3339 UTC timestamp, severity, target and structured fields
including an `event` name. Startup, normal shutdown, geometry completion/rejection,
worker failure and panics use the same tracing facility. Geometry failures include
the submitted request, error/reason, input mode and sampling context. Request/error
fields are capped at 1024 UTF-8 bytes and marked when truncated. The request is
captured at submission so later input edits cannot misidentify a failure; History
restores use the snapshot's sampling settings. Helix-control failures include the
edited parameters and error. Successful events omit request text; history contents
and panic payloads are not dumped. Startup errors are capped at 4096 UTF-8 bytes.
Logs stay local; share them only deliberately.

The active file rotates at 2 MiB, retaining `application.jsonl.1` and `.2` (about
6 MiB total). A record over 64 KiB is omitted rather than split into invalid JSON.
Each complete event is synchronously appended, flushed and synced; there is no
background queue to lose on ordinary shutdown/startup failure. An OS file lock
coordinates writers/rotation across app instances; contention or unavailable I/O
reports a logging failure to stderr and omits that event without crashing the app.
This cannot guarantee delivery during forced termination or filesystem failure.

App events default to INFO; dependencies default to WARN. Set
`MATH_PLAYGROUND_LOG=trace|debug|info|warn|error|off` to change app verbosity.
Unknown values use INFO. The legacy `startup-error.log` is no longer written;
any existing file is preserved for historical diagnostics and may be deleted manually.

Windows release builds open no console. Startup failure still shows a dialog with
the original error and the general log location or logging failure details.
To verify that path, run `cargo run --release -- --simulate-startup-error` or pass
`--simulate-startup-error` to a packaged executable. It appends `startup_failed`
to the normal log, shows the error dialog, and exits unsuccessfully after dismissal.
Remove the flag to launch normally; history and plot settings are unchanged.
Browser diagnostics continue to use the browser console, without native file logging.

## Remembered native workspace layout

Native builds use eframe's persistence store at
`MathPlayground/workspace-v1.ron` under the OS local-data directory (Windows:
`%LOCALAPPDATA%`; Linux: `$XDG_DATA_HOME` or `~/.local/share`; macOS:
`~/Library/Application Support`). Window position, size, maximized/fullscreen state,
and History visibility/width are restored. History is currently the only optional
resizable side panel. The layout record is versioned and separate from plot history.
Equations, camera, plot settings and exploration toggles are not saved here; general
egui widget memory is intentionally not persisted.

The framework saves changed storage on its 30-second autosave cadence and normal
shutdown, and joins its final file-write thread. Missing/corrupt/incompatible layout
records use defaults. Unavailable/unwritable storage does not prevent startup;
framework I/O diagnostics go to stderr (or the general logger after #46 merges).
Reset by closing the app and deleting only `workspace-v1.ron`; plot history is untouched.

Window restoration uses eframe/winit's monitor and scaling handling (Windows clamps
restored positions to connected monitors; other platforms depend on window-manager
behavior). Nonfinite/extreme geometry is rejected; a first-frame size check fits the
current monitor and relaxes the usual 950×760 minimum on smaller screens. History
width is clamped to leave 600 logical pixels for the main area; below that budget it
is temporarily hidden without changing its saved visibility. Wayland does not expose
window positions and controls placement itself. Restart, disconnected-monitor and
DPI behavior still needs graphical platform testing; the reset procedure is the
fallback for a window manager which fails to relocate an off-screen window.

Browser layout remains session-only; this native feature does not enable localStorage
or persist browser history (#55).
