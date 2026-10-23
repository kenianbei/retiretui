//! What a payee says a row is: the user's rules, the most specific match
//! winning, the merges that gather a payee's variants under one name, and
//! the embedded table of substrings, first match winning.

use serde::Deserialize;

use super::rules::{Merge, MergeText, Outcome, PatternError, Rule, RuleText, best};
use super::{Category, Row};

const EMBEDDED: &str = include_str!("patterns.toml");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    patterns: Vec<RuleText>,
    #[serde(default)]
    payees: Vec<MergeText>,
}

/// The rules and merges in force.
#[derive(Debug, Clone, Default)]
pub struct Patterns {
    user: Vec<Rule>,
    merges: Vec<Merge>,
    embedded: Vec<Rule>,
    text: String,
}

impl Patterns {
    /// The embedded table alone.
    ///
    /// # Panics
    ///
    /// Never: the embedded table is asserted to read by a test.
    #[must_use]
    pub fn embedded() -> Self {
        let (embedded, _) = read(EMBEDDED).expect("the embedded pattern table reads");
        Self {
            embedded,
            ..Self::default()
        }
    }

    /// Puts a file's rules and merges before those held, so the latest
    /// file wins a tie.
    ///
    /// # Errors
    ///
    /// The file is not TOML of `patterns` and `payees`, or an entry tests
    /// nothing, says nothing, or holds a regular expression that does not
    /// compile.
    pub fn add(&mut self, text: &str) -> Result<(), PatternError> {
        let (rules, merges) = read(text)?;
        self.user.splice(0..0, rules);
        self.merges.splice(0..0, merges);
        self.text.insert_str(0, text);
        Ok(())
    }

    /// The text of every file added, the latest first.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The name the payee is merged under, where a merge takes it.
    pub(super) fn merged(&self, payee: &str) -> Option<&str> {
        let found = self.merges.iter().filter(|merge| merge.holds(payee));
        best(found.map(|merge| (merge.score(), merge.name.as_str())))
    }

    /// What the user's rules say of `row`, whose payee reads `payee`.
    pub(super) fn user(&self, payee: &str, row: &Row) -> Option<Outcome> {
        let found = self.user.iter().filter(|rule| rule.holds(payee, row));
        best(found.map(|rule| (rule.score, rule.outcome)))
    }

    /// What the embedded table says of the payee.
    pub(super) fn embedded_outcome(&self, payee: &str, row: &Row) -> Option<Outcome> {
        self.embedded
            .iter()
            .find(|rule| rule.holds(payee, row))
            .map(|rule| rule.outcome)
    }

    /// The category the payee alone is given, by the rules that test
    /// nothing else.
    #[must_use]
    pub fn category(&self, payee: &str) -> Option<Category> {
        let alone = |rule: &&Rule| rule.is_payee_only() && rule.holds_payee(payee);
        let user = self.user.iter().filter(alone);
        let outcome = best(user.map(|rule| (rule.score, rule.outcome)))
            .or_else(|| self.embedded.iter().find(alone).map(|rule| rule.outcome));
        match outcome? {
            Outcome::Category(category) => Some(category),
            Outcome::Ignore => None,
        }
    }
}

/// A file's rules and merges, each entry's complaint naming it.
fn read(text: &str) -> Result<(Vec<Rule>, Vec<Merge>), PatternError> {
    let file: File = toml::from_str(text)?;
    let entry = |table: &'static str, at: usize, why: String| PatternError::Entry {
        table,
        at: at + 1,
        why,
    };
    let rules = (file.patterns.into_iter().enumerate())
        .map(|(at, rule)| rule.rule().map_err(|why| entry("patterns", at, why)))
        .collect::<Result<_, _>>()?;
    let merges = (file.payees.into_iter().enumerate())
        .map(|(at, merge)| merge.merge().map_err(|why| entry("payees", at, why)))
        .collect::<Result<_, _>>()?;
    Ok((rules, merges))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(text: &str) -> Patterns {
        let mut patterns = Patterns::embedded();
        patterns.add(text).unwrap();
        patterns
    }

    #[test]
    fn the_embedded_table_sorts_the_common_payees_and_the_users_rows_win() {
        let patterns = Patterns::embedded();
        assert_eq!(patterns.category("SAFEWAY"), Some(Category::Groceries));
        assert_eq!(patterns.category("UBER EATS"), Some(Category::Dining));
        assert_eq!(patterns.category("UBER TRIP"), Some(Category::Transit));
        assert_eq!(
            patterns.category("ACME CORP PAYROLL"),
            Some(Category::Income)
        );
        assert_eq!(patterns.category("VISA PAYMENT"), Some(Category::Transfer));
        assert_eq!(patterns.category("ZZZ"), None);
        let patterns = with("[[patterns]]\nmatch = \"safeway\"\ncategory = \"dining\"\n");
        assert_eq!(patterns.category("SAFEWAY"), Some(Category::Dining));
        let patterns = with("[[patterns]]\nmatch = \"acme*\"\ncategory = \"pets\"\n");
        assert_eq!(patterns.category("ACME WIDGETS"), Some(Category::Pets));
    }

    #[test]
    fn the_most_specific_rule_wins_and_the_earliest_on_a_tie() {
        let patterns = with(
            r#"patterns = [
  { match = "CAT", category = "pets" },
  { is = "CATAN", category = "entertainment" },
  { match = "CA", category = "gifts" },
]"#,
        );
        assert_eq!(
            patterns.category("CATAN"),
            Some(Category::Entertainment),
            "exact beats contains, wherever it is written"
        );
        assert_eq!(patterns.category("CATS"), Some(Category::Pets), "a tie");
        assert_eq!(patterns.category("CAR"), Some(Category::Gifts));
    }

    #[test]
    fn a_file_that_says_nothing_or_too_much_is_refused_by_entry() {
        let refused = |text: &str| Patterns::default().add(text).unwrap_err().to_string();
        assert!(Patterns::default().add("nonsense = [").is_err());
        assert_eq!(
            refused("[[patterns]]\ncategory = \"pets\"\n"),
            "patterns entry 1: has no condition"
        );
        assert!(
            refused("[[patterns]]\nis = \"X\"\nmatch = \"X\"\ncategory = \"pets\"\n")
                .contains("more than one way")
        );
        assert!(refused("[[patterns]]\nis = \"X\"\n").contains("needs a category"));
        assert!(
            refused("[[patterns]]\nmatches = \"(\"\ncategory = \"pets\"\n")
                .contains("not a regular expression")
        );
        assert!(
            refused("[[patterns]]\nis = \"X\"\ncategory = \"pets\"\ncolour = 1\n")
                .contains("unknown field")
        );
        assert!(refused("[[payees]]\nname = \"X\"\n").contains("payees entry 1: names no payee"));
    }

    #[test]
    fn a_merge_names_its_variants_and_the_most_specific_wins() {
        let patterns = with(
            r#"payees = [
  { name = "Google", match = "GOOGLE" },
  { name = "Google Fi", one_of = ["GOOGLE FI G.CO/HELPPAY#", "GOOGLE FI"] },
]"#,
        );
        assert_eq!(patterns.merged("GOOGLE FI"), Some("Google Fi"));
        assert_eq!(patterns.merged("GOOGLE PLAY"), Some("Google"));
        assert_eq!(patterns.merged("SAFEWAY"), None);
        assert_eq!(patterns.text().lines().next(), Some("payees = ["));
    }
}
