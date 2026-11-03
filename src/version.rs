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
