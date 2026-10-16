//! One editorial source for every frontend. Reading it performs no I/O.
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Debug, Deserialize, Serialize)]
pub struct About {
    pub title: String,
    pub name: String,
    pub tagline: String,
    pub summary: String,
    pub principles: Vec<String>,
    pub sections: Vec<Section>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Section {
    pub id: String,
    pub title: String,
    pub paragraphs: Vec<String>,
    pub link: Option<Link>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Link {
    pub label: String,
    pub url: String,
}

// sirno:witness:about-page:begin
/// XcodeGen also bundles this exact file; there are no frontend copies.
pub const CONTENT_JSON: &str = include_str!("../content.json");

static CONTENT: LazyLock<About> = LazyLock::new(|| {
    #[allow(
        clippy::expect_used,
        reason = "embedded editorial resource is validated by tests"
    )]
    serde_json::from_str(CONTENT_JSON).expect("invalid embedded About content")
});

pub fn content() -> &'static About {
    &CONTENT
}

/// Resolve an editorial action, never an arbitrary URL supplied by a webview.
pub fn link(section_id: &str) -> Option<&'static Link> {
    content()
        .sections
        .iter()
        .find(|section| section.id == section_id)
        .and_then(|section| section.link.as_ref())
}
// sirno:witness:about-page:end

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn shared_resource_has_unique_sections_and_only_project_links() {
        let about: About = serde_json::from_str(CONTENT_JSON).unwrap();
        assert!(!about.title.trim().is_empty());
        let mut ids = HashSet::new();
        for section in &about.sections {
            assert!(ids.insert(&section.id), "duplicate section: {}", section.id);
            assert!(!section.title.trim().is_empty());
            assert!(!section.paragraphs.is_empty());
            assert!(
                section
                    .paragraphs
                    .iter()
                    .all(|text| !text.trim().is_empty())
            );
            if let Some(link) = &section.link {
                assert!(!link.label.is_empty());
                let repo = env!("CARGO_PKG_REPOSITORY");
                assert!(link.url == repo || link.url.starts_with(&format!("{repo}/")));
            }
        }
        for id in ["local-first", "architecture", "contributors", "support"] {
            assert!(ids.contains(&id.to_owned()));
        }
    }

    #[test]
    fn action_resolution_rejects_unknown_ids_and_injected_urls() {
        assert!(link("contributors").is_some());
        assert!(link("support").is_some());
        assert!(link("local-first").is_none());
        assert!(link("unknown").is_none());
        assert!(link("https://example.com").is_none());
        assert!(link("file:///tmp/example").is_none());
    }
}
