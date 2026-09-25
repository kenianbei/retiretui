use retiretui_engine::plan::{Plan, Settings};

use super::domain::{FieldSpec, Single};
use super::offers::{RefSource, Vocabulary};
use crate::commands::tui::nav::Page;

pub struct Config;

const SKIPPED: &str = "Skip";
const ORDER_HELP: &str = "The kinds of account spent from, in turn, when income falls short. A kind skipped is never drawn on.";

const fn drawn_from(label: &'static str, place: usize) -> FieldSpec {
    FieldSpec::ordered("withdrawal_order", label, Vocabulary::TreatmentClass, place)
        .blank(SKIPPED)
        .help(ORDER_HELP)
}

impl Single for Config {
    type Item = Settings;
    const PAGE: Page = Page::Settings;
    const PATHS: &'static [&'static str] = &["plan"];
    const FIELDS: &'static [FieldSpec] = &[
        FieldSpec::text("name", "Plan name").help("What the plan is called wherever it is shown."),
        FieldSpec::whole("start_year", "Start year").help("The first calendar year projected."),
        FieldSpec::whole("horizon_age", "Plan to age")
            .help("The projection runs until the oldest person reaches this age."),
        FieldSpec::rate("inflation", "Inflation")
            .help("Yearly inflation, which every amount grows with unless it says otherwise."),
        FieldSpec::rate("wage_growth", "Wage growth")
            .blank("SSA's assumption")
            .help("Yearly growth of the national average wage, which a computed Social Security benefit is indexed over."),
        drawn_from("Withdraw first", 0),
        drawn_from("then", 1),
        drawn_from("then", 2),
        drawn_from("last", 3),
        FieldSpec::refers("surplus_to", "Surplus goes to", RefSource::Account)
            .blank("First cash account")
            .help("The account unspent income is saved into."),
    ];

    fn get(plan: &Plan) -> Settings {
        plan.plan.clone()
    }

    fn set(plan: &mut Plan, settings: Settings) {
        plan.plan = settings;
    }
}
