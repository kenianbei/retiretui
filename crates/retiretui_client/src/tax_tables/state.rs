//! A state's income tax as sections of a year's tables: its deduction, what
//! it leaves untaxed, and its brackets.

use retiretui_engine::params::{Exclusion, Source, StateParams, TaxParams};
use retiretui_engine::plan::FilingStatus;

use super::{
    ABROAD, BRACKET_COLUMNS, LABELLED_COLUMNS, TaxSection, brackets, labelled, noted, section,
    state_name,
};
use crate::table::money;

pub(super) const NO_INCOME_TAX: &str = "This state has no income tax.";
pub(super) const STATE_TITLE: &str = "State income tax";
const UNTAXED: &str = "Untaxed";
const DEFERRALS: &str = "Retirement contributions";
const HALF_A_YEAR: f64 = 0.5;

pub(super) fn state_sections(
    params: &TaxParams,
    status: FilingStatus,
    state: Option<&str>,
    year: i16,
) -> Vec<TaxSection> {
    let Some(code) = state else {
        let note = format!("The plan lives in {ABROAD} in {year}, so it owes no state tax.");
        return vec![noted(STATE_TITLE, note)];
    };
    let name = state_name(code);
    let Some(table) = params.states.get(code) else {
        return vec![noted(
            STATE_TITLE,
            format!("{name}'s income tax is not modeled."),
        )];
    };
    let title = format!("{name} income tax");
    let state_brackets = table.brackets.for_status(status);
    let bracket_title = format!("{name} brackets");
    let bracketed = if state_brackets.is_empty() {
        noted(&bracket_title, NO_INCOME_TAX.to_owned())
    } else {
        section(&bracket_title, &BRACKET_COLUMNS, brackets(state_brackets))
    };
    let rows = income_rows(table, status);
    vec![section(&title, &LABELLED_COLUMNS, rows), bracketed]
}

fn income_rows(table: &StateParams, status: FilingStatus) -> Vec<Vec<String>> {
    let taxes_benefits = if table.taxes_social_security {
        "Yes"
    } else {
        "No"
    };
    let mut rows = vec![
        labelled("Standard deduction", money(table.deduction.get(status))),
        labelled("Taxes Social Security", taxes_benefits.to_owned()),
    ];
    rows.extend(table.exclusions.iter().flat_map(untaxed_rows));
    if table.taxes_deferrals {
        rows.push(labelled(DEFERRALS, "Taxed when paid in".to_owned()));
    }
    rows
}

/// A row for each source the exclusion leaves untaxed, saying from what age.
fn untaxed_rows(exclusion: &Exclusion) -> impl Iterator<Item = Vec<String>> {
    let untaxed = exclusion.from_age.map_or_else(
        || UNTAXED.to_owned(),
        |age| format!("{UNTAXED} from {}", age_name(age)),
    );
    let sources = exclusion.sources.iter();
    sources.map(move |&source| labelled(source_name(source), untaxed.clone()))
}

/// An age in years as a person says it, a half said in words.
fn age_name(age: f64) -> String {
    let whole = age.trunc();
    if (age - whole - HALF_A_YEAR).abs() < f64::EPSILON {
        format!("{whole} and a half")
    } else {
        age.to_string()
    }
}

/// A source of income as a state's table is read out.
const fn source_name(source: Source) -> &'static str {
    match source {
        Source::Wages => "Wages",
        Source::Pension => "Pensions",
        Source::Distribution => "Retirement withdrawals, not early",
        Source::EarlyDistribution => "Retirement withdrawals, early",
        Source::Conversion => "Roth conversions",
        Source::Other => "Other income",
    }
}

#[cfg(test)]
mod tests {
    use retiretui_engine::params::{Inflation, TaxTables};

    use super::*;

    fn rows(code: &str, year: i16) -> Vec<Vec<String>> {
        let params = TaxTables::embedded().params_for(year, &Inflation::constant(0.0));
        let sections = state_sections(&params, FilingStatus::Single, Some(code), year);
        sections.into_iter().flat_map(|each| each.rows).collect()
    }

    fn said(rows: &[Vec<String>], label: &str) -> Option<String> {
        let found = rows.iter().find(|row| row[0] == label);
        found.map(|row| row[1].clone())
    }

    #[test]
    fn a_state_s_table_says_what_it_leaves_untaxed_and_from_what_age() {
        let pennsylvania = rows("pa", 2026);
        let early = source_name(Source::EarlyDistribution);
        let untaxed = |label| said(&pennsylvania, label);
        assert_eq!(untaxed("Pensions").as_deref(), Some("Untaxed"));
        assert_eq!(untaxed("Roth conversions").as_deref(), Some("Untaxed"));
        assert_eq!(
            untaxed(source_name(Source::Distribution)).as_deref(),
            Some("Untaxed from 59 and a half")
        );
        assert_eq!(untaxed(early), None);
        assert_eq!(untaxed(DEFERRALS).as_deref(), Some("Taxed when paid in"));
        let iowa = rows("ia", 2026);
        assert_eq!(said(&iowa, early).as_deref(), Some("Untaxed from 55"));
        let oregon = rows("or", 2026);
        assert_eq!(said(&oregon, "Pensions"), None);
        assert_eq!(said(&oregon, DEFERRALS), None);
    }

    #[test]
    fn a_bracket_shows_the_rate_the_law_sets_for_the_year() {
        let rate_in = |year| rows("ms", year).last().unwrap()[1].clone();
        assert_eq!(rate_in(2026), "4%");
        assert_eq!(rate_in(2027), "3.75%");
        assert_eq!(rate_in(2031), "3%");
    }
}
