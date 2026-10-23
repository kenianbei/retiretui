//! The spending estimate: a bank's exports read and merged, the window's
//! spending sorted, annualized and deflated, and the proposals applied.

use retiretui_engine::estimate::{
    BankError, Category, Cents, Estimate, Patterns, Phases, Proposal, Why, apply, estimate, merge,
    read,
};
use retiretui_engine::plan::Plan;

const CHASE: &str = include_str!("fixtures/bank/chase.csv");
const VISA: &str = include_str!("fixtures/bank/visa.csv");
const OFX: &str = include_str!("fixtures/bank/checking.ofx");
const QFX: &str = include_str!("fixtures/bank/checking.qfx");

fn plan(expenses: &str) -> Plan {
    Plan::from_toml_str(&format!(
        r#"
schema = 1

[plan]
start_year = 2026
horizon_age = 90
inflation = 0.03

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1960-06-15

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 100000
{expenses}
"#
    ))
    .unwrap()
}

const GROCERIES: &str = r#"
[[expenses]]
id = "groceries"
amount = 6000
"#;

const PHASED_HOUSING: &str = r#"
[[expenses]]
id = "housing"
amount = 12000

[[expenses.phases]]
from = { date = 2026-06-01 }
amount = 15000
"#;

fn estimated(plan: &Plan, csv: &str) -> Estimate {
    let rows = read("x.csv", csv).unwrap();
    estimate(plan, &rows, &Patterns::embedded())
}

fn proposal_amount(estimate: &Estimate, category: Category) -> Option<i64> {
    estimate
        .proposals
        .iter()
        .find(|p| gathers(p, category))
        .map(|p| p.amount)
}

fn gathers(proposal: &Proposal, category: Category) -> bool {
    proposal
        .categories
        .iter()
        .any(|each| each.category == category)
}

/// Each proposal's id, the categories it gathers, what the unknown payees
/// bring it, its amount, and whether it is the bucket.
fn landed(found: &Estimate) -> Vec<(&str, Vec<Category>, i64, i64, bool)> {
    found
        .proposals
        .iter()
        .map(|p| {
            let categories = p.categories.iter().map(|it| it.category).collect();
            (p.id.as_str(), categories, p.unknown, p.amount, p.is_bucket)
        })
        .collect()
}

const GROCERY_AND_MISC: &str = r#"
[[expenses]]
id = "grocery"
amount = 6000

[[expenses]]
id = "misc"
name = "Misc"
amount = 3000
"#;

/// Groceries, dining by the bank's own column, a subscription, and a payee
/// nothing names: $300, $100, $20 and $30 in one month.
const MIXED: &str = "Date,Description,Category,Amount\n2026-10-01,SAFEWAY,,-300.00\n2026-10-02,LUIGI'S,Food & Drink,-100.00\n2026-10-03,NETFLIX,,-20.00\n2026-10-04,SOMEONE,,-30.00\n";

#[test]
fn a_csv_sorts_its_rows_and_names_the_expense_each_sets() {
    let plan = plan(GROCERIES);
    let found = estimated(&plan, CHASE);
    assert_eq!(found.months, 1);
    assert_eq!(found.through.unwrap().0.to_string(), "2026-10-15");
    let [groceries] = &found.proposals[..] else {
        panic!("{:?}", found.proposals)
    };
    assert_eq!(
        (groceries.id.as_str(), groceries.is_new),
        ("groceries", false)
    );
    assert_eq!(
        groceries.name, "groceries",
        "the expense as the plan names it"
    );
    assert_eq!(groceries.amount, 142 * 12 + 2);
    assert_eq!(groceries.total, 142);
    assert_eq!(groceries.payees, vec![("SAFEWAY".to_owned(), 142)]);
    assert_eq!(found.income, 4213 * 12 - 5);
    assert_eq!(found.transfers, -(1850 + 120) * 12);
    assert!(found.unknown.is_empty());
}

#[test]
fn a_headerless_csv_is_refused_with_its_first_row() {
    assert_eq!(
        read("visa.csv", VISA),
        Err(BankError::Unreadable {
            name: "visa.csv".to_owned(),
            header: "10/03/26,SAFEWAY #1234 SAN JOSE,142.17,".to_owned(),
        })
    );
}

#[test]
fn the_same_statement_in_both_ofx_forms_merges_to_its_rows() {
    let mut rows = read("checking.ofx", OFX).unwrap();
    assert_eq!(rows.len(), 2);
    merge(&mut rows, read("checking.qfx", QFX).unwrap());
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].payee, "CITY OF SAN JOSE WATER");
    assert_eq!(rows[1].amount, Cents(-7122));
    assert_eq!(rows[1].bank_id.as_deref(), Some("20261018-c3"));
    assert_eq!(
        read("x.ofx", "nothing").unwrap_err(),
        BankError::NotOfx("x.ofx".to_owned())
    );
}

#[test]
fn twins_in_one_file_both_stay_and_another_files_twin_answers_for_one() {
    let mut rows = read("chase.csv", CHASE).unwrap();
    let atm = |rows: &[retiretui_engine::estimate::Row]| {
        rows.iter().filter(|r| r.payee == "ATM WITHDRAWAL").count()
    };
    assert_eq!(atm(&rows), 2);
    let again = "Date,Description,Amount\n10/07/2026,ATM WITHDRAWAL,-60.00\n10/07/2026,ATM WITHDRAWAL,-60.00\n10/07/2026,ATM WITHDRAWAL,-60.00\n";
    merge(&mut rows, read("again.csv", again).unwrap());
    assert_eq!(atm(&rows), 3);
}

#[test]
fn each_month_is_deflated_by_its_year_and_a_gap_still_counts() {
    let plan = plan(GROCERIES);
    let later = "Date,Description,Amount\n2026-12-01,SAFEWAY,-100.00\n2027-01-05,SAFEWAY,-103.00\n";
    let found = estimated(&plan, later);
    assert_eq!(found.months, 2);
    assert_eq!(proposal_amount(&found, Category::Groceries), Some(1200));

    let gap = "Date,Description,Amount\n2026-01-10,SAFEWAY,-120.00\n2026-03-10,SAFEWAY,-120.00\n";
    let found = estimated(&plan, gap);
    assert_eq!(found.months, 3);
    assert_eq!(proposal_amount(&found, Category::Groceries), Some(960));

    let long = "Date,Description,Amount\n2025-01-10,SAFEWAY,-9999.00\n2025-06-10,SAFEWAY,-120.00\n2026-05-10,SAFEWAY,-120.00\n";
    let found = estimated(&plan, long);
    assert_eq!(found.months, 12);
    assert_eq!(found.proposals[0].rows, 2);
}

#[test]
fn a_banks_own_category_wins_and_a_payee_nothing_names_is_listed() {
    let plan = plan(GROCERIES);
    let csv = "Date,Description,Category,Amount\n2026-10-01,SAFEWAY,Food & Drink,-50.00\n2026-10-02,SOMEONE,,-30.00\n2026-10-03,SOMEONE,,-30.00\n";
    let found = estimated(&plan, csv);
    assert_eq!(proposal_amount(&found, Category::Dining), Some(600));
    assert_eq!(proposal_amount(&found, Category::Groceries), None);
    let [unknown] = &found.unknown[..] else {
        panic!()
    };
    assert_eq!(
        (unknown.payee.as_str(), unknown.amount, unknown.rows),
        ("SOMEONE", 720, 2)
    );
    let dining = &found.proposals[0];
    assert!(dining.is_new);
    assert_eq!(dining.name, "Dining");
    assert!(found.proposals[1].is_bucket && found.proposals[1].unknown == 720);
}

#[test]
fn applying_sets_the_matched_expense_and_adds_the_rest() {
    let mut plan = plan(&format!("{GROCERIES}\n[estimate]\nlean = \"new\"\n"));
    let csv = "Date,Description,Amount\n2026-10-01,SAFEWAY,-500.00\n2026-10-02,NETFLIX,-20.00\n2026-10-03,HOME DEPOT,-1200.00\n";
    let mut found = estimated(&plan, csv);
    found
        .proposals
        .iter_mut()
        .find(|p| gathers(p, Category::Shopping))
        .unwrap()
        .one_time = true;
    apply(&mut plan, &found);
    let ids: Vec<&str> = plan.expenses.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["groceries", "subscriptions", "shopping"]);
    assert_eq!(plan.expenses[0].amount, 6000);
    assert_eq!(plan.expenses[1].amount, 240);
    assert_eq!(plan.expenses[1].name.as_deref(), Some("Subscriptions"));
    assert_eq!(plan.expenses[2].amount, 1200);
    assert_eq!(
        plan.expenses[2]
            .on
            .as_ref()
            .unwrap()
            .date
            .unwrap()
            .0
            .to_string(),
        "2026-10-03"
    );
    assert!(plan.validate().is_empty(), "{:?}", plan.validate());
}

#[test]
fn a_phased_expense_is_set_in_the_phase_in_force_scaled_or_whole() {
    let csv = "Date,Description,Amount\n2026-10-01,ROCKET MORTGAGE,-1500.00\n";
    for (phases, base, phase) in [
        (Phases::This, 12000, 18000),
        (Phases::Scaled, 14400, 18000),
        (Phases::All, 18000, 18000),
    ] {
        let mut plan = plan(PHASED_HOUSING);
        let mut found = estimated(&plan, csv);
        found.proposals[0].phases = phases;
        apply(&mut plan, &found);
        assert_eq!(
            (plan.expenses[0].amount, plan.expenses[0].phases[0].amount),
            (base, phase),
            "{phases:?}"
        );
    }
    let before = "Date,Description,Amount\n2026-03-01,ROCKET MORTGAGE,-1500.00\n";
    let mut plan = plan(PHASED_HOUSING);
    let found = estimated(&plan, before);
    apply(&mut plan, &found);
    assert_eq!(
        (plan.expenses[0].amount, plan.expenses[0].phases[0].amount),
        (18000, 15000)
    );
}

#[test]
fn leaning_on_existing_folds_each_category_into_its_family_else_the_bucket() {
    let plan = plan(&format!(
        "{GROCERY_AND_MISC}\n[estimate]\nlean = \"existing\"\n"
    ));
    let found = estimated(&plan, MIXED);
    assert_eq!(
        landed(&found),
        [
            (
                "grocery",
                vec![Category::Groceries, Category::Dining],
                0,
                4800,
                false
            ),
            ("misc", vec![Category::Subscriptions], 360, 600, true),
        ]
    );
    assert_eq!(
        found.unknown.len(),
        1,
        "a payee in the bucket is still listed"
    );
    let whys: Vec<Why> = found
        .proposals
        .iter()
        .flat_map(|p| p.categories.iter().map(|it| it.why))
        .collect();
    assert_eq!(whys, [Why::Named, Why::Family, Why::Bucket]);
}

#[test]
fn an_assignment_wins_over_the_family_and_a_name_match() {
    let assigned = format!(
        "{GROCERY_AND_MISC}\n[estimate]\nlean = \"existing\"\n\n[[estimate.assign]]\ncategory = \"dining\"\nexpense = \"misc\"\n"
    );
    let found = estimated(&plan(&assigned), MIXED);
    let misc = &found.proposals[1];
    assert_eq!(misc.id, "misc");
    assert!(gathers(misc, Category::Dining) && !gathers(&found.proposals[0], Category::Dining));
}

#[test]
fn leaning_large_proposes_a_new_expense_only_past_the_share() {
    let with = |share: &str| {
        let lean = format!("{GROCERY_AND_MISC}\n[estimate]\nlean = \"large\"\n{share}");
        landed(&estimated(&plan(&lean), MIXED))
            .into_iter()
            .map(|(id, ..)| id.to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(with(""), ["grocery", "misc", "dining"], "dining is 22%");
    assert_eq!(with("large_share = 0.5\n"), ["grocery", "misc"]);
    let always = format!("{GROCERY_AND_MISC}\n[estimate]\nlean = \"new\"\n");
    let ids: Vec<String> = landed(&estimated(&plan(&always), MIXED))
        .into_iter()
        .map(|(id, ..)| id.to_owned())
        .collect();
    assert_eq!(ids, ["grocery", "misc", "dining", "subscriptions"]);
}

#[test]
fn a_plan_without_a_bucket_is_proposed_one_and_the_named_one_wins() {
    let mut taken = plan(&format!("{GROCERIES}\n[estimate]\nlean = \"existing\"\n"));
    let found = estimated(&taken, MIXED);
    let bucket = found.proposals.last().unwrap();
    assert_eq!((bucket.id.as_str(), bucket.is_new), ("other", true));
    assert!(bucket.is_bucket && gathers(bucket, Category::Subscriptions));
    apply(&mut taken, &found);
    assert_eq!(
        taken.expenses.last().unwrap().name.as_deref(),
        Some("Other")
    );
    assert!(taken.validate().is_empty(), "{:?}", taken.validate());

    let named = format!(
        "{GROCERY_AND_MISC}\n[[expenses]]\nid = \"other-costs\"\namount = 0\n\n[estimate]\nlean = \"existing\"\nbucket = \"other-costs\"\n"
    );
    let found = estimated(&plan(&named), MIXED);
    assert_eq!(
        found.proposals.iter().find(|p| p.is_bucket).unwrap().id,
        "other-costs"
    );
}

#[test]
fn a_banks_type_says_what_moved_and_what_came_in() {
    let csv = "Date,Description,Type,Amount\n2026-10-01,CARD THANK YOU,Payment,500.00\n2026-10-02,MONTHLY INTEREST,INTADJUST,1.25\n2026-10-03,SOMEONE,TRANSFER,-200.00\n";
    let rows = read("x.csv", csv).unwrap();
    let kinds: Vec<_> = rows.iter().map(|row| row.kind).collect();
    assert_eq!(
        kinds,
        [
            Some(Category::Transfer),
            Some(Category::Income),
            Some(Category::Transfer)
        ]
    );
    let found = estimate(&plan(GROCERIES), &rows, &Patterns::embedded());
    assert!(found.unknown.is_empty() && found.proposals.is_empty());
    assert_eq!(found.transfers, 300 * 12);
}

#[test]
fn the_estimate_table_is_checked_against_the_plans_expenses() {
    let bad = format!(
        "{GROCERIES}\n[estimate]\nbucket = \"nowhere\"\nlarge_share = 0.0\n\n[[estimate.assign]]\ncategory = \"income\"\nexpense = \"groceries\"\n\n[[estimate.assign]]\ncategory = \"dining\"\nexpense = \"gone\"\n\n[[estimate.assign]]\ncategory = \"dining\"\nexpense = \"groceries\"\n"
    );
    let paths: Vec<String> = plan(&bad).validate().into_iter().map(|i| i.path).collect();
    assert_eq!(
        paths,
        [
            "estimate.bucket",
            "estimate.large_share",
            "estimate.assign[0].category",
            "estimate.assign[1].expense",
            "estimate.assign[2].category",
        ]
    );
}
