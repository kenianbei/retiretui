//! What a payee's words say it was for: a table of substrings matched on
//! the normalized payee, first match wins, the user's rows before the
//! embedded ones.

use serde::Deserialize;

use super::category::Category;

const EMBEDDED: &str = include_str!("patterns.toml");

#[derive(Deserialize)]
struct File {
    patterns: Vec<Pattern>,
}

#[derive(Deserialize)]
struct Pattern {
    r#match: String,
    category: Category,
}

/// The patterns in force, earliest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Patterns {
    rows: Vec<(String, Category)>,
}

impl Patterns {
    /// The embedded table alone.
    ///
    /// # Panics
    ///
    /// Never: the embedded table is asserted to parse by a test.
    #[must_use]
    pub fn embedded() -> Self {
        let mut patterns = Self::default();
        patterns
            .add(EMBEDDED)
            .expect("the embedded pattern table parses");
        patterns
    }

    /// Puts a file's rows before those held, so the user's win.
    ///
    /// # Errors
    ///
    /// The file is not a `[[patterns]]` table of `match` and `category`.
    pub fn add(&mut self, text: &str) -> Result<&mut Self, toml::de::Error> {
        let file: File = toml::from_str(text)?;
        let rows = file
            .patterns
            .into_iter()
            .map(|p| (p.r#match.to_ascii_uppercase(), p.category));
        self.rows.splice(0..0, rows);
        Ok(self)
    }

    /// Puts one payee's category first, as the page assigns it.
    pub fn assign(&mut self, payee: &str, category: Category) {
        self.rows.insert(0, (payee.to_ascii_uppercase(), category));
    }

    /// The category of the first pattern found in the normalized payee.
    #[must_use]
    pub fn category(&self, payee: &str) -> Option<Category> {
        self.rows
            .iter()
            .find(|(pattern, _)| payee.contains(pattern.as_str()))
            .map(|&(_, category)| category)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_table_sorts_the_common_payees_and_the_users_rows_win() {
        let mut patterns = Patterns::embedded();
        assert_eq!(patterns.category("SAFEWAY"), Some(Category::Groceries));
        assert_eq!(patterns.category("UBER EATS"), Some(Category::Dining));
        assert_eq!(patterns.category("UBER TRIP"), Some(Category::Transit));
        assert_eq!(
            patterns.category("ACME CORP PAYROLL"),
            Some(Category::Income)
        );
        assert_eq!(patterns.category("VISA PAYMENT"), Some(Category::Transfer));
        assert_eq!(patterns.category("ZZZ"), None);
        patterns
            .add("[[patterns]]\nmatch = \"safeway\"\ncategory = \"dining\"\n")
            .unwrap();
        assert_eq!(patterns.category("SAFEWAY"), Some(Category::Dining));
        patterns.assign("ZZZ", Category::Pets);
        assert_eq!(patterns.category("ZZZ"), Some(Category::Pets));
        assert!(Patterns::default().add("nonsense = [").is_err());
    }
}
