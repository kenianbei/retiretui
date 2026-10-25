//! What a payee says a row is: the user's rules, the most specific match
//! winning, the merges that gather a payee's variants under one name, and
//! the embedded table of substrings, first match winning.

use std::sync::{Arc, LazyLock};

use serde::Deserialize;

use super::rules::{Merge, MergeText, PatternError, Rule, RuleText, best};
use super::{Row, normalize};

const EMBEDDED: &str = include_str!("patterns.toml");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    patterns: Vec<RuleText>,
    #[serde(default)]
    payees: Vec<MergeText>,
}

/// The embedded table, read once.
static EMBEDDED_RULES: LazyLock<Arc<[Rule]>> = LazyLock::new(|| {
    let (rules, _) = read(EMBEDDED, None).expect("the embedded pattern table reads");
    rules.into()
});

/// The rules and merges in force.
#[derive(Debug, Clone, Default)]
pub struct Patterns {
    user: Vec<Rule>,
    merges: Vec<Merge>,
    embedded: Arc<[Rule]>,
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
        Self {
            embedded: Arc::clone(&EMBEDDED_RULES),
            ..Self::default()
        }
    }

    /// Puts the rules and merges of the file named `file` before those
    /// held, so the latest file wins a tie.
    ///
    /// # Errors
    ///
    /// The file is not TOML of `patterns` and `payees`, or an entry tests
    /// nothing, says nothing, or holds a regular expression that does not
    /// compile.
    pub fn add(&mut self, file: &str, text: &str) -> Result<(), PatternError> {
        let (rules, merges) = read(text, Some(file))?;
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

    /// The name a payee as the bank wrote it is listed under: merged,
    /// else normalized.
    #[must_use]
    pub fn name_of(&self, payee: &str) -> String {
        let payee = normalize(payee);
        self.merged(&payee).map_or(payee, str::to_owned)
    }

    /// The name the payee is merged under, where a merge takes it.
    pub(super) fn merged(&self, payee: &str) -> Option<&str> {
        let found = self.merges.iter().filter(|merge| merge.holds(payee));
        best(found.map(|merge| (merge.score(), merge.name.as_str())))
    }

    /// The user's rule that says what `row`, whose payee reads `payee`, is.
    pub(super) fn user(&self, payee: &str, row: &Row) -> Option<&Rule> {
        let found = self.user.iter().filter(|rule| rule.holds(payee, row));
        best(found.map(|rule| (rule.score, rule)))
    }

    /// The embedded table's rule that says what the payee is.
    pub(super) fn embedded_rule(&self, payee: &str, row: &Row) -> Option<&Rule> {
        self.embedded.iter().find(|rule| rule.holds(payee, row))
    }
}

/// A file's rules and merges, each entry's complaint naming it; `name`
/// names it where it is the user's.
fn read(text: &str, name: Option<&str>) -> Result<(Vec<Rule>, Vec<Merge>), PatternError> {
    let file: File = toml::from_str(text)?;
    let entry = |table: &'static str, at: usize, why: String| PatternError::Entry {
        table,
        at: at + 1,
        why,
    };
    let rules = (file.patterns.into_iter().enumerate())
        .map(|(at, rule)| (rule.rule(name, at + 1)).map_err(|why| entry("patterns", at, why)))
        .collect::<Result<_, _>>()?;
    let merges = (file.payees.into_iter().enumerate())
        .map(|(at, merge)| merge.merge().map_err(|why| entry("payees", at, why)))
        .collect::<Result<_, _>>()?;
    Ok((rules, merges))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::estimate::{Category, Cents, Outcome};
    use crate::plan::PlanDate;

    fn with(text: &str) -> Patterns {
        let mut patterns = Patterns::embedded();
        patterns.add("test.toml", text).unwrap();
        patterns
    }

    /// What `patterns` make of a row of `payee`, as the estimate sorts it.
    fn sorted(patterns: &Patterns, payee: &str) -> Option<Category> {
        let row = Row {
            on: PlanDate(jiff::civil::date(2026, 10, 1)),
            amount: Cents(-100),
            payee: payee.to_owned(),
            category: None,
            kind: None,
            bank_id: None,
            export: String::new(),
        };
        let rule = (patterns.user(payee, &row)).or_else(|| patterns.embedded_rule(payee, &row));
        match rule?.outcome {
            Outcome::Category(category) => Some(category),
            Outcome::Ignore | Outcome::Bucket => None,
        }
    }

    #[test]
    fn the_embedded_table_sorts_the_common_payees_and_the_users_rows_win() {
        let patterns = Patterns::embedded();
        assert_eq!(sorted(&patterns, "SAFEWAY"), Some(Category::Groceries));
        assert_eq!(sorted(&patterns, "UBER EATS"), Some(Category::Dining));
        assert_eq!(sorted(&patterns, "UBER TRIP"), Some(Category::Transit));
        assert_eq!(
            sorted(&patterns, "ACME CORP PAYROLL"),
            Some(Category::Income)
        );
        assert_eq!(sorted(&patterns, "VISA PAYMENT"), Some(Category::Transfer));
        assert_eq!(sorted(&patterns, "ZZZ"), None);
        let patterns = with("[[patterns]]\nmatch = \"safeway\"\ncategory = \"dining\"\n");
        assert_eq!(sorted(&patterns, "SAFEWAY"), Some(Category::Dining));
        let patterns = with("[[patterns]]\nmatch = \"acme*\"\ncategory = \"pets\"\n");
        assert_eq!(sorted(&patterns, "ACME WIDGETS"), Some(Category::Pets));
    }

    #[test]
    fn a_space_at_either_end_of_a_match_is_the_edge_of_a_word() {
        let patterns = Patterns::embedded();
        assert_eq!(sorted(&patterns, "ATT WIRELESS"), Some(Category::Phone));
        assert_eq!(sorted(&patterns, "ATT"), Some(Category::Phone));
        assert_eq!(sorted(&patterns, "SEATTLE LOCKSMITH"), None);
        assert_eq!(
            sorted(&patterns, "VSP VISION CARE"),
            Some(Category::Medical)
        );
        assert_eq!(sorted(&patterns, "VISION WORKS"), Some(Category::Medical));
        assert_eq!(sorted(&patterns, "DIVISION LOCKSMITH"), None);
        assert_eq!(sorted(&patterns, "XCEL ENERGY"), Some(Category::Utilities));
        assert_eq!(sorted(&patterns, "T-MOBILE"), Some(Category::Phone));
        assert_eq!(sorted(&patterns, "MOBIL 0412"), Some(Category::Fuel));
        assert_eq!(sorted(&patterns, "SHELLFISH SHACK"), None);
        assert_eq!(sorted(&patterns, "TRAVIS COUNTY"), None);
        assert_eq!(sorted(&patterns, "EXCELSIOR LOCKSMITH"), None);
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
            sorted(&patterns, "CATAN"),
            Some(Category::Entertainment),
            "exact beats contains, wherever it is written"
        );
        assert_eq!(sorted(&patterns, "CATS"), Some(Category::Pets), "a tie");
        assert_eq!(sorted(&patterns, "CAR"), Some(Category::Gifts));
    }

    #[test]
    fn a_file_that_says_nothing_or_too_much_is_refused_by_entry() {
        let refused =
            |text: &str| (Patterns::default().add("test.toml", text).unwrap_err()).to_string();
        assert!(
            Patterns::default()
                .add("test.toml", "nonsense = [")
                .is_err()
        );
        assert_eq!(
            refused("[[patterns]]\ncategory = \"pets\"\n"),
            "patterns entry 1: has no condition"
        );
        assert!(
            refused("[[patterns]]\nis = \"X\"\nmatch = \"X\"\ncategory = \"pets\"\n")
                .contains("more than one way")
        );
        assert!(refused("[[patterns]]\nis = \"X\"\n").contains("needs one of"));
        assert!(
            refused("[[patterns]]\nmatches = \"(\"\ncategory = \"pets\"\n")
                .contains("not a regular expression")
        );
        assert!(
            refused("[[patterns]]\nis = \"X\"\ncategory = \"pets\"\ncolour = 1\n")
                .contains("unknown field")
        );
        assert!(refused("[[payees]]\nname = \"X\"\n").contains("payees entry 1: names no payee"));
        assert!(
            refused("[[patterns]]\nmatch = \"*\"\ncategory = \"pets\"\n").contains("no words"),
            "a match of nothing would hold every payee"
        );
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
