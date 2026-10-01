//! A year's tax tables as a plan's projection applies them: the federal
//! brackets, deductions, limits and surcharges for a filing status, the
//! benefit formula's amounts, and a state's income tax, each a section of
//! rows in the words every other table is said in.

use retiretui_engine::params::{
    BenefitParams, Bracket, Inflation, IrmaaTier, PerStatus, PhaseOut, RmdDivisor, TaxParams,
    TaxTables,
};
use retiretui_engine::plan::{Dollars, FilingStatus, Plan, US_STATES, place_name};
use retiretui_engine::project::{benefit_params, state_lived_in};
use serde::{Deserialize, Serialize};

use crate::forms::offers::{Offer, Vocabulary};
use crate::present::filing_status;
use crate::table::{money, rate};

/// Which tables are asked for: a year, and a status and state other than
/// the plan's where one is named.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TablesView {
    /// The year the tables are for.
    pub year: i16,
    /// A filing status as the plan file spells it; the plan's where none.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub status: Option<String>,
    /// A state's lowercase code; the one the plan lives in that year where
    /// none.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub state: Option<String>,
}

/// One table of the year's: a title, its columns and its rows, or a note
/// saying why it has none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TaxSection {
    /// What the table holds.
    pub title: String,
    /// Its column headers.
    pub columns: Vec<&'static str>,
    /// Its rows, each a cell per column.
    pub rows: Vec<Vec<String>>,
    /// Why it has no rows, where it has none.
    pub note: Option<String>,
}

/// What the filing status is picked under.
pub const STATUS_PICK: &str = "Filing status";
/// What the state is picked under.
pub const STATE_PICK: &str = "State";

/// What the Tax Tables page is for, said once above its tables.
const ABOUT: &str = "The amounts the projection applies in a year; past the latest table, they grow at the plan's inflation.";

/// A year's tables for a status and a state, and the others that can be
/// asked for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct YearTables {
    /// The year they are for.
    pub year: i16,
    /// The filing status shown, as the view names it.
    pub status: String,
    /// The filing status shown, as a person says it.
    pub status_name: &'static str,
    /// The state shown, where there is one.
    pub state: Option<String>,
    /// The state shown, by its name.
    pub state_name: Option<String>,
    /// Every filing status.
    pub statuses: Vec<Offer>,
    /// Every state the year's tables model.
    pub states: Vec<Offer>,
    /// The tables, federal first.
    pub sections: Vec<TaxSection>,
    /// What the tables are, said once above them.
    pub about: &'static str,
    /// What the filing status is picked under.
    pub status_pick: &'static str,
    /// What the state is picked under.
    pub state_pick: &'static str,
    /// The plan's own filing status, as a pick offers it.
    pub own_status: String,
    /// Where the plan lives in the year, as a pick offers it.
    pub own_state: String,
}

const BRACKET_COLUMNS: [&str; 2] = ["Taxable income over", "Rate"];
const LABELLED_COLUMNS: [&str; 2] = ["", "Amount"];
const IRMAA_COLUMNS: [&str; 3] = ["MAGI over", "Part B a year", "Part D a year"];
const RMD_COLUMNS: [&str; 2] = ["Age", "Divisor"];
const NOT_PUBLISHED: &str = "not yet published";
const NO_SURCHARGES: &str = "No surcharges in this year's table.";
const NO_INCOME_TAX: &str = "This state has no income tax.";
const SPOUSE_BAND: &str = "IRA deduction phase-out, spouse covered (MAGI)";
const STATE_TITLE: &str = "State income tax";
/// Where a plan living in no U.S. state lives, as the state pick says it.
const ABROAD: &str = "no U.S. state";

/// What `view` asks of `plan`'s tables: those its projection applies in
/// the year, the latest table grown past its year at the plan's inflation,
/// for the plan's filing status and the state it lives in, or those named.
#[must_use]
pub fn year_tables(plan: &Plan, tables: &TaxTables, view: &TablesView) -> YearTables {
    let year = view.year;
    let params = tables.params_for(year, &Inflation::constant(plan.plan.inflation));
    let own_state = state_lived_in(plan, year);
    let status = (view.status.as_deref())
        .and_then(|key| {
            FilingStatus::ALL
                .iter()
                .copied()
                .find(|each| each.as_str() == key)
        })
        .unwrap_or(plan.household.filing);
    let state = (view.state.clone()).or_else(|| own_state.map(str::to_owned));
    let mut sections = federal(&params, status);
    sections.push(social_security(
        &params,
        benefit_params(plan, tables).as_ref(),
        status,
        year,
    ));
    sections.extend(state_sections(&params, status, state.as_deref(), year));
    YearTables {
        year,
        status: status.as_str().to_owned(),
        status_name: filing_status(status),
        state_name: state.as_deref().map(|code| state_name(code).to_owned()),
        state,
        statuses: Vocabulary::FilingStatus.offers(),
        states: params.states.keys().map(|code| state_offer(code)).collect(),
        sections,
        about: ABOUT,
        status_pick: STATUS_PICK,
        state_pick: STATE_PICK,
        own_status: format!("The plan's ({})", filing_status(plan.household.filing)),
        own_state: format!(
            "Where the plan lives ({})",
            own_state.map_or(ABROAD, state_name)
        ),
    }
}

fn state_offer(code: &str) -> Offer {
    Offer {
        value: code.to_owned(),
        label: state_name(code).to_owned(),
    }
}

fn state_name(code: &str) -> &str {
    place_name(US_STATES, code).unwrap_or(code)
}

fn section(title: &str, columns: &[&'static str], rows: Vec<Vec<String>>) -> TaxSection {
    TaxSection {
        title: title.to_owned(),
        columns: columns.to_vec(),
        rows,
        note: None,
    }
}

fn noted(title: &str, note: String) -> TaxSection {
    TaxSection {
        title: title.to_owned(),
        columns: Vec::new(),
        rows: Vec::new(),
        note: Some(note),
    }
}

fn labelled(label: &str, value: String) -> Vec<String> {
    vec![label.to_owned(), value]
}

fn brackets(brackets: &[Bracket]) -> Vec<Vec<String>> {
    let row = |bracket: &Bracket| vec![money(bracket.over), rate(bracket.rate)];
    brackets.iter().map(row).collect()
}

fn phase_out(band: PhaseOut) -> String {
    format!("{} to {}", money(band.from), money(band.to))
}

fn federal(params: &TaxParams, status: FilingStatus) -> Vec<TaxSection> {
    let of = |amounts: PerStatus<Dollars>| money(amounts.get(status));
    let ltcg = &params.ltcg;
    let gains = vec![
        vec![money(0), rate(0.0)],
        vec![of(ltcg.zero_until), rate(ltcg.middle_rate)],
        vec![of(ltcg.fifteen_until), rate(ltcg.top_rate)],
    ];
    let deductions = vec![
        labelled("Standard deduction", of(params.deductions.standard)),
        labelled(
            "Early-withdrawal penalty",
            rate(params.early_withdrawal.penalty),
        ),
        labelled(
            "HSA non-medical penalty before 65",
            rate(params.early_withdrawal.hsa_penalty),
        ),
    ];
    let irmaa = if params.irmaa.is_empty() {
        noted("Medicare surcharges (IRMAA)", NO_SURCHARGES.to_owned())
    } else {
        let tier = |tier: &_| irmaa_row(tier, status);
        let rows = params.irmaa.iter().map(tier).collect();
        section("Medicare surcharges (IRMAA)", &IRMAA_COLUMNS, rows)
    };
    vec![
        section(
            "Income tax brackets",
            &BRACKET_COLUMNS,
            brackets(params.brackets.for_status(status)),
        ),
        section("Long-term capital gains", &BRACKET_COLUMNS, gains),
        section("Deductions and penalties", &LABELLED_COLUMNS, deductions),
        section(
            "Contribution limits",
            &LABELLED_COLUMNS,
            limits(params, status),
        ),
        irmaa,
        section(
            "Required minimum distributions",
            &RMD_COLUMNS,
            params.rmd.divisors.iter().map(rmd_row).collect(),
        ),
    ]
}

fn irmaa_row(tier: &IrmaaTier, status: FilingStatus) -> Vec<String> {
    vec![
        money(tier.magi_over.get(status)),
        money(tier.part_b),
        money(tier.part_d),
    ]
}

fn rmd_row(row: &RmdDivisor) -> Vec<String> {
    vec![row.age.to_string(), format!("{:.1}", row.divisor)]
}

fn limits(params: &TaxParams, status: FilingStatus) -> Vec<Vec<String>> {
    let limits = &params.limits;
    let dollars = [
        ("401(k), 403(b) and 457(b) deferral", limits.employer_plan),
        (
            "Workplace catch-up from 50",
            limits.employer_plan_catch_up_50,
        ),
        (
            "Workplace catch-up at 60 to 63",
            limits.employer_plan_catch_up_60,
        ),
        ("Everything paid into one plan", limits.overall_plan),
        ("SIMPLE IRA deferral", limits.simple),
        ("SIMPLE catch-up from 50", limits.simple_catch_up_50),
        ("SIMPLE catch-up at 60 to 63", limits.simple_catch_up_60),
        ("IRA, traditional and Roth together", limits.ira),
        ("IRA catch-up from 50", limits.ira_catch_up_50),
        ("HSA, self-only coverage", limits.hsa_self),
        ("HSA, family coverage", limits.hsa_family),
        ("HSA catch-up from 55", limits.hsa_catch_up_55),
    ];
    let bands = [
        ("Roth IRA phase-out (MAGI)", limits.roth_ira_phase_out),
        (
            "IRA deduction phase-out (MAGI)",
            limits.ira_deduction_phase_out,
        ),
    ];
    let dollars = dollars.map(|(label, amount)| labelled(label, money(amount)));
    let bands = bands.map(|(label, band)| labelled(label, phase_out(band.get(status))));
    let spouse = limits
        .spouse_ira_deduction_phase_out(status)
        .map(|band| labelled(SPOUSE_BAND, phase_out(band)));
    dollars.into_iter().chain(bands).chain(spouse).collect()
}

fn social_security(
    params: &TaxParams,
    formula: Option<&BenefitParams>,
    status: FilingStatus,
    year: i16,
) -> TaxSection {
    let thresholds = &params.social_security;
    let mut rows = vec![
        labelled(
            "Up to 50% taxed over provisional income of",
            money(thresholds.provisional_base.get(status)),
        ),
        labelled(
            "Up to 85% taxed over provisional income of",
            money(thresholds.provisional_upper.get(status)),
        ),
    ];
    if let Some(formula) = formula {
        let [first, second] = formula.bend_points(year);
        let cola = formula
            .cola
            .get(&year)
            .map_or_else(|| NOT_PUBLISHED.to_owned(), |&each| rate(each));
        rows.extend([
            labelled("Wage base", money(formula.wage_base(year))),
            labelled(
                "Bend points, first eligible this year",
                format!("{} and {} a month", money(first), money(second)),
            ),
            labelled("Cost-of-living adjustment", cola),
        ]);
    }
    section("Social Security", &LABELLED_COLUMNS, rows)
}

fn state_sections(
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
    let taxes_benefits = if table.taxes_social_security {
        "Yes"
    } else {
        "No"
    };
    let rows = vec![
        labelled("Standard deduction", money(table.deduction.get(status))),
        labelled("Taxes Social Security", taxes_benefits.to_owned()),
    ];
    let title = format!("{name} income tax");
    let state_brackets = table.brackets.for_status(status);
    let bracket_title = format!("{name} brackets");
    let bracketed = if state_brackets.is_empty() {
        noted(&bracket_title, NO_INCOME_TAX.to_owned())
    } else {
        section(&bracket_title, &BRACKET_COLUMNS, brackets(state_brackets))
    };
    vec![section(&title, &LABELLED_COLUMNS, rows), bracketed]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::examples::named;

    fn moving() -> Plan {
        let (_, _, text) = named("moving-states.toml").expect("the example");
        Plan::from_toml_str(text).unwrap()
    }

    fn asked(year: i16) -> TablesView {
        TablesView {
            year,
            ..TablesView::default()
        }
    }

    fn row<'a>(tables: &'a YearTables, title: &str, label: &str) -> &'a str {
        let section = tables
            .sections
            .iter()
            .find(|section| section.title == title);
        let rows = &section.expect(title).rows;
        let found = rows.iter().find(|row| row[0] == label).expect(label);
        &found[1]
    }

    #[test]
    fn the_plan_s_status_and_state_are_shown_until_others_are_named() {
        let tables = year_tables(&moving(), &TaxTables::embedded(), &asked(2026));
        assert_eq!(tables.status, "married-joint");
        assert_eq!(tables.state.as_deref(), Some("or"));
        let deductions = "Deductions and penalties";
        assert_eq!(row(&tables, deductions, "Standard deduction"), "$32,200");
        let hsa_penalty = "HSA non-medical penalty before 65";
        assert_eq!(row(&tables, deductions, hsa_penalty), "20%");
        assert_eq!(
            row(&tables, "Oregon income tax", "Standard deduction"),
            "$5,800"
        );
        let single = TablesView {
            status: Some("single".to_owned()),
            state: Some("wa".to_owned()),
            ..asked(2026)
        };
        let tables = year_tables(&moving(), &TaxTables::embedded(), &single);
        assert_eq!(row(&tables, deductions, "Standard deduction"), "$16,100");
        assert_eq!(tables.own_status, "The plan's (Married filing jointly)");
        assert_eq!(tables.status_name, "Single");
        assert_eq!(tables.state_name.as_deref(), Some("Washington"));
        assert_eq!(tables.own_state, "Where the plan lives (Oregon)");
        let washington = tables
            .sections
            .iter()
            .find(|each| each.title == "Washington brackets");
        assert_eq!(washington.unwrap().note.as_deref(), Some(NO_INCOME_TAX));
        assert!(tables.states.iter().any(|state| state.label == "Oregon"));
    }

    #[test]
    fn a_year_past_the_table_is_grown_at_the_plan_s_inflation() {
        let tables = year_tables(&moving(), &TaxTables::embedded(), &asked(2030));
        let grown = (32_200.0 * 1.025_f64.powi(4)).round() as Dollars;
        let deduction = row(&tables, "Deductions and penalties", "Standard deduction");
        let said = deduction.trim_start_matches('$').replace(',', "");
        let difference = said.parse::<Dollars>().unwrap() - grown;
        assert!(difference.abs() <= 50, "{deduction} against {grown}");
        let cola = row(&tables, "Social Security", "Cost-of-living adjustment");
        assert_eq!(cola, NOT_PUBLISHED);
        let bends = row(
            &tables,
            "Social Security",
            "Bend points, first eligible this year",
        );
        assert!(bends.ends_with("a month"));
    }

    #[test]
    fn the_spouse_s_ira_band_is_shown_on_a_joint_return_alone() {
        let title = "Contribution limits";
        let joint = year_tables(&moving(), &TaxTables::embedded(), &asked(2026));
        assert_eq!(row(&joint, title, SPOUSE_BAND), "$242,000 to $252,000");
        let single = TablesView {
            status: Some("single".to_owned()),
            ..asked(2026)
        };
        let single = year_tables(&moving(), &TaxTables::embedded(), &single);
        let section = single.sections.iter().find(|each| each.title == title);
        let rows = &section.expect(title).rows;
        assert!(rows.iter().all(|row| row[0] != SPOUSE_BAND));
    }

    #[test]
    fn a_plan_abroad_owes_no_state_tax() {
        let mut abroad = moving();
        abroad.residency.clear();
        let tables = year_tables(&abroad, &TaxTables::embedded(), &asked(2026));
        assert_eq!(tables.state, None);
        assert_eq!(tables.own_state, "Where the plan lives (no U.S. state)");
        let state = tables.sections.last().unwrap();
        assert_eq!(state.title, STATE_TITLE);
        assert!(state.note.as_deref().unwrap().contains("no U.S. state"));
    }
}
