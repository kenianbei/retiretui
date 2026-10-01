//! A state's tax as sections of a year's tables: its deduction and what it
//! adjusts it by, what it leaves untaxed, its brackets, and its excise on
//! gains.

use retiretui_engine::params::PerStatus;
use retiretui_engine::params::{
    Exclusion, ExemptionCredit, FederalTaxSubtraction, GainsExcise, Source, StateParams, TaxParams,
};
use retiretui_engine::plan::{Dollars, FilingStatus};

use super::{
    ABROAD, BRACKET_COLUMNS, LABELLED_COLUMNS, TaxSection, brackets, labelled, noted, phase_out,
    section, state_name,
};
use crate::table::{money, rate};

pub(super) const NO_INCOME_TAX: &str = "This state has no income tax.";
pub(super) const STATE_TITLE: &str = "State income tax";
const UNTAXED: &str = "Untaxed";
const DEFERRALS: &str = "Retirement contributions";
const DEDUCTION_AT_65: &str = "More deduction for each person from 65";
const DEDUCTION_UNTIL: &str = "No deduction over AGI of";
const SUBTRACTION: &str = "Federal tax subtracted, up to";
const SUBTRACTION_BAND: &str = "Subtraction phase-out (AGI)";
const CREDIT: &str = "Credit for each person";
const CREDIT_UNTIL: &str = "No credit over AGI of";
const CREDIT_AT_65: &str = "More credit for each person from 65";
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
    let mut sections = vec![section(&title, &LABELLED_COLUMNS, rows), bracketed];
    if let Some(excise) = &table.gains_excise {
        let title = format!("{name} excise on long-term gains");
        sections.push(section(&title, &LABELLED_COLUMNS, excise_rows(excise)));
    }
    sections
}

fn income_rows(table: &StateParams, status: FilingStatus) -> Vec<Vec<String>> {
    let taxes_benefits = if table.taxes_social_security {
        "Yes"
    } else {
        "No"
    };
    let of = |amounts: PerStatus<Dollars>| money(amounts.get(status));
    let mut rows = vec![labelled("Standard deduction", of(table.deduction))];
    if table.deduction_at_65.get(status) > 0 {
        rows.push(labelled(DEDUCTION_AT_65, of(table.deduction_at_65)));
    }
    if let Some(limit) = table.deduction_until_agi {
        rows.push(labelled(DEDUCTION_UNTIL, of(limit)));
    }
    if let Some(subtraction) = &table.federal_tax_subtraction {
        rows.extend(subtraction_rows(subtraction, status));
    }
    if let Some(credit) = &table.exemption_credit {
        rows.extend(credit_rows(credit, status));
    }
    rows.push(labelled("Taxes Social Security", taxes_benefits.to_owned()));
    rows.extend(table.exclusions.iter().flat_map(untaxed_rows));
    if table.taxes_deferrals {
        rows.push(labelled(DEFERRALS, "Taxed when paid in".to_owned()));
    }
    rows
}

fn subtraction_rows(subtraction: &FederalTaxSubtraction, status: FilingStatus) -> [Vec<String>; 2] {
    [
        labelled(SUBTRACTION, money(subtraction.cap)),
        labelled(
            SUBTRACTION_BAND,
            phase_out(subtraction.phase_out.get(status)),
        ),
    ]
}

fn credit_rows(credit: &ExemptionCredit, status: FilingStatus) -> Vec<Vec<String>> {
    let mut rows = vec![labelled(CREDIT, money(credit.per_person))];
    if let Some(limit) = credit.until_agi {
        rows.push(labelled(CREDIT_UNTIL, money(limit.get(status))));
    }
    if credit.at_65 > 0 {
        rows.push(labelled(CREDIT_AT_65, money(credit.at_65)));
    }
    rows
}

fn excise_rows(excise: &GainsExcise) -> Vec<Vec<String>> {
    let floors = excise.brackets.iter().map(|bracket| {
        let label = format!("Rate on taxed gains over {}", money(bracket.over));
        labelled(&label, rate(bracket.rate))
    });
    let untaxed = labelled("Gains left untaxed", money(excise.deduction));
    std::iter::once(untaxed).chain(floors).collect()
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
    fn a_state_s_table_says_what_it_adjusts_its_tax_by() {
        let oregon = rows("or", 2026);
        assert_eq!(said(&oregon, DEDUCTION_AT_65).as_deref(), Some("$1,200"));
        assert_eq!(said(&oregon, SUBTRACTION).as_deref(), Some("$8,750"));
        assert_eq!(
            said(&oregon, SUBTRACTION_BAND).as_deref(),
            Some("$125,000 to $145,000")
        );
        assert_eq!(said(&oregon, CREDIT).as_deref(), Some("$263"));
        assert_eq!(said(&oregon, CREDIT_UNTIL).as_deref(), Some("$100,000"));
        assert_eq!(said(&oregon, CREDIT_AT_65), None);
        assert_eq!(said(&oregon, DEDUCTION_UNTIL), None);
        let illinois = rows("il", 2026);
        assert_eq!(
            said(&illinois, DEDUCTION_UNTIL).as_deref(),
            Some("$250,000")
        );
        assert_eq!(said(&illinois, SUBTRACTION), None);
        let iowa = rows("ia", 2026);
        assert_eq!(said(&iowa, CREDIT).as_deref(), Some("$40"));
        assert_eq!(said(&iowa, CREDIT_UNTIL), None);
        assert_eq!(said(&iowa, CREDIT_AT_65).as_deref(), Some("$20"));
        assert_eq!(said(&iowa, DEDUCTION_AT_65), None);
    }

    #[test]
    fn washington_s_excise_is_a_section_beside_its_lack_of_an_income_tax() {
        let params = TaxTables::embedded().params_for(2026, &Inflation::constant(0.0));
        let sections = state_sections(&params, FilingStatus::MarriedJoint, Some("wa"), 2026);
        let [_, brackets, excise] = sections.as_slice() else {
            panic!("{sections:?}");
        };
        assert_eq!(brackets.note.as_deref(), Some(NO_INCOME_TAX));
        assert_eq!(excise.title, "Washington excise on long-term gains");
        let expected = [
            ["Gains left untaxed", "$290,000"],
            ["Rate on taxed gains over $0", "7%"],
            ["Rate on taxed gains over $1,000,000", "9.9%"],
        ];
        assert_eq!(excise.rows, expected);
        let oregon = state_sections(&params, FilingStatus::Single, Some("or"), 2026);
        assert_eq!(oregon.len(), 2);
    }

    #[test]
    fn a_bracket_shows_the_rate_the_law_sets_for_the_year() {
        let rate_in = |year| rows("ms", year).last().unwrap()[1].clone();
        assert_eq!(rate_in(2026), "4%");
        assert_eq!(rate_in(2027), "3.75%");
        assert_eq!(rate_in(2031), "3%");
    }
}
