//! Official Supervisor App changelog as bounded, unrendered plain text.
//! No HTML, scripts, links or secrets are interpreted by the Rusty Home UI.
use crate::{app_detail::valid_slug, official_store::ApiVersion};
use serde::Serialize;

pub const MAX_CHANGELOG_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppChangelog {
    pub slug: String,
    pub text: String,
    pub read_only: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangelogError {
    UnsafeSlug,
    TooLarge,
    InvalidText,
}
pub fn changelog_path(version: ApiVersion, slug: &str) -> Result<String, ChangelogError> {
    if !valid_slug(slug) {
        return Err(ChangelogError::UnsafeSlug);
    }
    Ok(match version {
        ApiVersion::V1 => format!("/store/addons/{slug}/changelog"),
        ApiVersion::V2 => format!("/v2/store/apps/{slug}/changelog"),
    })
}
pub fn parse_changelog(raw: &[u8], slug: &str) -> Result<AppChangelog, ChangelogError> {
    if !valid_slug(slug) {
        return Err(ChangelogError::UnsafeSlug);
    }
    if raw.len() > MAX_CHANGELOG_BYTES {
        return Err(ChangelogError::TooLarge);
    }
    let text = std::str::from_utf8(raw).map_err(|_| ChangelogError::InvalidText)?;
    if text
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err(ChangelogError::InvalidText);
    }
    Ok(AppChangelog {
        slug: slug.to_owned(),
        text: text.to_owned(),
        read_only: true,
    })
}
