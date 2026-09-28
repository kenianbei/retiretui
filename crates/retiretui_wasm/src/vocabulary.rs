//! The client's own words for what a page is named and says, so that no
//! page restates them.

use retiretui_client::forms::{DOMAINS, DomainId, Form};
use serde::Serialize;

/// One of the plan's editing domains, as its page is named.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Domain {
    /// Its page's address: its title, lower-cased.
    pub slug: String,
    /// What a heading calls it.
    pub title: &'static str,
    /// What it holds, where it holds many items.
    pub purpose: Option<&'static str>,
}

/// A domain's page address: its title, lower-cased.
#[must_use]
pub fn slug_of(domain: DomainId) -> String {
    domain.title().to_lowercase()
}

/// The form whose page is at `slug`.
///
/// # Errors
///
/// Where no domain's page is at `slug`.
pub fn form_at(slug: &str) -> Result<&'static Form, String> {
    let is_at = |form: &&Form| form.domain.is_some_and(|domain| slug_of(domain) == slug);
    DOMAINS
        .iter()
        .find(is_at)
        .ok_or_else(|| format!("no domain is at {slug}"))
}

/// Every editing domain, in the order the plan lists them.
#[must_use]
pub fn domains() -> Vec<Domain> {
    DOMAINS
        .iter()
        .map(|form| Domain {
            slug: form.domain.map(slug_of).unwrap_or_default(),
            title: form.title,
            purpose: form.list.map(|list| list.purpose),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_domain_has_a_page_of_its_own() {
        let domains = domains();
        assert_eq!(domains.len(), DOMAINS.len());
        let accounts = &domains[0];
        assert_eq!(
            (accounts.slug.as_str(), accounts.title),
            ("accounts", "Accounts")
        );
        assert!(accounts.purpose.is_some());
        let mut slugs: Vec<_> = domains.iter().map(|domain| domain.slug.as_str()).collect();
        slugs.dedup();
        assert_eq!(slugs.len(), DOMAINS.len());
        assert!(
            slugs
                .iter()
                .all(|slug| slug.chars().all(|c| c.is_ascii_lowercase()))
        );
    }
}
