//! Build-information types and tools; applications own their compiled values.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfo {
    pub version: String,
    pub built_at_utc: String,
}

impl From<&build_info::BuildInfo> for BuildInfo {
    fn from(info: &build_info::BuildInfo) -> Self {
        Self {
            version: info.crate_info.version.to_string(),
            built_at_utc: info
                .timestamp
                .to_rfc3339_opts(build_info::chrono::SecondsFormat::Secs, true),
        }
    }
}

impl std::fmt::Display for BuildInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (built {})", self.version, self.built_at_utc)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn build_info_preserves_utc_timestamp_through_serialization() {
        let info = super::BuildInfo {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            built_at_utc: "2026-10-09T21:45:00Z".to_owned(),
        };
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        let time = build_info::chrono::DateTime::parse_from_rfc3339(&info.built_at_utc).unwrap();
        assert_eq!(time.offset().local_minus_utc(), 0);
        assert!(info.built_at_utc.ends_with('Z'));
        let encoded = serde_json::to_string(&info).unwrap();
        assert_eq!(
            serde_json::from_str::<super::BuildInfo>(&encoded).unwrap(),
            info
        );
    }
}
