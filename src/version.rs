//! Identity embedded at compilation, shared by UI, packages and browser metadata.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DISPLAY: &str = env!("MATH_BUILD_DISPLAY");
pub const JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/build-info.json"));

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_identity_matches_package_and_display() {
        let info: serde_json::Value = serde_json::from_str(super::JSON).unwrap();
        assert_eq!(info["version"], super::VERSION);
        assert_eq!(info["display_version"], super::DISPLAY);
        assert!(!info["target"].as_str().unwrap().is_empty());
        assert!(
            info["revision"].as_str().unwrap() == "unknown"
                || info["revision"].as_str().unwrap().len() == 40
        );
    }
}

#[cfg(test)]
#[path = "../build_identity.rs"]
mod policy;
#[cfg(test)]
mod policy_tests {
    use super::policy::{Build, identity};
    #[test]
    fn branches_and_local_builds_are_prereleases_but_clean_main_and_tags_are_stable() {
        let mut build = Build {
            version: "0.2.0",
            revision: "0123456789abcdef",
            reference: "refs/pull/85/merge",
            event: "pull_request",
            run: Some("123"),
            attempt: "2",
            dirty: false,
        };
        assert_eq!(identity(&build).0, "0.2.0-dev.ci.123.2+g0123456789ab");
        build.reference = "refs/heads/main";
        build.event = "push";
        assert_eq!(identity(&build), ("0.2.0".into(), "main"));
        build.dirty = true;
        assert!(identity(&build).0.contains("-dev.ci."));
        build.dirty = false;
        build.run = None;
        assert_eq!(identity(&build).0, "0.2.0-dev.local+g0123456789ab");
        build.revision = "unknown";
        build.dirty = true;
        assert_eq!(identity(&build).0, "0.2.0-dev.local+gunknown.dirty");
        build.reference = "refs/tags/v0.2.0";
        build.dirty = false;
        assert_eq!(identity(&build), ("0.2.0".into(), "release"));
        build.version = "0.3.0-rc.1";
        build.reference = "refs/heads/feature";
        assert!(identity(&build).0.starts_with("0.3.0-rc.1.dev.local+"));
    }
    #[test]
    #[should_panic(expected = "Release tag must match")]
    fn wrong_tag_cannot_produce_an_official_version() {
        identity(&Build {
            version: "0.2.0",
            revision: "unknown",
            reference: "refs/tags/v0.1.0",
            event: "push",
            run: Some("1"),
            attempt: "1",
            dirty: false,
        });
    }
    #[test]
    #[should_panic(expected = "Release checkout must be clean")]
    fn dirty_tag_cannot_produce_an_official_version() {
        identity(&Build {
            version: "0.2.0",
            revision: "unknown",
            reference: "refs/tags/v0.2.0",
            event: "push",
            run: Some("1"),
            attempt: "1",
            dirty: true,
        });
    }
}
