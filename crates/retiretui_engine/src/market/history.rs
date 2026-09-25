//! The historical record the historical markets are drawn from.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::plan::ClassReturns;

const EMBEDDED: &str = include_str!("../../market/shiller.toml");

/// One calendar year's nominal returns and inflation.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoricalYear {
    /// The calendar year.
    pub year: i16,
    /// Stocks' total return.
    pub stocks: f64,
    /// Bonds' total return.
    pub bonds: f64,
    /// Cash's return.
    pub cash: f64,
    /// Inflation from its January to the next.
    pub inflation: f64,
}

impl HistoricalYear {
    /// The year's return on each class.
    #[must_use]
    pub fn returns(&self) -> ClassReturns {
        [self.stocks, self.bonds, self.cash]
    }
}

/// Why a historical record was refused.
#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    /// The text is not the record's shape.
    #[error("invalid historical record: {0}")]
    Parse(#[from] toml::de::Error),
    /// The years skip or repeat, or there are none.
    #[error("the historical record's years must run one after another, and there must be some")]
    NotConsecutive,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    years: Vec<HistoricalYear>,
}

/// Consecutive calendar years of returns and inflation.
#[derive(Debug, Clone, PartialEq)]
pub struct History {
    years: Vec<HistoricalYear>,
}

impl History {
    /// The record the engine carries, derived from Shiller's data.
    ///
    /// # Panics
    ///
    /// Never in a built engine: the embedded record is checked by a test.
    #[must_use]
    pub fn embedded() -> &'static Self {
        static EMBEDDED_HISTORY: OnceLock<History> = OnceLock::new();
        EMBEDDED_HISTORY
            .get_or_init(|| Self::from_toml_str(EMBEDDED).expect("the embedded history parses"))
    }

    /// Reads a record of the embedded one's shape.
    ///
    /// # Errors
    ///
    /// [`HistoryError`] when the text does not parse, holds no year, or its
    /// years do not run one after another.
    pub fn from_toml_str(text: &str) -> Result<Self, HistoryError> {
        let Document { years } = toml::from_str(text)?;
        let is_consecutive = years
            .windows(2)
            .all(|pair| pair[1].year == pair[0].year + 1);
        if years.is_empty() || !is_consecutive {
            return Err(HistoryError::NotConsecutive);
        }
        Ok(Self { years })
    }

    /// The first year recorded.
    #[must_use]
    pub fn first_year(&self) -> i16 {
        self.years[0].year
    }

    /// The last year recorded.
    #[must_use]
    pub fn last_year(&self) -> i16 {
        self.years[self.years.len() - 1].year
    }

    /// How many years are recorded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.years.len()
    }

    /// Whether no year is recorded; never, for a record that was read.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.years.is_empty()
    }

    /// The years a history starting in `start` walks over `length` years,
    /// going on from the first year past the last where `wrap`; none where
    /// `start` is not recorded, or the walk passes the last year unwrapped.
    #[must_use]
    pub(crate) fn sequence(
        &self,
        start: i16,
        length: usize,
        wrap: bool,
    ) -> Option<Vec<HistoricalYear>> {
        let at = usize::try_from(start - self.first_year())
            .ok()
            .filter(|&at| at < self.years.len())?;
        if !wrap && at + length > self.years.len() {
            return None;
        }
        Some(
            self.years
                .iter()
                .copied()
                .cycle()
                .skip(at)
                .take(length)
                .collect(),
        )
    }

    /// The years recorded, first to last.
    #[must_use]
    pub fn years(&self) -> &[HistoricalYear] {
        &self.years
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_record_runs_from_1871_to_2025() {
        let history = History::embedded();
        assert_eq!((history.first_year(), history.last_year()), (1871, 2025));
    }

    #[test]
    fn a_sequence_starts_in_its_year_and_wraps_only_when_asked() {
        let history = History::embedded();
        let from_1966 = history.sequence(1966, 3, false).unwrap();
        let years: Vec<i16> = from_1966.iter().map(|year| year.year).collect();
        assert_eq!(years, [1966, 1967, 1968]);
        assert_eq!(from_1966[0], history.years()[1966 - 1871]);
        let wrapped = history.sequence(2024, 3, true).unwrap();
        let years: Vec<i16> = wrapped.iter().map(|year| year.year).collect();
        assert_eq!(years, [2024, 2025, 1871]);
        assert!(history.sequence(2024, 3, false).is_none());
    }

    #[test]
    fn a_record_that_skips_a_year_is_refused() {
        let text = "years = [\n  { year = 2000, stocks = 0.0, bonds = 0.0, cash = 0.0, inflation = 0.0 },\n  { year = 2002, stocks = 0.0, bonds = 0.0, cash = 0.0, inflation = 0.0 },\n]\n";
        assert!(matches!(
            History::from_toml_str(text),
            Err(HistoryError::NotConsecutive)
        ));
    }
}
