//! An expense's phases: its amount restated from triggers, the year a
//! phase fires in split by months, and how the ceiling scales them.

mod common;

use retiretui_engine::optimize::{Measure, apply_spending, spending_ceiling, spending_overlay};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Plan, Scenario};
use retiretui_engine::project::project;
use retiretui_engine::search::Progress;

use common::{assert_issue, issues, plan_from, run};

/// Someone born in June 1980 spending `housing` from 2026, under
/// `items`: more expenses, events or incomes.
fn plan(housing: &str, items: &str) -> String {
    format!(
        r#"
schema = 1

[plan]
start_year = 2026
horizon_age = 55
inflation = 0.0

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1980-06-15

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 1000000

[[expenses]]
id = "housing"
amount = 12000
{housing}
{items}
"#
    )
}

fn housing_in(text: &str, year: i16) -> i64 {
    let projection = run(text);
    let row = projection
        .years
        .iter()
        .find(|row| row.year == year)
        .unwrap();
    row.spending.get("housing").copied().unwrap_or_default()
}

const MOVE_IN_OCTOBER: &str = r"
[[expenses.phases]]
from = { date = 2029-10-15 }
amount = 24000
";

#[test]
fn a_phase_at_a_date_splits_its_year_by_months_and_holds_after() {
    let text = plan(MOVE_IN_OCTOBER, "");
    assert_eq!(housing_in(&text, 2028), 12000);
    assert_eq!(housing_in(&text, 2029), 9000 + 6000);
    assert_eq!(housing_in(&text, 2030), 24000);
}

#[test]
fn the_split_year_is_escalated_whole() {
    let text = plan(&format!("cola = 0.1\n{MOVE_IN_OCTOBER}"), "");
    let expected = (15000.0_f64 * 1.1_f64.powi(3)).round() as i64;
    assert_eq!(housing_in(&text, 2029), expected);
}

#[test]
fn a_phase_at_an_age_splits_at_the_month_it_is_attained() {
    let text = plan(
        r#"
[[expenses.phases]]
from = { age = 50, owner = "me" }
amount = 24000
"#,
        "",
    );
    assert_eq!(housing_in(&text, 2030), 5000 + 14000);
}

#[test]
fn a_phase_at_an_event_splits_at_the_events_date_and_an_offset_keeps_the_month() {
    let event = r#"
[[events]]
id = "move"
trigger = { date = 2029-10-15 }
"#;
    let at_move = plan(
        r#"
[[expenses.phases]]
from = { event = "move" }
amount = 24000
"#,
        event,
    );
    assert_eq!(housing_in(&at_move, 2029), 15000);
    let year_after = plan(
        r#"
[[expenses.phases]]
from = { event = "move", offset = 1 }
amount = 24000
"#,
        event,
    );
    assert_eq!(housing_in(&year_after, 2029), 12000);
    assert_eq!(housing_in(&year_after, 2030), 15000);
    assert_eq!(housing_in(&year_after, 2031), 24000);
}

#[test]
fn a_phase_at_an_incomes_start_splits_at_the_month_it_starts() {
    let text = plan(
        r#"
[[expenses.phases]]
from = { income = "pension" }
amount = 24000
"#,
        r#"
[[income]]
id = "pension"
kind = "pension"
owner = "me"
amount = 1000
start = { date = 2031-04-01 }
"#,
    );
    assert_eq!(housing_in(&text, 2031), 3000 + 18000);
}

#[test]
fn two_phases_in_one_year_each_take_their_months() {
    let text = plan(
        r"
[[expenses.phases]]
from = { date = 2029-09-01 }
amount = 36000

[[expenses.phases]]
from = { date = 2029-03-01 }
amount = 24000
",
        "",
    );
    assert_eq!(housing_in(&text, 2029), 2000 + 12000 + 12000);
}

#[test]
fn the_phase_fired_latest_is_in_force_and_a_later_listed_one_wins_a_tie() {
    let text = plan(
        r"
[[expenses.phases]]
from = { date = 2029-01-01 }
amount = 36000

[[expenses.phases]]
from = { date = 2028-01-01 }
amount = 24000

[[expenses.phases]]
from = { date = 2029-01-01 }
amount = 48000
",
        "",
    );
    assert_eq!(housing_in(&text, 2027), 12000);
    assert_eq!(housing_in(&text, 2028), 24000);
    assert_eq!(housing_in(&text, 2030), 48000);
}

#[test]
fn a_phase_fired_before_the_window_is_in_force_from_its_start() {
    let text = plan(
        r"
start = { date = 2030-01-01 }

[[expenses.phases]]
from = { date = 2028-07-01 }
amount = 24000
",
        "",
    );
    assert_eq!(housing_in(&text, 2029), 0);
    assert_eq!(housing_in(&text, 2030), 24000);
}

#[test]
fn a_phase_is_refused_where_it_cannot_be_spent() {
    let negative = plan(
        r"
[[expenses.phases]]
from = { date = 2029-01-01 }
amount = -1
",
        "",
    );
    assert_issue(
        &issues(&negative),
        "expenses[0].phases[0].amount",
        "must not be negative",
    );
    let once = plan(
        r"
on = { date = 2029-01-01 }

[[expenses.phases]]
from = { date = 2030-01-01 }
amount = 1
",
        "",
    );
    assert_issue(
        &issues(&once),
        "expenses[0].phases",
        "`on` excludes `phases`",
    );
    let unowned = plan(
        r"
[[expenses.phases]]
from = { age = 50 }
amount = 1
",
        "",
    );
    assert_issue(&issues(&unowned), "expenses[0].phases[0].from", "owner");
}

#[test]
fn the_ceiling_scales_every_phase_and_its_overlay_restates_them() {
    let text = plan(MOVE_IN_OCTOBER, "");
    let plan = plan_from(&text);
    let found = spending_ceiling(
        &plan,
        &TaxTables::embedded(),
        retiretui_engine::market::History::embedded(),
        Measure::Planned,
        &Progress::default(),
    )
    .unwrap();
    let scaled = &found.expenses[0];
    assert_eq!(
        scaled.phases,
        [(24000.0 * found.factor).floor() as i64],
        "{found:?}"
    );
    assert_ne!(scaled.phases[0], 24000);
    let overlay = spending_overlay("base.toml", &plan, &found.expenses).unwrap();
    assert!(
        overlay.contains("[[expenses.phases]]\namount = ")
            && overlay.contains("[expenses.phases.from]\ndate = 2029-10-15\n"),
        "{overlay}"
    );
    let scenario = Scenario::from_toml_str(&overlay)
        .unwrap()
        .expect("a scenario");
    let merged = scenario.apply(toml::from_str(&text).unwrap()).unwrap();
    let merged_plan = Plan::from_toml_table(merged).unwrap();
    let mut applied = plan.clone();
    apply_spending(&mut applied, &found.expenses);
    assert_eq!(applied, merged_plan);
    assert_eq!(
        project(&merged_plan, &TaxTables::embedded()),
        found.judged.projection
    );
}
