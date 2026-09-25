//! Claim-age optimizer tests over a zero-inflation plan with a computed
//! Social Security benefit, so every candidate's figures hand-check.

mod common;

use retiretui_engine::optimize::{ClaimSearch, apply_claims, claims_overlay, optimize_claims};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Plan, Scenario};
use retiretui_engine::project::project;

use common::plan_from;

const BASE: &str = r#"
schema = 1

[plan]
name = "claims-base"
start_year = 2026
horizon_age = 90
inflation = 0.0

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1964-06-15
earnings = { 2000 = 60000, 2005 = 70000, 2010 = 80000, 2015 = 90000, 2020 = 100000, 2025 = 110000 }

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 600000

[[income]]
id = "ss"
kind = "social-security"
owner = "me"
start = { age = 67, owner = "me" }

[[expenses]]
id = "living"
amount = 40000
cola = false
"#;

const PARTNER: &str = r#"
[[household.people]]
id = "you"
birth = 1962-01-01
earnings = { 2000 = 50000, 2010 = 60000, 2020 = 70000 }

[[income]]
id = "ss-you"
kind = "social-security"
owner = "you"
start = { age = 70, owner = "you" }
"#;

fn couple() -> String {
    format!(
        "{}{PARTNER}",
        BASE.replace("\"single\"", "\"married-joint\"")
    )
}

fn search(plan: &Plan, incomes: &[&str]) -> Result<ClaimSearch, Vec<String>> {
    let incomes: Vec<String> = incomes.iter().map(|&id| id.to_owned()).collect();
    optimize_claims(plan, &TaxTables::embedded(), &incomes, &[])
        .map_err(|issues| issues.iter().map(ToString::to_string).collect())
}

fn ages(search: &ClaimSearch) -> Vec<Vec<u8>> {
    search
        .candidates
        .iter()
        .map(|candidate| candidate.claims.iter().map(|claim| claim.age).collect())
        .collect()
}

#[test]
fn one_person_gets_the_nine_ages_ranked_best_first() {
    let plan = plan_from(BASE);
    let found = search(&plan, &[]).unwrap();
    let mut seen: Vec<u8> = ages(&found).into_iter().map(|ages| ages[0]).collect();
    seen.sort_unstable();
    assert_eq!(seen, (62..=70).collect::<Vec<u8>>());
    for pair in found.candidates.windows(2) {
        let (better, worse) = (
            pair[0].projection.summary(true),
            pair[1].projection.summary(true),
        );
        assert!(
            better.lifetime_unfunded < worse.lifetime_unfunded
                || (better.lifetime_unfunded == worse.lifetime_unfunded
                    && better.final_net_worth >= worse.final_net_worth),
            "{better:?} before {worse:?}"
        );
    }
    let best = &found.candidates[0];
    assert_eq!(best.claims[0].income, "ss");
    assert_eq!(best.claims[0].owner, "me");
    assert_eq!(found.baseline, project(&plan, &TaxTables::embedded()));
    let stated = found
        .candidates
        .iter()
        .find(|candidate| candidate.claims[0].age == 67)
        .unwrap();
    assert_eq!(
        stated.projection, found.baseline,
        "the stated claim is one of the cells"
    );
}

#[test]
fn a_couple_is_the_joint_grid_clipped_to_reachable_ages() {
    let plan = plan_from(&couple());
    let found = search(&plan, &[]).unwrap();
    assert_eq!(found.incomes, ["ss", "ss-you"]);
    assert_eq!(found.candidates.len(), 9 * 7, "62-70 by 64-70");
    for candidate in &found.candidates {
        assert_eq!(candidate.claims[0].income, "ss");
        assert_eq!(candidate.claims[1].income, "ss-you");
        assert!((64..=70).contains(&candidate.claims[1].age));
    }
    let only = search(&plan, &["ss-you"]).unwrap();
    assert_eq!(only.candidates.len(), 7);
}

#[test]
fn the_search_is_deterministic() {
    let plan = plan_from(&couple());
    assert_eq!(
        ages(&search(&plan, &[]).unwrap()),
        ages(&search(&plan, &[]).unwrap())
    );
}

#[test]
fn a_stated_amount_leaves_nothing_to_search() {
    let stated = BASE.replace("start = { age = 67", "amount = 30000\nstart = { age = 67");
    let plan = plan_from(&stated);
    let refused = search(&plan, &[]).unwrap_err();
    assert_eq!(
        refused,
        [
            "income: no social-security income computes its benefit, and no one without one has an earnings record"
        ]
    );
    let named = search(&plan, &["ss"]).unwrap_err();
    assert_eq!(
        named,
        ["options.incomes[0]: states its amount, so its benefit cannot move"]
    );
}

#[test]
fn named_incomes_are_checked() {
    let plan = plan_from(BASE);
    assert_eq!(
        search(&plan, &["nope"]).unwrap_err(),
        ["options.incomes[0]: unknown income `nope`"]
    );
    assert_eq!(
        search(&plan, &["ss", "ss"]).unwrap_err(),
        ["options.incomes[1]: `ss` is named twice"]
    );
    let salaried = BASE.replace(
        "[[expenses]]",
        "[[income]]\nid = \"pay\"\nkind = \"salary\"\nowner = \"me\"\namount = 1\nend = { age = 63, owner = \"me\" }\n\n[[expenses]]",
    );
    assert_eq!(
        search(&plan_from(&salaried), &["pay"]).unwrap_err(),
        ["options.incomes[0]: must be a social-security income"]
    );
}

#[test]
fn a_person_past_seventy_has_no_age_left() {
    let plan = plan_from(&BASE.replace("birth = 1964-06-15", "birth = 1950-06-15"));
    assert_eq!(
        search(&plan, &[]).unwrap_err(),
        ["income[0].start: no claim age from 76 to 70 falls inside the horizon"]
    );
}

#[test]
fn overlay_round_trips_into_the_winning_projection() {
    let plan = plan_from(&couple());
    let found = search(&plan, &[]).unwrap();
    let best = found.best();
    let text = claims_overlay("base.toml", &found.added, &best.claims).unwrap();
    assert!(
        text.starts_with("schema = 1\nbase = \"base.toml\"\n"),
        "{text}"
    );
    let trigger = best.claims[0].trigger();
    assert_eq!(trigger.age, Some(best.claims[0].age));
    assert_eq!(trigger.owner.as_deref(), Some("me"));
    assert_eq!(
        (trigger.date, trigger.event, trigger.income),
        (None, None, None)
    );
    assert!(text.contains("[[income]]\nid = \"ss\"\n"), "{text}");
    assert!(
        text.contains(&format!(
            "[income.start]\nage = {}\nowner = \"me\"\n",
            best.claims[0].age
        )),
        "{text}"
    );
    let scenario = Scenario::from_toml_str(&text).unwrap().expect("a scenario");
    let merged = scenario.apply(toml::from_str(&couple()).unwrap()).unwrap();
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
}

/// The base with its `social-security` income taken out.
fn without_benefit() -> String {
    BASE.replace(
        "[[income]]\nid = \"ss\"\nkind = \"social-security\"\nowner = \"me\"\nstart = { age = 67, owner = \"me\" }\n",
        "",
    )
}

#[test]
fn a_record_without_an_income_is_searched_under_a_made_up_one() {
    let base = without_benefit();
    let plan = plan_from(&base);
    assert!(plan.income.is_empty());
    let found = search(&plan, &[]).unwrap();
    assert_eq!(found.incomes, ["ss-me"]);
    assert_eq!(found.added.len(), 1);
    assert!(found.added[0].is_derived());
    assert_eq!(found.candidates.len(), 9);
    let best = found.best();

    let mut taken = plan.clone();
    apply_claims(&mut taken, &found.added, &best.claims);
    assert!(taken.validate().is_empty(), "{:?}", taken.validate());
    assert_eq!(project(&taken, &TaxTables::embedded()), best.projection);

    let text = claims_overlay("base.toml", &found.added, &best.claims).unwrap();
    assert!(text.contains("kind = \"social-security\""), "{text}");
    let scenario = Scenario::from_toml_str(&text).unwrap().expect("a scenario");
    let merged = scenario.apply(toml::from_str(&base).unwrap()).unwrap();
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
}

#[test]
fn nothing_is_made_up_without_a_record_or_when_incomes_are_named() {
    let no_record = without_benefit().replace("earnings = {", "# earnings = {");
    assert_eq!(
        search(&plan_from(&no_record), &[]).unwrap_err(),
        [
            "income: no social-security income computes its benefit, and no one without one has an earnings record"
        ]
    );
    let couple = couple().replace(
        "[[income]]\nid = \"ss-you\"\nkind = \"social-security\"\nowner = \"you\"\nstart = { age = 70, owner = \"you\" }\n",
        "",
    );
    let plan = plan_from(&couple);
    assert!(search(&plan, &["ss"]).unwrap().added.is_empty());
    assert_eq!(search(&plan, &[]).unwrap().incomes, ["ss", "ss-you"]);
}

#[test]
fn a_held_id_leaves_a_made_up_income_nowhere_to_go() {
    let held = without_benefit().replace(
        "[[expenses]]",
        "[[income]]\nid = \"ss-me\"\nkind = \"pension\"\nowner = \"me\"\namount = 1\n\n[[expenses]]",
    );
    assert_eq!(
        search(&plan_from(&held), &[]).unwrap_err(),
        ["income: `ss-me` is taken, so me's benefit has no id to be added under"]
    );
}

#[test]
fn a_held_person_keeps_their_claim_and_the_plan_s_own_ages_are_reported() {
    let plan = plan_from(&couple());
    let all = search(&plan, &[]).unwrap();
    assert_eq!(all.current, [Some(67), Some(70)]);
    let held = optimize_claims(&plan, &TaxTables::embedded(), &[], &["you".to_owned()]).unwrap();
    assert_eq!(held.incomes, ["ss"]);
    assert_eq!(held.candidates.len(), 9);
    let made_up = plan_from(&without_benefit());
    let refused =
        optimize_claims(&made_up, &TaxTables::embedded(), &[], &["me".to_owned()]).unwrap_err();
    assert_eq!(refused.len(), 1, "a held person gets no made-up income");
}
