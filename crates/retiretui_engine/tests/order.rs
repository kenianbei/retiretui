//! Withdrawal-order optimizer tests over a zero-inflation plan of one
//! retiree, whose accounts differ by class and by what they earn.

mod common;

use retiretui_engine::market::{Progress, RunError};
use retiretui_engine::optimize::{
    OrderSearch, apply_order, optimize_order, order_overlay, rank_key,
};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::TreatmentClass::{self, Deferred, Hsa, Roth, Taxable};
use retiretui_engine::plan::{Plan, Scenario};
use retiretui_engine::project::project;

use common::{assert_issue, plan_from};

const RETIREE: &str = include_str!("fixtures/order-plan.toml");

/// The fixture's retiree holding `cash` and drained in `order`.
fn retiree(order: &str, cash: i64) -> String {
    RETIREE
        .replacen(r#"["roth", "deferred", "taxable"]"#, order, 1)
        .replacen("balance = 200000", &format!("balance = {cash}"), 1)
}

fn search(plan: &Plan) -> OrderSearch {
    optimize_order(plan, &TaxTables::embedded(), &Progress::default()).unwrap()
}

fn orders(found: &OrderSearch) -> Vec<Vec<TreatmentClass>> {
    found
        .candidates
        .iter()
        .map(|candidate| candidate.order.clone())
        .collect()
}

#[test]
fn an_order_that_ends_with_more_than_the_plans_own_ranks_first() {
    let plan = plan_from(&retiree(r#"["roth", "deferred", "taxable"]"#, 200_000));
    let found = search(&plan);
    assert_eq!(found.baseline, project(&plan, &TaxTables::embedded()));
    assert_eq!(found.candidates.len(), 6, "{:?}", orders(&found));
    assert_eq!(
        found.best().order,
        [Taxable, Roth, Deferred],
        "the cash earns nothing, and a Roth dollar is spent without the tax a deferred one costs"
    );
    assert!(rank_key(&found.best().projection) < rank_key(&found.baseline));
    for pair in found.candidates.windows(2) {
        assert!(rank_key(&pair[0].projection) <= rank_key(&pair[1].projection));
    }
    let own = found
        .candidates
        .iter()
        .find(|candidate| candidate.order == plan.plan.withdrawal_order)
        .expect("the plan's own order is a candidate");
    assert_eq!(own.projection, found.baseline);
}

#[test]
fn a_class_the_plan_skips_is_in_no_order() {
    let found = search(&plan_from(&retiree(r#"["deferred", "taxable"]"#, 200_000)));
    assert_eq!(
        orders(&found),
        [[Taxable, Deferred], [Deferred, Taxable]],
        "the Roth account is held and stays undrained"
    );
}

#[test]
fn a_class_held_only_under_a_drain_priority_keeps_its_place_after_the_ordered() {
    let text = retiree(r#"["roth", "taxable", "deferred"]"#, 200_000)
        .replace("roth = true\n", "roth = true\ndrain_priority = 1\n");
    let found = search(&plan_from(&text));
    let mut tried = orders(&found);
    tried.sort();
    assert_eq!(
        tried,
        [[Deferred, Taxable, Roth], [Roth, Taxable, Deferred]],
        "taxable before deferred projects as the plan does, and is said as the plan says it"
    );
}

#[test]
fn a_listed_class_with_no_account_follows_the_ordered_ones() {
    let plan = plan_from(&retiree(
        r#"["hsa", "roth", "deferred", "taxable"]"#,
        200_000,
    ));
    let found = search(&plan);
    assert_eq!(found.candidates.len(), 6, "{:?}", orders(&found));
    for candidate in &found.candidates {
        if candidate.order == plan.plan.withdrawal_order {
            assert_eq!(candidate.projection, found.baseline);
        } else {
            assert_eq!(candidate.order.last(), Some(&Hsa), "{:?}", candidate.order);
        }
    }
    assert_eq!(found.best().order, [Taxable, Roth, Deferred, Hsa]);
}

#[test]
fn orders_that_project_alike_are_one_candidate_named_nearest_the_plans_own() {
    let rich = |order| search(&plan_from(&retiree(order, 1_000_000)));
    let own_tied = rich(r#"["taxable", "roth", "deferred"]"#);
    assert!(
        orders(&own_tied).contains(&vec![Taxable, Roth, Deferred]),
        "the cash outlasts the plan, so what follows it is never reached"
    );
    assert!(!orders(&own_tied).contains(&vec![Taxable, Deferred, Roth]));

    let other_tied = rich(r#"["roth", "taxable", "deferred"]"#);
    assert!(orders(&other_tied).contains(&vec![Taxable, Roth, Deferred]));
    assert!(!orders(&other_tied).contains(&vec![Taxable, Deferred, Roth]));
    assert!(other_tied.candidates.len() < 6);
}

#[test]
fn the_plans_own_order_ranks_ahead_of_another_that_ends_with_as_much() {
    let text = retiree(r#"["roth", "taxable"]"#, 200_000)
        .replace("expected_return = 0.05", "expected_return = 0.0");
    let plan = plan_from(&text);
    let found = search(&plan);
    assert_eq!(orders(&found), [[Roth, Taxable], [Taxable, Roth]]);
    assert_ne!(
        found.candidates[0].projection,
        found.candidates[1].projection
    );
    assert_eq!(
        rank_key(&found.candidates[0].projection),
        rank_key(&found.candidates[1].projection),
        "neither account earns or is taxed, so both orders end with the same"
    );
}

#[test]
fn a_plan_with_one_class_to_order_is_refused() {
    let plan = plan_from(&retiree(r#"["taxable", "hsa"]"#, 2_000_000));
    let RunError::Refused(issues) =
        optimize_order(&plan, &TaxTables::embedded(), &Progress::default()).unwrap_err()
    else {
        panic!("nothing cancels it");
    };
    assert_eq!(issues.len(), 1);
    assert_issue(&issues, "plan.withdrawal_order", "nothing to order");
}

#[test]
fn a_cancelled_search_answers_cancelled() {
    let progress = Progress::default();
    progress.cancel();
    let plan = plan_from(&retiree(r#"["roth", "deferred", "taxable"]"#, 200_000));
    assert_eq!(
        optimize_order(&plan, &TaxTables::embedded(), &progress).unwrap_err(),
        RunError::Cancelled
    );
}

#[test]
fn overlay_round_trips_into_the_winning_projection() {
    let text = retiree(r#"["hsa", "roth", "deferred", "taxable"]"#, 200_000);
    let plan = plan_from(&text);
    let found = search(&plan);
    let best = found.best();
    let overlay = order_overlay("base.toml", &best.order).unwrap();
    assert!(
        overlay.starts_with("schema = 1\nbase = \"base.toml\"\n\n[plan]\nwithdrawal_order = ["),
        "{overlay}"
    );
    let scenario = Scenario::from_toml_str(&overlay)
        .unwrap()
        .expect("a scenario");
    let merged = scenario.apply(toml::from_str(&text).unwrap()).unwrap();
    let merged_plan = Plan::from_toml_table(merged).unwrap();
    assert!(
        merged_plan.validate().is_empty(),
        "{:?}",
        merged_plan.validate()
    );
    assert_eq!(
        project(&merged_plan, &TaxTables::embedded()),
        best.projection
    );
    let mut applied = plan.clone();
    apply_order(&mut applied, &best.order);
    assert_eq!(applied, merged_plan);
}
