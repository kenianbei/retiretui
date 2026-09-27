//! The client's own words for what a page is named and says, so that no
//! page restates them.

use retiretui_client::forms::DOMAINS;
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

/// Every editing domain, in the order the plan lists them.
#[must_use]
pub fn domains() -> Vec<Domain> {
    DOMAINS
        .iter()
        .map(|form| Domain {
            slug: form.title.to_lowercase(),
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
