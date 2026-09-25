use retiretui_engine::plan::{FilingStatus, Medicare, Person, Plan, Residency};
use serde::{Deserialize, Serialize};
use toml::{Table, Value};

use super::applies;
use super::cells::{Column, field_text};
use super::domain::{Domain, FieldKind, FieldSpec, Single};
use super::offers::Vocabulary;
use crate::commands::tui::nav::Page;

pub struct People;

impl Domain for People {
    type Item = Person;
    const PAGE: Page = Page::People;
    const PURPOSE: &'static str = "Who the plan is for, and when they were born";
    const PATH: &'static str = "household.people";
    const SINGULAR: &'static str = "Person";
    const FIELDS: &'static [FieldSpec] = &[
        FieldSpec::text("id", "ID")
            .help("A short name for this person, which accounts and income refer to."),
        FieldSpec::text("name", "Name")
            .help("The person's name as it is shown. Blank shows the ID."),
        FieldSpec::text("birth", "Birth date")
            .help("As year-month-day, such as 1975-06-14. Ages are counted from it."),
    ];
    const COLUMNS: &'static [Column] = &[
        Column::new("id").headed("Person"),
        Column::new("birth"),
        Column::new(EARNINGS_KEY)
            .headed("Earnings")
            .phrased(earnings_span),
    ];
    const BLANK: &'static str = "
birth = 1980-01-01
";
    const RECORD: Option<fn(&Table) -> Vec<[String; 2]>> = Some(earnings_record);

    fn items(plan: &Plan) -> &[Person] {
        &plan.household.people
    }

    fn items_mut(plan: &mut Plan) -> &mut Vec<Person> {
        &mut plan.household.people
    }
}

const EARNINGS_KEY: &str = "earnings";
const NO_EARNINGS: &str = "-";
const RECORD_HEADING: &str = "Earnings";

/// The record's rows for the details pane: a heading, then each year's
/// covered earnings, earliest first.
fn earnings_record(item: &Table) -> Vec<[String; 2]> {
    let years = item.get(EARNINGS_KEY).and_then(Value::as_table);
    let mut rows = vec![[RECORD_HEADING.to_owned(), String::new()]];
    rows.extend(years.into_iter().flatten().map(|(year, amount)| {
        let amount = field_text(FieldKind::Money, Some(amount), false);
        [year.clone(), amount]
    }));
    rows
}

/// The years a person's earnings record spans; the record itself is only
/// ever imported, never typed.
fn earnings_span(item: &Table, _: &Plan) -> String {
    let years = item.get(EARNINGS_KEY).and_then(Value::as_table);
    match years.and_then(|years| years.keys().min().zip(years.keys().max())) {
        Some((first, last)) => format!("{first}-{last}"),
        None => NO_EARNINGS.to_owned(),
    }
}

pub struct Residencies;

impl Domain for Residencies {
    type Item = Residency;
    const PAGE: Page = Page::Residency;
    const PURPOSE: &'static str = "Where you live, and when that changes";
    const PATH: &'static str = "residency";
    const SINGULAR: &'static str = "Residency";
    const IDENTITY: &'static str = "country";
    const FIELDS: &'static [FieldSpec] = &[
        FieldSpec::choice("country", "Country", Vocabulary::Country)
            .help("Where you live. Space searches the countries by name or code."),
        FieldSpec::choice("state", "State", Vocabulary::UsState)
            .shown_when(applies::is_in_the_us)
            .blank("None")
            .help("The state you live in. Its income tax is part of each year's taxes."),
        FieldSpec::trigger("from", "From")
            .blank("Plan start")
            .help("When you move there. Blank means the start of the plan."),
    ];
    const COLUMNS: &'static [Column] = &[
        Column::new("country"),
        Column::new("state"),
        Column::new("from"),
    ];
    const BLANK: &'static str = r#"
country = "us"
"#;

    fn items(plan: &Plan) -> &[Residency] {
        &plan.residency
    }

    fn items_mut(plan: &mut Plan) -> &mut Vec<Residency> {
        &mut plan.residency
    }
}

/// The household's scalar settings, edited as one form.
#[derive(Serialize, Deserialize)]
pub struct HouseholdSettings {
    filing: FilingStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    medicare: Option<Medicare>,
}

pub struct Household;

impl Single for Household {
    type Item = HouseholdSettings;
    const PAGE: Page = Page::Household;
    const PATHS: &'static [&'static str] = &["household", "medicare"];
    const FIELDS: &'static [FieldSpec] = &[
        FieldSpec::choice("filing", "Filing status", Vocabulary::FilingStatus).help(
            "How the household files its federal return. Married filing jointly needs two people.",
        ),
        FieldSpec::presence("medicare", "Medicare surcharges", "part_d = true")
            .help("Whether to charge IRMAA, what Medicare adds to a premium at a higher income."),
        FieldSpec::flag("medicare.part_d", "Include Part D")
            .help("Whether the surcharge on drug coverage is charged beside Part B's."),
        FieldSpec::listed("medicare.prior_magi", "Income last year", 0).help(
            "Your MAGI the year before the plan starts, which prices its second year. Optional.",
        ),
        FieldSpec::listed("medicare.prior_magi", "Income year before", 1).help(
            "Your MAGI two years before the plan starts, which prices its first year. Optional.",
        ),
    ];

    fn get(plan: &Plan) -> HouseholdSettings {
        HouseholdSettings {
            filing: plan.household.filing,
            medicare: plan.medicare.clone(),
        }
    }

    fn set(plan: &mut Plan, settings: HouseholdSettings) {
        plan.household.filing = settings.filing;
        plan.medicare = settings.medicare;
    }
}
