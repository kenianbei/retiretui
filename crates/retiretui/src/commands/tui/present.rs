//! How the plan's values read on screen: the words for what the schema
//! spells as keys and kebab-case, each a whole phrase rather than a part
//! of one.

use retiretui_engine::plan::{
    AccountKind, COUNTRIES, Dollars, Draw, FilingStatus, IncomeKind, Payer, Plan, PlanDate,
    Residency, TreatmentClass, Trigger, TriggerBasis, TriggerForm, US_STATES, place_name,
};
use retiretui_engine::project::Summary;
use toml::Value;

use super::edit::from_table;
pub use crate::commands::table::{account_name, event_name, income_name, money, rate};

/// The dollars figures are shown in.
pub const fn basis_name(is_nominal: bool) -> &'static str {
    if is_nominal {
        "future dollars"
    } else {
        "today's dollars"
    }
}

pub const fn account_kind(kind: AccountKind) -> &'static str {
    match kind {
        AccountKind::K401k => "401(k)",
        AccountKind::K403b => "403(b)",
        AccountKind::K457b => "457(b)",
        AccountKind::K414k => "414(k)",
        AccountKind::Ira => "IRA",
        AccountKind::SepIra => "SEP IRA",
        AccountKind::SimpleIra => "SIMPLE IRA",
        AccountKind::Hsa => "HSA",
        AccountKind::Brokerage => "Brokerage",
        AccountKind::Cash => "Cash",
    }
}

pub const fn draw(draw: Draw) -> &'static str {
    match draw {
        Draw::Assumptions => "The assumptions",
        Draw::History => "Historical years",
    }
}

pub const fn payer(payer: Payer) -> &'static str {
    match payer {
        Payer::Employee => "Employee",
        Payer::Employer => "Employer",
        Payer::AfterTax => "After tax",
    }
}

pub const fn income_kind(kind: IncomeKind) -> &'static str {
    match kind {
        IncomeKind::Salary => "Salary",
        IncomeKind::Pension => "Pension",
        IncomeKind::Annuity => "Annuity",
        IncomeKind::Rental => "Rental",
        IncomeKind::SocialSecurity => "Social Security",
        IncomeKind::Windfall => "Windfall",
        IncomeKind::Other => "Other",
    }
}

pub const fn filing_status(status: FilingStatus) -> &'static str {
    match status {
        FilingStatus::Single => "Single",
        FilingStatus::MarriedJoint => "Married filing jointly",
    }
}

pub const fn treatment_class(class: TreatmentClass) -> &'static str {
    match class {
        TreatmentClass::Taxable => "Taxable",
        TreatmentClass::Deferred => "Deferred",
        TreatmentClass::Roth => "Roth",
        TreatmentClass::Hsa => "HSA",
    }
}

/// Short enough for the select a trigger's kind is picked in.
pub const fn trigger_basis(basis: TriggerBasis) -> &'static str {
    match basis {
        TriggerBasis::Date => "Date",
        TriggerBasis::Age => "Age",
        TriggerBasis::Event => "Event",
        TriggerBasis::Income => "Income",
    }
}

/// Dollars as [`money`] writes them or as plain digits.
pub fn parse_money(text: &str) -> Option<Dollars> {
    let digits: String = text
        .chars()
        .filter(|&each| !matches!(each, '$' | ',' | ' '))
        .collect();
    digits.parse().ok()
}

/// Dollars in as few cells as say how much: `450k`, `2.58M`.
pub fn compact_dollars(amount: Dollars) -> String {
    let magnitude = amount.abs();
    if magnitude >= 1_000_000 {
        format!("{:.2}M", amount as f64 / 1e6)
    } else if magnitude >= 10_000 {
        format!("{}k", amount / 1_000)
    } else {
        amount.to_string()
    }
}

/// [`compact_dollars`] as a sum of money: `$2.58M`, `-$42k`.
pub fn compact_money(amount: Dollars) -> String {
    let sign = if amount < 0 { "-" } else { "" };
    format!("{sign}${}", compact_dollars(amount.abs()))
}

/// How many issues there are: `1 issue`, `3 issues`.
pub fn issue_count(count: usize) -> String {
    let plural = if count == 1 { "" } else { "s" };
    format!("{count} issue{plural}")
}

/// A difference in money, signed either way: `+$220k`, `-$12k`, `$0`.
pub fn signed_money(amount: Dollars) -> String {
    let sign = if amount > 0 { "+" } else { "" };
    format!("{sign}{}", compact_money(amount))
}

const PERCENT: f64 = 100.0;
/// A rate is kept to a millionth, which a percent typed to four places is.
const RATE_GRAIN: f64 = 1e6;

/// A percent, with or without its sign, as the rate the file keeps.
pub fn parse_rate(text: &str) -> Option<f64> {
    let percent: f64 = text.trim().trim_end_matches('%').trim().parse().ok()?;
    Some((percent / PERCENT * RATE_GRAIN).round() / RATE_GRAIN)
}

pub const FOLLOWS_INFLATION: &str = "Inflation";
const HELD_FIXED: &str = "Fixed";

/// How an amount grows. Unstated is the schema's default, which follows
/// inflation.
pub fn growth(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Boolean(true)) => FOLLOWS_INFLATION.to_owned(),
        Some(Value::Boolean(false)) => HELD_FIXED.to_owned(),
        Some(Value::Float(own)) => rate(*own),
        Some(Value::Integer(own)) => rate(*own as f64),
        Some(other) => other.to_string(),
    }
}

/// What [`growth`] writes, or the file's own `true` and `false`.
pub fn parse_growth(text: &str) -> Option<Value> {
    let word = text.trim().to_ascii_lowercase();
    match word.as_str() {
        "" => None,
        "inflation" | "true" => Some(Value::Boolean(true)),
        "fixed" | "false" => Some(Value::Boolean(false)),
        _ => parse_rate(&word).map(Value::Float),
    }
}

/// A date to the month, which is as fine as a plan's years resolve.
fn month(date: PlanDate) -> String {
    date.0.strftime("%b %-Y").to_string()
}

/// A span of whole years: "1 yr", "3 yrs".
fn years(span: u32) -> String {
    let unit = if span == 1 { "yr" } else { "yrs" };
    format!("{span} {unit}")
}

/// How many years before or after, ahead of what they are counted from.
fn shifted(offset: i32, from: &str, unshifted: String) -> String {
    if offset == 0 {
        return unshifted;
    }
    let side = if offset < 0 { "before" } else { "after" };
    format!("{} {side} {from}", years(offset.unsigned_abs()))
}

pub const ENDS_WITH: &str = "Ends with";
pub const MONEY_LASTS: &str = "Money lasts";
pub const PEAKS_AT: &str = "Peaks at";
pub const LIFETIME_TAXES: &str = "Lifetime taxes";

/// What reads as no difference from the baseline.
pub const SAME: &str = "same";

/// A move of `offset` years along the calendar: "same", "1 yr later",
/// "3 yrs sooner".
fn shift(offset: i16) -> String {
    let span = years(u32::from(offset.unsigned_abs()));
    match offset.signum() {
        0 => SAME.to_owned(),
        1 => format!("{span} later"),
        _ => format!("{span} sooner"),
    }
}

/// How long the money lasts: never short, or by how much from when.
pub fn money_lasts(summary: &Summary) -> String {
    summary.first_unfunded_year.map_or_else(
        || "Never short".to_owned(),
        |year| {
            format!(
                "Short {} from {year}",
                compact_money(summary.lifetime_unfunded)
            )
        },
    )
}

/// How long the money lasts, against how long the baseline's does.
pub fn money_lasts_against(own: &Summary, base: &Summary) -> String {
    match (base.first_unfunded_year, own.first_unfunded_year) {
        (None, None) => SAME.to_owned(),
        (Some(_), None) => "now never short".to_owned(),
        (None, Some(year)) => format!("now short in {year}"),
        (Some(then), Some(year)) => shift(year - then),
    }
}

/// The highest net worth reached, and when.
pub fn peaks_at(summary: &Summary) -> String {
    format!(
        "{} in {}",
        compact_money(summary.peak_net_worth),
        summary.peak_year
    )
}

/// The peak's difference from the baseline's in amount, and in when it
/// comes.
pub fn peaks_at_against(own: &Summary, base: &Summary) -> String {
    let amount = signed_money(own.peak_net_worth - base.peak_net_worth);
    let when = match own.peak_year - base.peak_year {
        0 => "same year".to_owned(),
        offset => shift(offset),
    };
    format!("{amount}, {when}")
}

/// Where a residency is: its state by name, else its country.
pub fn residence(residency: &Residency) -> &str {
    let named = match &residency.state {
        Some(state) => place_name(US_STATES, state),
        None => place_name(COUNTRIES, &residency.country),
    };
    named.unwrap_or_else(|| residency.state.as_deref().unwrap_or(&residency.country))
}

/// A mix as whole percents, stocks then bonds, then cash where it holds
/// any: `60/40`, `40/50/10`.
pub fn mix(stocks: f64, bonds: f64, cash: f64) -> String {
    let percent = |share: f64| format!("{:.0}", share * PERCENT);
    let mut parts = vec![percent(stocks), percent(bonds)];
    if cash > 0.0 {
        parts.push(percent(cash));
    }
    parts.join("/")
}

/// A trigger as a phrase: `Jan 2028`, `age 80 (jordan)`, `at retire`,
/// `2 yrs after retire`. It is read through the schema's own type, and a
/// value that is no trigger reads as written.
pub fn trigger(value: &Value, plan: &Plan) -> String {
    let stated = value.as_table().cloned().map(from_table::<Trigger>);
    let Some(Ok(stated)) = stated else {
        return value.to_string();
    };
    match stated.form() {
        Ok(TriggerForm::Date(date)) => month(date),
        Ok(TriggerForm::Age { owner, years }) => format!("age {years} ({owner})"),
        Ok(TriggerForm::Event { id, offset }) => {
            let named = event_name(plan, id);
            shifted(offset, named, format!("at {named}"))
        }
        Ok(TriggerForm::Income { id, offset }) => {
            let starts = format!("{} starts", income_name(plan, id));
            shifted(offset, &starts, format!("when {starts}"))
        }
        Err(_) => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(text: &str) -> Value {
        let table: toml::Table = format!("v = {text}").parse().unwrap();
        table["v"].clone()
    }

    #[test]
    fn money_groups_thousands_and_reads_itself_back() {
        let cases = [
            (0, "$0"),
            (500, "$500"),
            (1_000, "$1,000"),
            (450_000, "$450,000"),
            (1_234_567, "$1,234,567"),
            (-5_000, "-$5,000"),
        ];
        for (amount, shown) in cases {
            assert_eq!(money(amount), shown);
            assert_eq!(parse_money(shown), Some(amount), "{shown}");
            assert_eq!(parse_money(&amount.to_string()), Some(amount));
        }
        assert_eq!(parse_money("lots"), None);
    }

    #[test]
    fn compact_dollars_scales() {
        assert_eq!(compact_dollars(0), "0");
        assert_eq!(compact_dollars(950), "950");
        assert_eq!(compact_dollars(9_999), "9999");
        assert_eq!(compact_dollars(42_000), "42k");
        assert_eq!(compact_dollars(1_234_567), "1.23M");
        assert_eq!(compact_dollars(-42_000), "-42k");
        assert_eq!(compact_money(2_580_000), "$2.58M");
        assert_eq!(compact_money(-42_000), "-$42k");
        assert_eq!(signed_money(220_000), "+$220k");
        assert_eq!(signed_money(-12_000), "-$12k");
        assert_eq!(signed_money(0), "$0");
    }

    #[test]
    fn a_rate_is_a_percent_and_reads_itself_back_exactly() {
        let cases = [
            (0.0, "0%"),
            (0.025, "2.5%"),
            (0.06, "6%"),
            (0.0125, "1.25%"),
            (-0.01, "-1%"),
        ];
        for (stated, shown) in cases {
            assert_eq!(rate(stated), shown);
            assert_eq!(parse_rate(shown), Some(stated), "{shown}");
        }
        assert_eq!(parse_rate("7"), Some(0.07), "the sign is optional");
        assert_eq!(parse_rate("fast"), None);
    }

    #[test]
    fn growth_says_what_the_amount_follows() {
        assert_eq!(growth(None), "Inflation", "the schema's default");
        assert_eq!(growth(Some(&Value::Boolean(true))), "Inflation");
        assert_eq!(growth(Some(&Value::Boolean(false))), "Fixed");
        assert_eq!(growth(Some(&Value::Float(0.03))), "3%");
        for shown in ["Inflation", "Fixed", "3%"] {
            let parsed = parse_growth(shown);
            assert_eq!(growth(parsed.as_ref()), shown);
        }
        assert_eq!(parse_growth("true"), Some(Value::Boolean(true)));
        assert_eq!(parse_growth(""), None);
    }

    #[test]
    fn a_mix_names_cash_only_where_it_holds_some() {
        assert_eq!(mix(0.6, 0.4, 0.0), "60/40");
        assert_eq!(mix(0.4, 0.5, 0.1), "40/50/10");
    }

    fn summary(short_from: Option<i16>, peak: (i64, i16)) -> Summary {
        Summary {
            final_net_worth: 0,
            peak_net_worth: peak.0,
            peak_year: peak.1,
            lifetime_taxes: 0,
            lifetime_conversions: 0,
            lifetime_unfunded: 0,
            lifetime_medicare: 0,
            first_unfunded_year: short_from,
            final_deferred: 0,
        }
    }

    #[test]
    fn years_read_as_shifts_from_the_baseline() {
        let never = summary(None, (1_000_000, 2040));
        let short = |year| summary(Some(year), (1_000_000, 2040));
        assert_eq!(money_lasts_against(&never, &never), "same");
        assert_eq!(money_lasts_against(&never, &short(2060)), "now never short");
        assert_eq!(
            money_lasts_against(&short(2063), &never),
            "now short in 2063"
        );
        assert_eq!(
            money_lasts_against(&short(2063), &short(2060)),
            "3 yrs later"
        );
        assert_eq!(
            money_lasts_against(&short(2059), &short(2060)),
            "1 yr sooner"
        );
        assert_eq!(money_lasts_against(&short(2060), &short(2060)), "same");
        let later = summary(None, (1_120_000, 2042));
        assert_eq!(peaks_at_against(&later, &never), "+$120k, 2 yrs later");
        assert_eq!(peaks_at_against(&never, &never), "$0, same year");
    }

    #[test]
    fn every_trigger_kind_reads_as_a_phrase() {
        let plan: Plan = toml::from_str(include_str!(
            "../../../../retiretui_engine/tests/fixtures/full.toml"
        ))
        .unwrap();
        let cases = [
            ("{ date = 2028-01-01 }", "Jan 2028"),
            ("{ date = 2037-12-31 }", "Dec 2037"),
            ("{ age = 80, owner = \"jordan\" }", "age 80 (jordan)"),
            ("{ event = \"retire\" }", "at retire"),
            ("{ event = \"retire\", offset = 0 }", "at retire"),
            ("{ event = \"retire\", offset = 2 }", "2 yrs after retire"),
            ("{ event = \"retire\", offset = -1 }", "1 yr before retire"),
            ("{ income = \"db-pension\" }", "when db-pension starts"),
            (
                "{ income = \"db-pension\", offset = 1 }",
                "1 yr after db-pension starts",
            ),
        ];
        for (stated, phrase) in cases {
            assert_eq!(trigger(&value(stated), &plan), phrase, "{stated}");
        }
    }
}
