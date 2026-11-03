//! Pure version-label policy shared by the build script and regression tests.
pub struct Build<'a> {
    pub version: &'a str,
    pub revision: &'a str,
    pub reference: &'a str,
    pub event: &'a str,
    pub run: Option<&'a str>,
    pub attempt: &'a str,
    pub dirty: bool,
}
pub fn identity(build: &Build<'_>) -> (String, &'static str) {
    if build.reference.starts_with("refs/tags/") {
        assert_eq!(
            build.reference,
            format!("refs/tags/v{}", build.version),
            "Release tag must match Cargo.toml"
        );
        assert!(!build.dirty, "Release checkout must be clean");
        return (
            build.version.to_owned(),
            if build.version.contains('-') {
                "prerelease"
            } else {
                "release"
            },
        );
    }
    if build.reference == "refs/heads/main"
        && build.run.is_some()
        && !build.dirty
        && matches!(build.event, "push" | "workflow_dispatch")
    {
        return (build.version.to_owned(), "main");
    }
    let short = &build.revision[..build.revision.len().min(12)];
    let separator = if build.version.contains('-') {
        "."
    } else {
        "-"
    };
    let (suffix, channel) = match build.run {
        Some(run) => (format!("dev.ci.{run}.{}", build.attempt), "ci"),
        None => ("dev.local".to_owned(), "local"),
    };
    (
        format!(
            "{}{separator}{suffix}+g{short}{}",
            build.version,
            if build.dirty { ".dirty" } else { "" }
        ),
        channel,
    )
}
