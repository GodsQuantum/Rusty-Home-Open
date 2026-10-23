use crate::official_store::{App, Catalog};

#[derive(Debug, Default, Clone, Copy)]
pub struct Filter<'a> {
    pub text: Option<&'a str>,
    pub repository: Option<&'a str>,
    pub installed: Option<bool>,
    pub updates_only: bool,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FilterError {
    InvalidLimit,
    InvalidQuery,
}

#[derive(serde::Serialize)]
pub struct Page<'a> {
    pub total: usize,
    pub apps: Vec<&'a App>,
    pub next_offset: Option<usize>,
}

impl Catalog {
    pub fn filter_apps(&self, q: Filter<'_>) -> Result<Page<'_>, FilterError> {
        if !(1..=200).contains(&q.limit) {
            return Err(FilterError::InvalidLimit);
        }
        for field in [q.text, q.repository].into_iter().flatten() {
            if field.len() > 256 || field.chars().any(char::is_control) {
                return Err(FilterError::InvalidQuery);
            }
        }
        let text = q.text.unwrap_or("").trim().to_lowercase();
        let mut found: Vec<&App> = self
            .apps
            .iter()
            .filter(|app| {
                q.repository
                    .is_none_or(|r| app.repository.eq_ignore_ascii_case(r))
                    && q.installed.is_none_or(|i| app.installed == i)
                    && (!q.updates_only || app.update_available)
                    && (text.is_empty()
                        || app.name.to_lowercase().contains(&text)
                        || app.slug.to_lowercase().contains(&text))
            })
            .collect();
        found.sort_unstable_by(|a, b| a.slug.cmp(&b.slug));
        let total = found.len();
        if q.offset >= total {
            return Ok(Page {
                total,
                apps: Vec::new(),
                next_offset: None,
            });
        }
        let end = q.offset.saturating_add(q.limit).min(total);
        Ok(Page {
            total,
            apps: found[q.offset..end].to_vec(),
            next_offset: if end < total { Some(end) } else { None },
        })
    }
}
