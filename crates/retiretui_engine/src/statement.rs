//! The Social Security statement a person downloads from `ssa.gov` as XML,
//! reduced to what a plan keeps: who it is for, and the covered earnings by
//! year. The estimates it also carries are not read; the engine computes
//! its own.

use std::collections::BTreeMap;

use crate::plan::{Dollars, Issue, Plan, PlanDate};

/// The namespace every statement of the shape read here declares.
const NAMESPACE: &str = "http://ssa.gov/osss/schemas/2.0";
const BIRTH_TAG: &str = "DateOfBirth";
const EARNINGS_TAG: &str = "Earnings";
const FICA_TAG: &str = "FicaEarnings";
const START_ATTRIBUTE: &str = "startYear=\"";
const END_ATTRIBUTE: &str = "endYear=\"";

/// What a plan takes from a statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    /// The date of birth the statement states.
    pub birth: PlanDate,
    /// Covered (FICA) earnings by calendar year, nominal.
    pub earnings: BTreeMap<i16, Dollars>,
}

/// Why a statement could not be read.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StatementError {
    /// The text is not a statement in the schema this build reads.
    #[error("not a Social Security statement (expected the {NAMESPACE} schema)")]
    NotAStatement,
    /// A tag the statement always carries is absent.
    #[error("the statement has no <{0}>")]
    Missing(&'static str),
    /// A value did not read as what its tag holds.
    #[error("<{tag}> holds `{text}`, not a {expected}")]
    Malformed {
        /// The tag.
        tag: &'static str,
        /// Its text.
        text: String,
        /// What was expected of it.
        expected: &'static str,
    },
    /// Several years' earnings stated as one lump, which no record can
    /// place year by year.
    #[error("earnings for {from}-{to} are one lump; enter those years by hand")]
    Grouped {
        /// The first year of the lump.
        from: i16,
        /// The last year of the lump.
        to: i16,
    },
}

/// Reads a statement's XML text.
///
/// # Errors
///
/// Returns a [`StatementError`] when the text is not a statement of the
/// expected schema, lacks its date of birth, holds a value that does not
/// read, or groups several years' earnings into one figure.
pub fn parse(xml: &str) -> Result<Statement, StatementError> {
    if !xml.contains(NAMESPACE) {
        return Err(StatementError::NotAStatement);
    }
    let birth_text = text_of(xml, BIRTH_TAG).ok_or(StatementError::Missing(BIRTH_TAG))?;
    let birth = birth_text
        .parse()
        .map(PlanDate)
        .map_err(|_| malformed(BIRTH_TAG, birth_text, "date"))?;
    let mut earnings = BTreeMap::new();
    let open = format!("<osss:{EARNINGS_TAG} ");
    let close = format!("</osss:{EARNINGS_TAG}>");
    for row in xml.split(&open).skip(1) {
        let row = row.split_once(&close).map_or(row, |(row, _)| row);
        let from = attribute(row, START_ATTRIBUTE)?;
        let to = attribute(row, END_ATTRIBUTE)?;
        if from != to {
            return Err(StatementError::Grouped { from, to });
        }
        let amount_text = text_of(row, FICA_TAG).ok_or(StatementError::Missing(FICA_TAG))?;
        let amount = amount_text
            .parse()
            .map_err(|_| malformed(FICA_TAG, amount_text, "whole number of dollars"))?;
        earnings.insert(from, amount);
    }
    Ok(Statement { birth, earnings })
}

fn text_of<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<osss:{tag}>");
    let after = &xml[xml.find(&open)? + open.len()..];
    after.split('<').next().map(str::trim)
}

fn attribute(row: &str, name: &'static str) -> Result<i16, StatementError> {
    let text = row
        .split(name)
        .nth(1)
        .and_then(|after| after.split('"').next())
        .ok_or(StatementError::Missing(EARNINGS_TAG))?;
    text.parse()
        .map_err(|_| malformed(EARNINGS_TAG, text, "year"))
}

fn malformed(tag: &'static str, text: &str, expected: &'static str) -> StatementError {
    StatementError::Malformed {
        tag,
        text: text.to_owned(),
        expected,
    }
}

impl Plan {
    /// Replaces a person's earnings record with the statement's.
    ///
    /// # Errors
    ///
    /// Returns an [`Issue`] naming the person when no such person exists,
    /// or their `birth` when it is not the date the statement states.
    pub fn adopt_earnings(&mut self, person: &str, statement: &Statement) -> Result<(), Issue> {
        let Some(index) = self
            .household
            .people
            .iter()
            .position(|candidate| candidate.id == person)
        else {
            return Err(Issue {
                path: "household.people".into(),
                message: format!("unknown person `{person}`"),
            });
        };
        let holder = &mut self.household.people[index];
        if holder.birth != statement.birth {
            return Err(Issue {
                path: format!("household.people[{index}].birth"),
                message: format!(
                    "the statement is for someone born {}; `{person}` was born {}",
                    statement.birth.0, holder.birth.0
                ),
            });
        }
        holder.earnings.clone_from(&statement.earnings);
        Ok(())
    }
}
