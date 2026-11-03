mod build_identity;
use std::{env, fs, path::PathBuf, process::Command};
fn git(args: &[&str]) -> Option<String> {
    let result = Command::new("git")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(args)
        .output()
        .ok()?;
    result
        .status
        .success()
        .then(|| String::from_utf8_lossy(&result.stdout).trim().to_owned())
}
fn main() {
    for path in [
        "src",
        "web",
        "scripts",
        "Cargo.toml",
        "Cargo.lock",
        "build.rs",
        "build_identity.rs",
        "assets/icon/app.ico",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    // Match invalidation to the tracked-file dirty-state contract. Keep the
    // fixed watches above as a no-Git/source-archive fallback.
    if let Some(paths) = git(&["ls-files", "-z"]) {
        for path in paths.split('\0').filter(|p| !p.is_empty()) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    for variable in [
        "GITHUB_REF",
        "GITHUB_RUN_ID",
        "GITHUB_RUN_ATTEMPT",
        "GITHUB_EVENT_NAME",
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
    }
    for path in ["HEAD", "index", "packed-refs", "refs"] {
        if let Some(path) = git(&["rev-parse", "--git-path", path])
            && std::path::Path::new(&path).exists()
        {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    if let Some(reference) = git(&["symbolic-ref", "-q", "HEAD"])
        && let Some(path) = git(&["rev-parse", "--git-path", &reference])
        && std::path::Path::new(&path).exists()
    {
        println!("cargo:rerun-if-changed={path}");
    }
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    assert!(
        !version.contains('+'),
        "Cargo version must not contain build metadata"
    );
    for part in ["MAJOR", "MINOR", "PATCH"] {
        assert!(
            env::var(format!("CARGO_PKG_VERSION_{part}"))
                .unwrap()
                .parse::<u64>()
                .unwrap()
                <= 65535,
            "Version components must fit Windows fields"
        );
    }
    let revision = git(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let dirty =
        git(&["status", "--porcelain", "--untracked-files=no"]).is_some_and(|s| !s.is_empty());
    let reference = env::var("GITHUB_REF").unwrap_or_default();
    let event = env::var("GITHUB_EVENT_NAME").unwrap_or_default();
    let run = env::var("GITHUB_RUN_ID").ok();
    let attempt = env::var("GITHUB_RUN_ATTEMPT").unwrap_or_else(|_| "1".into());
    let (display, channel) = build_identity::identity(&build_identity::Build {
        version: &version,
        revision: &revision,
        reference: &reference,
        event: &event,
        run: run.as_deref(),
        attempt: &attempt,
        dirty,
    });
    let target = env::var("TARGET").unwrap();
    let profile = env::var("PROFILE").unwrap();
    let rust = Command::new(env::var("RUSTC").unwrap_or_else(|_| "rustc".into()))
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_else(|| "unknown".into());
    let info = serde_json::json!({"version":version,"display_version":display,"revision":revision,"dirty":dirty,"channel":channel,"target":target,"profile":profile,"rust":rust});
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("build-info.json");
    fs::write(&output, serde_json::to_string(&info).unwrap()).unwrap();
    println!("cargo:rustc-env=MATH_BUILD_DISPLAY={display}");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/icon/app.ico")
            .set("ProductName", "Mathematical playground")
            .set("FileDescription", "Mathematical playground")
            .set("ProductVersion", &display)
            .set("FileVersion", &display)
            .compile()
            .expect("compile Windows application resources");
    }
}
