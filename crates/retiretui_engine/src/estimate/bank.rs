//! A bank's export as rows: CSV sniffed from its header, or OFX and QFX,
//! told apart by the file's extension.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, Neg, Sub};

use super::{csv, ofx, payee};
use crate::plan::{Dollars, PlanDate};

const CENTS_PER_DOLLAR: i64 = 100;

/// Money in whole cents, negative where it left the account.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cents(pub i64);

impl Cents {
    /// Whole dollars, rounded half away from zero.
    #[must_use]
    pub const fn to_dollars(self) -> Dollars {
        (self.0 + self.0.signum() * CENTS_PER_DOLLAR / 2) / CENTS_PER_DOLLAR
    }

    /// Reads `12.34`, `-12.3`, `12`, `.5`, `(12.34)`, `$1,234.56`, with
    /// surrounding whitespace; a third decimal rounds.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let (is_negated, text) = match text.strip_prefix('(').and_then(|t| t.strip_suffix(')')) {
            Some(inner) => (true, inner),
            None => (false, text),
        };
        let cleaned: String = text
            .chars()
            .filter(|c| !matches!(c, '$' | ',' | ' '))
            .collect();
        let (is_minus, digits) = match cleaned.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, cleaned.strip_prefix('+').unwrap_or(&cleaned)),
        };
        let (whole, frac) = digits.split_once('.').unwrap_or((digits, ""));
        if whole.is_empty() && frac.is_empty() || !frac.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let whole: i64 = if whole.is_empty() {
            0
        } else {
            whole.parse().ok()?
        };
        let mut frac_digits = frac.chars().map(|c| i64::from(c as u8 - b'0'));
        let tens = frac_digits.next().unwrap_or(0);
        let ones = frac_digits.next().unwrap_or(0);
        let rounds_up = i64::from(frac_digits.next().is_some_and(|d| d >= 5));
        let cents = whole * CENTS_PER_DOLLAR + tens * 10 + ones + rounds_up;
        Some(Self(if is_minus ^ is_negated { -cents } else { cents }))
    }
}

impl fmt::Display for Cents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        let per = CENTS_PER_DOLLAR.unsigned_abs();
        write!(f, "{sign}{}.{:02}", abs / per, abs % per)
    }
}

impl Add for Cents {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl Sub for Cents {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl Neg for Cents {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Sum for Cents {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self(0), Add::add)
    }
}

/// One transaction as the bank wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The date posted.
    pub on: PlanDate,
    /// Signed: negative where money left.
    pub amount: Cents,
    /// The payee text, as written.
    pub payee: String,
    /// The memo, where the file has one.
    pub memo: Option<String>,
    /// The bank's own category, where the file has a column of them.
    pub category: Option<String>,
    /// The bank's id for the transaction, where the file states one.
    pub bank_id: Option<String>,
}

/// Why a file did not read.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BankError {
    /// The extension is none of `csv`, `ofx`, `qfx`.
    #[error("{0}: not a .csv, .ofx or .qfx file")]
    Unknown(String),
    /// A CSV whose header names no date, payee and amount or in/out pair.
    #[error("{name}: cannot tell the columns apart from its first row `{header}`")]
    Unreadable {
        /// The file.
        name: String,
        /// Its first row.
        header: String,
    },
    /// The text is not an OFX or QFX file.
    #[error("{0}: not an OFX file (no <OFX> and no <STMTTRN>)")]
    NotOfx(String),
    /// A row's value did not read as what its column holds.
    #[error("{name} row {row}: `{text}` is not a {expected}")]
    Malformed {
        /// The file.
        name: String,
        /// The row, counted from one, the header included.
        row: usize,
        /// Its text.
        text: String,
        /// What was expected of it.
        expected: &'static str,
    },
}

/// Reads one export by its name's extension.
///
/// # Errors
///
/// Names the file and what did not read.
pub fn read(name: &str, text: &str) -> Result<Vec<Row>, BankError> {
    let extension = name
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase());
    match extension.as_deref() {
        Some("csv") => csv::rows(name, text),
        Some("ofx" | "qfx") => ofx::rows(name, text),
        _ => Err(BankError::Unknown(name.to_owned())),
    }
}

/// Adds a file's rows to those read so far, dropping each row a row
/// already read answers for: the same bank id, else the same date, amount
/// and payee, one row answering for one.
pub fn merge(read: &mut Vec<Row>, rows: Vec<Row>) {
    let mut ids: HashMap<&str, usize> = HashMap::new();
    let mut twins: BTreeMap<(PlanDate, Cents, String), usize> = BTreeMap::new();
    for row in read.iter() {
        match &row.bank_id {
            Some(id) => *ids.entry(id).or_default() += 1,
            None => *twins.entry(twin_key(row)).or_default() += 1,
        }
    }
    let mut kept = Vec::new();
    for row in rows {
        let seen = match &row.bank_id {
            Some(id) => ids.get_mut(id.as_str()),
            None => twins.get_mut(&twin_key(&row)),
        };
        match seen {
            Some(count) if *count > 0 => *count -= 1,
            _ => kept.push(row),
        }
    }
    read.extend(kept);
}

fn twin_key(row: &Row) -> (PlanDate, Cents, String) {
    (row.on, row.amount, payee::normalize(&row.payee))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cents_read_every_bank_spelling() {
        assert_eq!(Cents::parse("$1,234.56"), Some(Cents(123_456)));
        assert_eq!(Cents::parse("(142.17)"), Some(Cents(-14217)));
        assert_eq!(Cents::parse("-60.00"), Some(Cents(-6000)));
        assert_eq!(Cents::parse(".5"), Some(Cents(50)));
        assert_eq!(Cents::parse("12.345"), Some(Cents(1235)));
        assert_eq!(Cents::parse("abc"), None);
        assert_eq!(Cents(-14217).to_dollars(), -142);
        assert_eq!(Cents(150).to_dollars(), 2);
        assert_eq!(Cents(-14217).to_string(), "-142.17");
    }

    #[test]
    fn an_unknown_extension_is_refused_by_name() {
        assert_eq!(
            read("export.xlsx", ""),
            Err(BankError::Unknown("export.xlsx".to_owned()))
        );
    }
}
