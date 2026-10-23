//! A patterns file's rules and merges: what each tests of a row, what it
//! says the row is, and how specific it is, scored as Actual Budget scores
//! its rules so the most specific match wins.

use regex_lite::Regex;
use serde::{Deserialize, Serialize};

use super::payee::starred_as_spaces;
use super::{Category, Cents, Row};
use crate::plan::PlanDate;

const EXACT_SCORE: u32 = 10;
const ONE_OF_SCORE: u32 = 9;
const SPENT_SCORE: u32 = 5;
const EXPORT_SCORE: u32 = 1;
/// A rule whose every condition is exact counts double.
const EXACT_FACTOR: u32 = 2;

/// Why a patterns file did not read.
#[derive(Debug, thiserror::Error)]
pub enum PatternError {
    /// The file is not TOML of `patterns` and `payees` tables.
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    /// A rule or merge that says nothing it can be matched or acted on by.
    #[error("{table} entry {at}: {why}")]
    Entry {
        /// `patterns` or `payees`.
        table: &'static str,
        /// The entry, counted from one.
        at: usize,
        /// What is wrong with it.
        why: String,
    },
}

/// What a rule says a row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Spending, income or a transfer of this category.
    Category(Category),
    /// Counted nowhere.
    Ignore,
    /// In the bucket, as a payee nothing names is.
    Bucket,
}

/// What decided where a row was sorted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum By {
    /// A rule of the user's.
    Rule {
        /// The name of the file it is in.
        file: String,
        /// Its place among the file's `patterns`, counted from one.
        at: usize,
    },
    /// The bank's own category column.
    Bank,
    /// The bank's transaction type.
    Type,
    /// The embedded table's entry, by the words it tests.
    Embedded(String),
}

/// What a rule tests of the payee, uppercased.
#[derive(Debug, Clone)]
enum PayeeTest {
    Contains(Part),
    Is(String),
    OneOf(Vec<String>),
    Matches(Regex),
    Any,
}

impl PayeeTest {
    fn holds(&self, payee: &str) -> bool {
        match self {
            Self::Contains(part) => part.holds(payee),
            Self::Is(whole) => payee == whole,
            Self::OneOf(wholes) => wholes.iter().any(|whole| whole == payee),
            Self::Matches(pattern) => pattern.is_match(payee),
            Self::Any => true,
        }
    }

    const fn score(&self) -> u32 {
        match self {
            Self::Is(_) => EXACT_SCORE,
            Self::OneOf(_) => ONE_OF_SCORE,
            Self::Contains(_) | Self::Matches(_) | Self::Any => 0,
        }
    }

    const fn is_exact(&self) -> bool {
        matches!(self, Self::Is(_) | Self::OneOf(_))
    }
}

/// A rule: its conditions, all of which hold, and what it says the row is.
#[derive(Debug, Clone)]
pub(super) struct Rule {
    payee: PayeeTest,
    on: Option<PlanDate>,
    spent: Option<(Cents, Cents)>,
    export: Option<String>,
    pub(super) outcome: Outcome,
    pub(super) score: u32,
    /// Where it was written.
    pub(super) by: By,
}

impl Rule {
    /// Whether it holds of `row`, whose payee reads `payee`.
    pub(super) fn holds(&self, payee: &str, row: &Row) -> bool {
        let spent = row.spent();
        self.payee.holds(payee)
            && self.on.is_none_or(|on| on == row.on)
            && self
                .spent
                .is_none_or(|(low, high)| (low..=high).contains(&spent))
            && self
                .export
                .as_ref()
                .is_none_or(|part| row.export.to_ascii_uppercase().contains(part.as_str()))
    }
}

/// Variants of a payee merged under one name.
#[derive(Debug, Clone)]
pub(super) struct Merge {
    payee: PayeeTest,
    pub(super) name: String,
}

impl Merge {
    pub(super) fn holds(&self, payee: &str) -> bool {
        self.payee.holds(payee)
    }

    pub(super) const fn score(&self) -> u32 {
        self.payee.score()
    }
}

/// The highest scoring of `found`, the earliest on a tie.
pub(super) fn best<T>(found: impl Iterator<Item = (u32, T)>) -> Option<T> {
    found
        .min_by_key(|&(score, _)| std::cmp::Reverse(score))
        .map(|(_, it)| it)
}

/// Words as a rule matches them against a payee.
pub(super) fn said(words: &str) -> String {
    starred_as_spaces(&words.trim().to_ascii_uppercase())
}

/// Words a payee contains, a space at either end marking the edge of a
/// word: `ATT ` holds of `ATT WIRELESS` and `ATT`, never `SEATTLE`.
#[derive(Debug, Clone)]
pub(super) struct Part {
    words: String,
    is_left_edge: bool,
    is_right_edge: bool,
}

impl Part {
    pub(super) fn new(text: &str) -> Self {
        Self {
            words: said(text),
            is_left_edge: text.starts_with(' '),
            is_right_edge: text.ends_with(' '),
        }
    }

    pub(super) const fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// Whether the uppercased `payee` contains it, each edge met by a space
    /// or by the payee's own start or end.
    pub(super) fn holds(&self, payee: &str) -> bool {
        payee.match_indices(&self.words).any(|(at, _)| {
            let (before, after) = (&payee[..at], &payee[at + self.words.len()..]);
            (!self.is_left_edge || before.is_empty() || before.ends_with(' '))
                && (!self.is_right_edge || after.is_empty() || after.starts_with(' '))
        })
    }
}

/// The payee keys of a rule or a merge, at most one of them stated.
struct PayeeText {
    r#match: Option<String>,
    is: Option<String>,
    one_of: Option<Vec<String>>,
    matches: Option<String>,
}

impl PayeeText {
    fn test(self) -> Result<PayeeTest, String> {
        let said = |text: String| said(&text);
        let tests = [
            self.r#match
                .map(|text| Ok(PayeeTest::Contains(Part::new(&text)))),
            self.is.map(|text| Ok(PayeeTest::Is(said(text)))),
            self.one_of
                .map(|texts| Ok(PayeeTest::OneOf(texts.into_iter().map(said).collect()))),
            self.matches.map(|text| {
                Regex::new(&format!("(?i){text}"))
                    .map(PayeeTest::Matches)
                    .map_err(|error| format!("`{text}` is not a regular expression: {error}"))
            }),
        ];
        let mut stated = tests.into_iter().flatten();
        let test = stated.next().unwrap_or(Ok(PayeeTest::Any))?;
        if stated.next().is_some() {
            return Err("tests the payee more than one way".to_owned());
        }
        Ok(test)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RuleText {
    r#match: Option<String>,
    is: Option<String>,
    one_of: Option<Vec<String>>,
    matches: Option<String>,
    on: Option<PlanDate>,
    spent: Option<[f64; 2]>,
    export: Option<String>,
    category: Option<Category>,
    #[serde(default)]
    ignore: bool,
    #[serde(default)]
    bucket: bool,
}

impl RuleText {
    /// The rule, written at `at` in `file`, or in the embedded table where
    /// there is none.
    pub(super) fn rule(self, file: Option<&str>, at: usize) -> Result<Rule, String> {
        let outcome = match (self.category, self.ignore, self.bucket) {
            (Some(category), false, false) => Outcome::Category(category),
            (None, true, false) => Outcome::Ignore,
            (None, false, true) => Outcome::Bucket,
            _ => {
                return Err(
                    "needs one of a category, `ignore = true` or `bucket = true`".to_owned(),
                );
            }
        };
        let by = file.map_or_else(
            || By::Embedded(self.r#match.clone().unwrap_or_default()),
            |file| By::Rule {
                file: file.to_owned(),
                at,
            },
        );
        let payee = PayeeText {
            r#match: self.r#match,
            is: self.is,
            one_of: self.one_of,
            matches: self.matches,
        }
        .test()?;
        let spent =
            (self.spent).map(|[low, high]| (Cents::from_dollars(low), Cents::from_dollars(high)));
        let export = self.export.map(|part| part.to_ascii_uppercase());
        if matches!(payee, PayeeTest::Any)
            && self.on.is_none()
            && spent.is_none()
            && export.is_none()
        {
            return Err("has no condition".to_owned());
        }
        let sum = payee.score()
            + self.on.map_or(0, |_| EXACT_SCORE)
            + spent.map_or(0, |_| SPENT_SCORE)
            + export.as_ref().map_or(0, |_| EXPORT_SCORE);
        let is_exact = (payee.is_exact() || matches!(payee, PayeeTest::Any))
            && spent.is_none()
            && export.is_none();
        Ok(Rule {
            payee,
            on: self.on,
            spent,
            export,
            outcome,
            score: if is_exact { sum * EXACT_FACTOR } else { sum },
            by,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MergeText {
    name: String,
    r#match: Option<String>,
    is: Option<String>,
    one_of: Option<Vec<String>>,
    matches: Option<String>,
}

impl MergeText {
    pub(super) fn merge(self) -> Result<Merge, String> {
        let payee = PayeeText {
            r#match: self.r#match,
            is: self.is,
            one_of: self.one_of,
            matches: self.matches,
        };
        match payee.test()? {
            PayeeTest::Any => Err("names no payee".to_owned()),
            payee => Ok(Merge {
                payee,
                name: self.name,
            }),
        }
    }
}
