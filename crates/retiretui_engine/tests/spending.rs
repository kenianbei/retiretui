//! Spending ceiling tests over one retiree, who spends on what they could
//! cut, a mortgage they could not, and a roof once.

mod common;

use retiretui_engine::market::{History, Progress, RunError, monte_carlo};
use retiretui_engine::optimize::{
    CEILING_STEPS, MAX_FACTOR, Measure, ScaledExpense, SpendingCeiling, apply_spending,
    spending_ceiling, spending_overlay,
};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Plan, Scenario};
use retiretui_engine::project::project;

use common::{assert_issue, plan_from};

const RETIREE: &str = include_str!("fixtures/spending-plan.toml");

const TARGET: f64 = 0.9;

fn ceiling_of(plan: &Plan, measure: Measure) -> Result<SpendingCeiling, RunError> {
    spending_ceiling(
        plan,
        &TaxTables::embedded(),
        History::embedded(),
        measure,
        &Progress::default(),
    )
}

fn amounts(plan: &Plan) -> Vec<(&str, i64)> {
    let stated = plan.expenses.iter();
    stated.map(|it| (it.id.as_str(), it.amount)).collect()
}

fn refusal(found: Result<SpendingCeiling, RunError>) -> Vec<retiretui_engine::plan::Issue> {
    match found.unwrap_err() {
        RunError::Refused(issues) => issues,
        RunError::Cancelled => panic!("nothing cancels it"),
    }
}

/// `plan` with `id` spent at `amount`.
fn spending(plan: &Plan, id: &str, amount: i64) -> Plan {
    let mut changed = plan.clone();
    apply_spending(
        &mut changed,
        &[ScaledExpense {
            id: id.to_owned(),
            amount,
        }],
    );
    changed
}

#[test]
fn an_expense_is_essential_only_where_it_says_so() {
    let plan = plan_from(RETIREE);
    let flexible: Vec<&str> = plan
        .expenses
        .iter()
        .filter(|expense| expense.is_flexible())
        .map(|expense| expense.id.as_str())
        .collect();
    assert_eq!(flexible, ["living", "travel"]);
    let written = plan.to_toml_string().unwrap();
    assert_eq!(written.matches("essential = true").count(), 1, "{written}");
    assert!(!written.contains("essential = false"), "{written}");
    assert_eq!(Plan::from_toml_str(&written).unwrap(), plan);
}

#[test]
fn the_ceiling_in_the_plans_own_market_lasts_and_a_hundred_more_does_not() {
    let plan = plan_from(RETIREE);
    let tables = TaxTables::embedded();
    let found = ceiling_of(&plan, Measure::Planned).unwrap();
    assert!(found.baseline.is_met && found.judged.is_met);
    assert_eq!(found.baseline.projection, project(&plan, &tables));
    assert!(found.factor > 1.0 && !found.is_capped, "{}", found.factor);
    assert_eq!(found.judged.success_rate, None);

    let mut at_ceiling = plan.clone();
    apply_spending(&mut at_ceiling, &found.expenses);
    assert_eq!(found.judged.projection, project(&at_ceiling, &tables));
    assert_eq!(
        found.judged.projection.summary(true).first_unfunded_year,
        None
    );
    let [living, travel] = &found.expenses[..] else {
        panic!("two flexible expenses, {:?}", found.expenses);
    };
    assert_eq!(living.amount, (30_000.0 * found.factor).floor() as i64);
    assert_eq!(travel.amount, (10_000.0 * found.factor).floor() as i64);
    assert_eq!(
        amounts(&at_ceiling),
        [
            ("living", living.amount),
            ("mortgage", 12_000),
            ("roof", 25_000),
            ("travel", travel.amount)
        ],
        "what is essential or spent once is left as stated"
    );

    let past = spending(&at_ceiling, "living", living.amount + 100);
    assert!(
        project(&past, &tables)
            .summary(true)
            .first_unfunded_year
            .is_some(),
        "the search settles within a hundred dollars a year"
    );
}

#[test]
fn a_reserve_to_leave_lowers_the_ceiling() {
    let plan = plan_from(RETIREE);
    let reserved = plan_from(&format!("{RETIREE}\n[market]\nleave_at_least = 300000\n"));
    let spent = ceiling_of(&plan, Measure::Planned).unwrap();
    let kept = ceiling_of(&reserved, Measure::Planned).unwrap();
    assert!(kept.factor < spent.factor);
    assert!(kept.judged.projection.summary(true).final_net_worth >= 300_000);
    assert!(spent.judged.projection.summary(true).final_net_worth < 300_000);
}

#[test]
fn a_plan_that_runs_short_is_answered_with_less_than_it_spends() {
    let plan = plan_from(&RETIREE.replace("balance = 1200000", "balance = 700000"));
    let found = ceiling_of(&plan, Measure::Planned).unwrap();
    assert!(!found.baseline.is_met && found.judged.is_met);
    assert!(found.factor > 0.0 && found.factor < 1.0, "{}", found.factor);
    assert!(found.expenses[0].amount < 30_000);
}

#[test]
fn a_ceiling_held_to_a_share_of_markets_meets_it_and_is_the_lower() {
    let plan = plan_from(RETIREE);
    let (tables, history) = (TaxTables::embedded(), History::embedded());
    let planned = ceiling_of(&plan, Measure::Planned).unwrap();
    let found = ceiling_of(&plan, Measure::Success(TARGET)).unwrap();
    assert!(found.factor < planned.factor);

    let rate = |plan: &Plan| {
        let runs = monte_carlo(plan, &tables, history, &Progress::default()).unwrap();
        Some(runs.runs.success_rate())
    };
    assert_eq!(found.baseline.success_rate, rate(&plan));
    let mut at_ceiling = plan.clone();
    apply_spending(&mut at_ceiling, &found.expenses);
    assert_eq!(found.judged.success_rate, rate(&at_ceiling));
    assert!(found.judged.success_rate >= Some(TARGET));

    let mut at_planned = plan.clone();
    apply_spending(&mut at_planned, &planned.expenses);
    assert!(
        rate(&at_planned) < Some(TARGET),
        "what lasts in the plan's own market lasts in fewer than nine in ten"
    );
}

#[test]
fn a_search_counts_each_market_and_judges_no_more_plans_than_its_steps() {
    let progress = Progress::default();
    spending_ceiling(
        &plan_from(RETIREE),
        &TaxTables::embedded(),
        History::embedded(),
        Measure::Success(TARGET),
        &progress,
    )
    .unwrap();
    assert_eq!(progress.done() % 100, 0);
    assert!(
        progress.done() > 100 && progress.done() <= CEILING_STEPS * 100,
        "{}",
        progress.done()
    );
}

#[test]
fn a_plan_that_lasts_at_eight_times_its_spending_is_capped_there() {
    let text = RETIREE
        .replace("amount = 30000", "amount = 300")
        .replace("amount = 10000", "amount = 100");
    let found = ceiling_of(&plan_from(&text), Measure::Planned).unwrap();
    assert!(found.is_capped);
    assert!((found.factor - MAX_FACTOR).abs() < f64::EPSILON);
    assert_eq!(found.expenses[0].amount, 2_400);
}

#[test]
fn a_plan_with_nothing_flexible_is_refused() {
    let essential = RETIREE.replace("amount = 30000", "amount = 30000\nessential = true");
    let text = essential.replace("amount = 10000", "amount = 0");
    let issues = refusal(ceiling_of(&plan_from(&text), Measure::Planned));
    assert_eq!(issues.len(), 1);
    assert_issue(&issues, "expenses", "no flexible spending");
}

#[test]
fn a_plan_short_with_no_flexible_spending_is_refused() {
    let plan = plan_from(&RETIREE.replace("balance = 1200000", "balance = 100000"));
    let issues = refusal(ceiling_of(&plan, Measure::Planned));
    assert_eq!(issues.len(), 1);
    assert_issue(&issues, "expenses", "even with no flexible spending");
}

#[test]
fn a_target_that_is_no_share_is_refused() {
    let plan = plan_from(RETIREE);
    for target in [0.0, 1.5, -0.1, f64::NAN] {
        let issues = refusal(ceiling_of(&plan, Measure::Success(target)));
        assert_issue(&issues, "success", "share");
    }
    assert!(ceiling_of(&plan, Measure::Success(1.0)).is_ok());
}

#[test]
fn a_cancelled_search_answers_cancelled() {
    let progress = Progress::default();
    progress.cancel();
    for measure in [Measure::Planned, Measure::Success(TARGET)] {
        let found = spending_ceiling(
            &plan_from(RETIREE),
            &TaxTables::embedded(),
            History::embedded(),
            measure,
            &progress,
        );
        assert_eq!(found.unwrap_err(), RunError::Cancelled);
    }
}

#[test]
fn overlay_round_trips_into_the_ceilings_projection() {
    let plan = plan_from(RETIREE);
    let found = ceiling_of(&plan, Measure::Planned).unwrap();
    let overlay = spending_overlay("base.toml", &found.expenses).unwrap();
    assert!(
        overlay.starts_with("schema = 1\nbase = \"base.toml\"\n\n[[expenses]]\nid = \"living\"\n"),
        "{overlay}"
    );
    assert_eq!(overlay.matches("[[expenses]]").count(), 2, "{overlay}");
    let scenario = Scenario::from_toml_str(&overlay)
        .unwrap()
        .expect("a scenario");
    let merged = scenario.apply(toml::from_str(RETIREE).unwrap()).unwrap();
    let merged_plan = Plan::from_toml_table(merged).unwrap();
    assert!(
        merged_plan.validate().is_empty(),
        "{:?}",
        merged_plan.validate()
    );
    let mut applied = plan.clone();
    apply_spending(&mut applied, &found.expenses);
    assert_eq!(applied, merged_plan);
    assert_eq!(
        project(&merged_plan, &TaxTables::embedded()),
        found.judged.projection
    );
}

#[test]
fn a_year_keeps_its_essential_and_flexible_spending_apart() {
    let plan = plan_from(RETIREE);
    let projection = project(&plan, &TaxTables::embedded());
    let roof_year = projection
        .years
        .iter()
        .find(|row| row.year == 2030)
        .unwrap();
    let once = roof_year.expenses_once();
    assert_eq!(
        roof_year.expenses_essential, 12_000,
        "the mortgage never grows"
    );
    assert!(once >= 25_000, "the roof, grown to its year: {once}");
    for row in projection.years.iter().filter(|row| row.year != 2030) {
        assert_eq!(
            row.expenses_once(),
            0,
            "nothing is spent once in {}",
            row.year
        );
        assert!(row.expenses_flexible >= 40_000, "living and travel, grown");
    }
}

#[test]
fn an_essential_expense_spent_once_is_essential_and_nothing_marked_is_flexible() {
    let marked = RETIREE.replace("amount = 25000\n", "amount = 25000\nessential = true\n");
    let projection = project(&plan_from(&marked), &TaxTables::embedded());
    let roof_year = projection
        .years
        .iter()
        .find(|row| row.year == 2030)
        .unwrap();
    assert_eq!(roof_year.expenses_once(), 0);
    assert!(
        roof_year.expenses_essential > 12_000 + 25_000,
        "the mortgage and the roof"
    );

    let unmarked = RETIREE.replace("essential = true\n", "");
    let projection = project(&plan_from(&unmarked), &TaxTables::embedded());
    assert!(
        projection
            .years
            .iter()
            .all(|row| row.expenses_essential == 0)
    );
}
