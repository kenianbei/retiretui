//! The early-withdrawal penalty: who pays it, and what frees a plan from it.

mod common;

use std::ops::RangeInclusive;

use retiretui_engine::plan::{Dollars, Plan};
use retiretui_engine::project::Projection;

use common::{assert_issue, head, issues, plan_from, run};

const PENALTY_RATE: f64 = 0.10;

/// A plan from 2026 without inflation, for someone born in June of
/// `birth_year`.
fn born_in(birth_year: i16, body: &str) -> String {
    format!(
        r#"
schema = 1

[plan]
start_year = 2026
horizon_age = 70
inflation = 0.0

[household]
filing = "single"

[[household.people]]
id = "me"
birth = {birth_year}-06-15
{body}
"#
    )
}

/// Someone who lives on a 401(k) alone, its table ended by `stated`.
fn living_on_a_plan(birth_year: i16, stated: &str) -> String {
    let body = format!(
        r#"
[[expenses]]
id = "living"
amount = 40000

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 900000
{stated}"#
    );
    born_in(birth_year, &body)
}

/// Someone who moves 100,000 out of `account` into a brokerage in the first
/// year, with cash enough to pay its tax.
fn moving_to_a_brokerage(birth_year: i16, account: &str) -> String {
    let body = format!(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 200000

[[accounts]]
id = "brokerage"
kind = "brokerage"
owner = "me"
balance = 0

[[transfers]]
id = "cash-out"
from = "k"
to = "brokerage"
amount = 100000
on = {{ date = 2026-01-01 }}

[[accounts]]
id = "k"
owner = "me"
balance = 400000
{account}"#
    );
    born_in(birth_year, &body)
}

/// The years among `years` that pay the penalty, every one of which draws on
/// `account` and pays a tenth of that draw or nothing.
#[track_caller]
fn penalized_years(projection: &Projection, account: &str, years: RangeInclusive<i16>) -> Vec<i16> {
    let is_penalized = |year: &i16| {
        let row = projection.row(*year).unwrap();
        let drawn = row.withdrawals.get(account).copied().unwrap_or(0);
        assert!(drawn > 0, "{year} draws nothing on {account}");
        let owed = (drawn as f64 * PENALTY_RATE).round() as Dollars;
        let paid = row.taxes.penalty;
        assert!(paid == 0 || paid == owed, "{year} pays {paid} on {drawn}");
        paid > 0
    };
    years.filter(is_penalized).collect()
}

#[test]
fn early_deferred_draw_pays_penalty_only_as_last_resort() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 30000

[[accounts]]
id = "401k"
kind = "401k"
owner = "me"
balance = 500000

[[expenses]]
id = "living"
amount = 50000
"#,
    );
    // Cash covers 30,000 penalty-free; the rest comes from the 401(k) at 46.
    let projection = run(&plan);
    assert_eq!(projection.years[0].withdrawals["cash"], 30_000);
    assert_eq!(penalized_years(&projection, "401k", 2026..=2026), [2026]);
    let exempt = run(&plan.replace("kind = \"401k\"", "kind = \"457b\""));
    assert!(penalized_years(&exempt, "401k", 2026..=2026).is_empty());
}

#[test]
fn a_plan_is_penalized_until_the_year_its_owner_turns_59_and_a_half() {
    // Born June 1970: 59 and a half in December 2029.
    let projection = run(&living_on_a_plan(1970, ""));
    assert_eq!(
        penalized_years(&projection, "k", 2026..=2032),
        [2026, 2027, 2028]
    );
}

#[test]
fn a_plan_left_at_55_or_later_is_free_from_the_year_it_is_left() {
    // Born June 1970: 58 in 2028, the year the job is left.
    let left_at_58 = living_on_a_plan(1970, "separated = { age = 58, owner = \"me\" }");
    let projection = run(&left_at_58);
    assert_eq!(penalized_years(&projection, "k", 2026..=2032), [2026, 2027]);
}

#[test]
fn the_year_its_owner_turns_55_is_the_first_a_plan_can_be_left_in() {
    // Born June 1971: 55 in 2026, 59 and a half in December 2030.
    let left_at_55 = living_on_a_plan(1971, "separated = { date = 2026-03-01 }");
    assert!(penalized_years(&run(&left_at_55), "k", 2026..=2031).is_empty());
    let left_at_54 = living_on_a_plan(1971, "separated = { date = 2025-12-31 }");
    let projection = run(&left_at_54);
    assert_eq!(
        penalized_years(&projection, "k", 2026..=2031),
        [2026, 2027, 2028, 2029]
    );
}

#[test]
fn a_job_left_before_the_plan_starts_frees_its_plan_from_the_first_year() {
    // Born June 1970: 55 in 2025.
    let already_left = living_on_a_plan(1970, "separated = { date = 2025-08-01 }");
    assert!(penalized_years(&run(&already_left), "k", 2026..=2030).is_empty());
}

#[test]
fn a_freed_plan_drains_in_its_place_in_the_withdrawal_order() {
    // Born June 1970: 56 in 2026.
    let deferred_first = |stated: &str| {
        let plan = living_on_a_plan(1970, stated)
            .replacen("balance = 0", "balance = 500000", 1)
            .replace(
                "inflation = 0.0",
                "inflation = 0.0\nwithdrawal_order = [\"deferred\", \"taxable\"]",
            );
        run(&plan).years[0].withdrawals.clone()
    };
    let penalized = deferred_first("");
    assert!(penalized.contains_key("cash") && !penalized.contains_key("k"));
    let freed = deferred_first("separated = { date = 2026-03-01 }");
    assert!(freed.contains_key("k") && !freed.contains_key("cash"));
}

#[test]
fn a_public_safety_plan_is_freed_from_50() {
    // Born June 1975: 51 in 2026, 59 and a half in December 2034.
    let left_at_51 = "separated = { date = 2026-03-01 }";
    let unflagged = run(&living_on_a_plan(1975, left_at_51));
    let until_59_and_a_half: Vec<i16> = (2026..=2033).collect();
    assert_eq!(
        penalized_years(&unflagged, "k", 2026..=2035),
        until_59_and_a_half
    );
    let flagged = format!("{left_at_51}\npublic_safety = true");
    let left_at_51 = run(&living_on_a_plan(1975, &flagged));
    assert!(penalized_years(&left_at_51, "k", 2026..=2035).is_empty());
    // Born June 1977: 49 in 2026.
    let left_at_49 = run(&living_on_a_plan(1977, &flagged));
    assert_eq!(
        penalized_years(&left_at_49, "k", 2026..=2028),
        [2026, 2027, 2028]
    );
}

#[test]
fn money_rolled_into_an_ira_is_penalized_again() {
    // Born June 1970: the 401(k) is freed in 2026, and emptied into an IRA.
    let rolled = living_on_a_plan(
        1970,
        r#"separated = { date = 2026-03-01 }

[[accounts]]
id = "ira"
kind = "ira"
owner = "me"
balance = 0

[[transfers]]
id = "rollover"
from = "k"
to = "ira"
on = { date = 2026-01-01 }"#,
    );
    let projection = run(&rolled);
    assert_eq!(
        penalized_years(&projection, "ira", 2026..=2030),
        [2026, 2027, 2028]
    );
}

#[test]
fn a_transfer_to_a_taxable_account_pays_the_penalty_a_withdrawal_would() {
    let penalty_of = |birth_year: i16, account: &str| {
        let projection = run(&moving_to_a_brokerage(birth_year, account));
        let first = &projection.years[0];
        assert!(
            !first.withdrawals.contains_key("k"),
            "the tax is paid from cash"
        );
        first.taxes.penalty
    };
    // Born June 1976: 50 in 2026. A tenth of the 100,000 moved.
    assert_eq!(penalty_of(1976, "kind = \"401k\""), 10_000);
    // A quarter of the balance is after-tax basis, so a quarter of what
    // moves is untaxed: a tenth of 75,000.
    assert_eq!(penalty_of(1976, "kind = \"401k\"\nbasis = 100000"), 7_500);
    assert_eq!(penalty_of(1976, "kind = \"ira\""), 10_000);
    assert_eq!(penalty_of(1976, "kind = \"457b\""), 0);
    // Born June 1966: 60 in 2026.
    assert_eq!(penalty_of(1966, "kind = \"401k\""), 0);
    // Born June 1970: 56 in 2026, the year the job is left.
    let freed = "kind = \"401k\"\nseparated = { date = 2026-03-01 }";
    assert_eq!(penalty_of(1970, freed), 0);
}

#[test]
fn only_a_tax_deferred_workplace_plan_states_when_its_job_is_left() {
    let stating = |account: &str| {
        issues(&living_on_a_plan(
            1970,
            &format!("\n[[accounts]]\nid = \"other\"\nowner = \"me\"\nbalance = 0\n{account}"),
        ))
    };
    let left = "separated = { date = 2026-03-01 }";
    for kind in ["401k", "403b", "414k"] {
        let found = stating(&format!("kind = \"{kind}\"\n{left}"));
        assert!(found.is_empty(), "{kind}: {found:?}");
    }
    for kind in [
        "457b",
        "ira",
        "sep-ira",
        "simple-ira",
        "hsa",
        "brokerage",
        "cash",
    ] {
        let found = stating(&format!("kind = \"{kind}\"\n{left}"));
        assert_issue(&found, "accounts[2].separated", "is freed by leaving a job");
    }
    let roth = stating(&format!("kind = \"401k\"\nroth = true\n{left}"));
    assert_issue(&roth, "accounts[2].separated", "is freed by leaving a job");
    let flag_alone = stating("kind = \"401k\"\npublic_safety = true");
    assert_issue(
        &flag_alone,
        "accounts[2].public_safety",
        "state when the job is left",
    );
    let flagged = stating(&format!("kind = \"401k\"\n{left}\npublic_safety = true"));
    assert!(flagged.is_empty(), "{flagged:?}");
    let dangling = stating("kind = \"401k\"\nseparated = { event = \"ghost\" }");
    assert_issue(&dangling, "accounts[2].separated", "unknown event `ghost`");
    let half_made = stating("kind = \"401k\"\nseparated = { age = 56 }");
    assert_eq!(half_made[0].path, "accounts[2].separated", "{half_made:?}");
}

#[test]
fn a_separation_is_written_back_only_where_it_is_stated() {
    let stated = "separated = { date = 2026-03-01 }\npublic_safety = true";
    let plan = plan_from(&living_on_a_plan(1975, stated));
    let written = plan.to_toml_string().unwrap();
    assert!(written.contains("public_safety = true"), "{written}");
    assert_eq!(Plan::from_toml_str(&written).unwrap(), plan);
    let unstated = plan_from(&living_on_a_plan(1975, ""));
    let written = unstated.to_toml_string().unwrap();
    assert!(!written.contains("separated"), "{written}");
    assert!(!written.contains("public_safety"), "{written}");
}
