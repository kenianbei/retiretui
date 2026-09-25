//! Semantic validation tests for the plan document.

mod common;

use retiretui_engine::plan::Plan;

use common::{assert_issue, issues};

const BASE: &str = r#"
schema = 1

[plan]
start_year = 2026
horizon_age = 90
inflation = 0.025

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1980-01-01

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 1000
"#;

fn with(extra: &str) -> String {
    format!("{BASE}\n{extra}")
}

#[test]
fn minimal_plan_is_valid() {
    assert!(issues(BASE).is_empty(), "{:?}", issues(BASE));
}

#[test]
fn wrong_schema_version() {
    let text = BASE.replace("schema = 1", "schema = 99");
    assert_issue(&issues(&text), "schema", "unsupported schema version");
}

#[test]
fn filing_status_must_match_people() {
    let text = BASE.replace("filing = \"single\"", "filing = \"married-joint\"");
    assert_issue(&issues(&text), "household.people", "exactly 2");
}

#[test]
fn duplicate_ids_are_reported() {
    let text = with("[[accounts]]\nid = \"cash\"\nkind = \"cash\"\nowner = \"me\"\nbalance = 1\n");
    assert_issue(&issues(&text), "accounts[1].id", "duplicate");
}

#[test]
fn duplicate_expense_ids_are_reported() {
    let text = with(
        "[[expenses]]\nid = \"living\"\namount = 1\n[[expenses]]\nid = \"living\"\namount = 2\n",
    );
    assert_issue(&issues(&text), "expenses[1].id", "duplicate");
    let text = with("[[expenses]]\nid = \"\"\namount = 1\n");
    assert_issue(&issues(&text), "expenses[0].id", "is required");
}

#[test]
fn an_item_known_only_by_its_name_is_asked_for_an_id() {
    let text = with("[[expenses]]\nname = \"living\"\namount = 1\n");
    assert_issue(
        &issues(&text),
        "expenses[0].id",
        "`name` is only what it is shown as",
    );
    let text = with("[[cliffs]]\nname = \"aca\"\nmagi_over = 1\ncost = 1\n");
    assert_issue(&issues(&text), "cliffs[0].id", "is required");
    let text = with("[[income]]\nkind = \"other\"\nowner = \"me\"\namount = 1\n");
    assert_issue(&issues(&text), "income[0].id", "is required");
}

#[test]
fn cliff_and_medicare_rules() {
    let cliff = "[[cliffs]]\nid = \"aca\"\nmagi_over = 90000\ncost = 12000\n";
    let text = with(&format!("{cliff}{cliff}"));
    assert_issue(&issues(&text), "cliffs[1].id", "duplicate");
    let text = with("[[cliffs]]\nid = \"aca\"\nmagi_over = -1\ncost = 12000\n");
    assert_issue(&issues(&text), "cliffs[0].magi_over", "negative");
    let text = with("[[cliffs]]\nid = \"aca\"\nmagi_over = 90000\ncost = -1\n");
    assert_issue(&issues(&text), "cliffs[0].cost", "negative");
    let text =
        with("[[cliffs]]\nid = \"aca\"\nmagi_over = 1\ncost = 1\nstart = { event = \"ghost\" }\n");
    assert_issue(&issues(&text), "cliffs[0].start", "unknown event");
    let text = with("[medicare]\nprior_magi = [1, 2, 3]\n");
    assert_issue(&issues(&text), "medicare.prior_magi", "at most two");
    let text = with("[medicare]\nprior_magi = [-5]\n");
    assert_issue(&issues(&text), "medicare.prior_magi", "negative");
}

#[test]
fn duplicate_flow_ids_are_reported() {
    let accounts = "[[accounts]]\nid = \"other\"\nkind = \"cash\"\nowner = \"me\"\nbalance = 1\n";
    let transfer = "[[transfers]]\nid = \"move\"\nfrom = \"cash\"\nto = \"other\"\non = { date = 2030-01-01 }\n";
    let text = with(&format!("{accounts}{transfer}{transfer}"));
    assert_issue(&issues(&text), "transfers[1].id", "duplicate");
    let conversions = "[[accounts]]\nid = \"k\"\nkind = \"401k\"\nowner = \"me\"\nbalance = 10\n\
        [[accounts]]\nid = \"r\"\nkind = \"ira\"\nroth = true\nowner = \"me\"\nbalance = 0\n\
        [[conversions]]\nid = \"ladder\"\nfrom = \"k\"\nto = \"r\"\namount = 5\n\
        [[conversions]]\nid = \"ladder\"\nfrom = \"k\"\nto = \"r\"\namount = 5\n";
    let text = with(conversions);
    assert_issue(&issues(&text), "conversions[1].id", "duplicate");
}

#[test]
fn unknown_account_owner() {
    let text =
        with("[[accounts]]\nid = \"b\"\nkind = \"brokerage\"\nowner = \"ghost\"\nbalance = 1\n");
    assert_issue(&issues(&text), "accounts[1].owner", "unknown person");
}

#[test]
fn roth_hsa_is_rejected() {
    let text = with(
        "[[accounts]]\nid = \"h\"\nkind = \"hsa\"\nroth = true\nowner = \"me\"\nbalance = 1\n",
    );
    assert_issue(&issues(&text), "accounts[1].roth", "cannot be Roth");
}

#[test]
fn negative_balance_and_bad_basis() {
    let text = with(
        "[[accounts]]\nid = \"b\"\nkind = \"brokerage\"\nowner = \"me\"\nbalance = 100\nbasis = 200\n",
    );
    assert_issue(&issues(&text), "accounts[1].basis", "exceed");
    let text = with(
        "[[accounts]]\nid = \"k\"\nkind = \"401k\"\nowner = \"me\"\nbalance = 100\nbasis = 50\n",
    );
    assert!(
        issues(&text).is_empty(),
        "after-tax basis on a deferred account"
    );
    let text = with(
        "[[accounts]]\nid = \"r\"\nkind = \"ira\"\nroth = true\nowner = \"me\"\nbalance = 100\nbasis = 50\n",
    );
    assert_issue(&issues(&text), "accounts[1].basis", "tax-deferred");
}

#[test]
fn contribution_rules_by_kind() {
    let text = with(
        "[[accounts]]\nid = \"i\"\nkind = \"ira\"\nowner = \"me\"\nbalance = 0\n[[contributions]]\nid = \"contribution-1\"\nto = \"i\"\nby = \"employer\"\namount = 5000\n",
    );
    assert_issue(&issues(&text), "contributions[0].by", "no employer");
    let text = with(
        "[[accounts]]\nid = \"s\"\nkind = \"sep-ira\"\nowner = \"me\"\nbalance = 0\n[[contributions]]\nid = \"contribution-2\"\nto = \"s\"\namount = 5000\n",
    );
    assert_issue(&issues(&text), "contributions[0].by", "no employee");
}

#[test]
fn a_contribution_names_an_account_and_one_timing() {
    let text = with("[[contributions]]\nid = \"contribution-3\"\nto = \"nowhere\"\namount = 1\n");
    assert_issue(&issues(&text), "contributions[0].to", "unknown account");
    let text = with(
        "[[accounts]]\nid = \"i\"\nkind = \"ira\"\nowner = \"me\"\nbalance = 0\n[[contributions]]\nid = \"contribution-4\"\nto = \"i\"\namount = 1\non = { age = 60, owner = \"me\" }\nend = { age = 65, owner = \"me\" }\n",
    );
    assert_issue(&issues(&text), "contributions[0]", "excludes");
}

#[test]
fn a_contribution_states_its_amount_one_way_and_a_share_names_an_income() {
    let ira = "[[accounts]]\nid = \"i\"\nkind = \"ira\"\nowner = \"me\"\nbalance = 0\n";
    let text = with(&format!(
        "{ira}[[contributions]]\nid = \"contribution-5\"\nto = \"i\"\n"
    ));
    assert_issue(&issues(&text), "contributions[0]", "exactly one");
    let text = with(&format!(
        "{ira}[[contributions]]\nid = \"contribution-6\"\nto = \"i\"\namount = 1\nrate = 0.1\nof = \"pay\"\n"
    ));
    assert_issue(&issues(&text), "contributions[0]", "exactly one");
    let text = with(&format!(
        "{ira}[[contributions]]\nid = \"contribution-7\"\nto = \"i\"\nrate = 0.1\n"
    ));
    assert_issue(&issues(&text), "contributions[0].of", "goes with");
    let text = with(&format!(
        "{ira}[[contributions]]\nid = \"contribution-8\"\nto = \"i\"\nrate = 0.1\nof = \"pay\"\n"
    ));
    assert_issue(&issues(&text), "contributions[0].of", "unknown income");
    let text = with(&format!(
        "{ira}[[contributions]]\nid = \"contribution-9\"\nto = \"i\"\namount = 1\nstep = {{ add = 0.01, up_to = 0.1 }}\n"
    ));
    assert_issue(&issues(&text), "contributions[0].step", "goes with `rate`");
    let pay = "[[income]]\nid = \"pay\"\nkind = \"salary\"\nowner = \"me\"\namount = 1000\n";
    let text = with(&format!(
        "{ira}{pay}[[contributions]]\nid = \"contribution-10\"\nto = \"i\"\nrate = 0.2\nof = \"pay\"\nstep = {{ add = 0.01, up_to = 0.1 }}\n"
    ));
    assert_issue(
        &issues(&text),
        "contributions[0].step.up_to",
        "at or above the rate",
    );
    let text = with(&format!(
        "{ira}{pay}[[contributions]]\nid = \"contribution-11\"\nto = \"i\"\nrate = 0.05\nof = \"pay\"\nstep = {{ add = 0.01, up_to = 0.1 }}\n"
    ));
    assert!(issues(&text).is_empty(), "{:?}", issues(&text));
}

#[test]
fn a_maximum_is_an_employee_s_and_a_match_an_employer_s_on_a_named_income() {
    let k = "[[accounts]]\nid = \"k\"\nkind = \"401k\"\nowner = \"me\"\nbalance = 0\n";
    let pay = "[[income]]\nid = \"pay\"\nkind = \"salary\"\nowner = \"me\"\namount = 1000\n";
    let text = with(&format!(
        "{k}[[contributions]]\nid = \"contribution-12\"\nto = \"k\"\nby = \"employer\"\nmax = true\n"
    ));
    assert_issue(&issues(&text), "contributions[0].max", "employee");
    let text = with(&format!(
        "{k}{pay}[[contributions]]\nid = \"contribution-13\"\nto = \"k\"\nmatch = {{ rate = 0.5, up_to = 0.06 }}\nof = \"pay\"\n"
    ));
    assert_issue(&issues(&text), "contributions[0].match", "employer");
    let text = with(&format!(
        "{k}[[contributions]]\nid = \"contribution-14\"\nto = \"k\"\nby = \"employer\"\nmatch = {{ rate = 0.5, up_to = 0.06 }}\n"
    ));
    assert_issue(&issues(&text), "contributions[0].of", "goes with");
    let text = with(
        "[[accounts]]\nid = \"s\"\nkind = \"sep-ira\"\nowner = \"me\"\nbalance = 0\n[[contributions]]\nid = \"contribution-15\"\nto = \"s\"\nby = \"employer\"\nmax = true\n",
    );
    assert_issue(&issues(&text), "contributions[0].max", "employee");
    let text = with(
        "[[accounts]]\nid = \"i\"\nkind = \"ira\"\nowner = \"me\"\nbalance = 0\n[[contributions]]\nid = \"contribution-16\"\nto = \"i\"\nby = \"after-tax\"\namount = 1\n",
    );
    assert_issue(&issues(&text), "contributions[0].by", "no after-tax");
    let text = with(&format!(
        "{k}{pay}[[contributions]]\nid = \"contribution-17\"\nto = \"k\"\nmax = true\n\n[[contributions]]\nid = \"contribution-18\"\nto = \"k\"\nby = \"employer\"\nmatch = {{ rate = 0.5, up_to = 0.06 }}\nof = \"pay\"\n\n[[contributions]]\nid = \"contribution-19\"\nto = \"k\"\nby = \"after-tax\"\namount = 1000\n"
    ));
    assert!(issues(&text).is_empty(), "{:?}", issues(&text));
}

#[test]
fn the_retired_contributions_table_is_refused_as_an_unknown_field() {
    let text = with(
        "[[accounts]]\nid = \"i\"\nkind = \"ira\"\nowner = \"me\"\nbalance = 0\n[accounts.contributions]\nemployee = 5000\n",
    );
    let refused = Plan::from_toml_str(&text).unwrap_err().to_string();
    assert!(refused.contains("[accounts.contributions]"), "{refused}");
    assert!(
        refused.contains("unknown field `contributions`"),
        "{refused}"
    );
}

#[test]
fn income_shape_rules() {
    let text =
        with("[[income]]\nid = \"income-1\"\nkind = \"windfall\"\nowner = \"me\"\namount = 10\n");
    assert_issue(&issues(&text), "income[0]", "requires `on`");
    let text = with(
        "[[income]]\nid = \"income-2\"\nkind = \"social-security\"\nowner = \"me\"\namount = 10\n",
    );
    assert_issue(&issues(&text), "income[0]", "explicit `start`");
    let text = with(
        "[[income]]\nid = \"income-3\"\nkind = \"other\"\nowner = \"me\"\namount = 10\non = { date = 2030-01-01 }\nstart = { date = 2030-01-01 }\n",
    );
    assert_issue(&issues(&text), "income[0]", "excludes");
}

#[test]
fn expense_shape_rules() {
    let text = with("[[expenses]]\nid = \"x\"\namount = -5\n");
    assert_issue(&issues(&text), "expenses[0].amount", "negative");
    let text = with(
        "[[expenses]]\nid = \"x\"\namount = 5\non = { date = 2030-01-01 }\nend = { date = 2031-01-01 }\n",
    );
    assert_issue(&issues(&text), "expenses[0]", "excludes");
}

#[test]
fn transfer_treatment_rules() {
    let accounts = "[[accounts]]\nid = \"k\"\nkind = \"401k\"\nowner = \"me\"\nbalance = 10\n[[accounts]]\nid = \"r\"\nkind = \"ira\"\nroth = true\nowner = \"me\"\nbalance = 0\n";
    let text = with(&format!(
        "{accounts}[[transfers]]\nid = \"transfer-1\"\nfrom = \"k\"\nto = \"r\"\non = {{ date = 2030-01-01 }}\n"
    ));
    assert_issue(&issues(&text), "transfers[0]", "conversion");
    let text = with(&format!(
        "{accounts}[[transfers]]\nid = \"transfer-2\"\nfrom = \"cash\"\nto = \"k\"\non = {{ date = 2030-01-01 }}\n"
    ));
    assert_issue(&issues(&text), "transfers[0]", "unsupported transfer");
    let text = with(
        "[[transfers]]\nid = \"transfer-3\"\nfrom = \"cash\"\nto = \"cash\"\non = { date = 2030-01-01 }\n",
    );
    assert_issue(&issues(&text), "transfers[0]", "must differ");
    let text = with(
        "[[transfers]]\nid = \"transfer-4\"\nfrom = \"cash\"\nto = \"ghost\"\non = { date = 2030-01-01 }\n",
    );
    assert_issue(&issues(&text), "transfers[0].to", "unknown account");
}

#[test]
fn conversion_rules() {
    let accounts = "[[accounts]]\nid = \"k\"\nkind = \"401k\"\nowner = \"me\"\nbalance = 10\n[[accounts]]\nid = \"r\"\nkind = \"ira\"\nroth = true\nowner = \"me\"\nbalance = 0\n";
    let text = with(&format!(
        "{accounts}[[conversions]]\nid = \"conversion-1\"\nfrom = \"cash\"\nto = \"r\"\namount = 5\n"
    ));
    assert_issue(&issues(&text), "conversions[0].from", "tax-deferred");
    let text = with(&format!(
        "{accounts}[[conversions]]\nid = \"conversion-2\"\nfrom = \"k\"\nto = \"cash\"\namount = 5\n"
    ));
    assert_issue(&issues(&text), "conversions[0].to", "Roth");
    let text = with(&format!(
        "{accounts}[[conversions]]\nid = \"conversion-3\"\nfrom = \"k\"\nto = \"r\"\namount = 0\n"
    ));
    assert_issue(&issues(&text), "conversions[0].amount", "positive");
}

#[test]
fn trigger_shape_rules() {
    let text =
        with("[[events]]\nid = \"e\"\ntrigger = { date = 2030-01-01, age = 60, owner = \"me\" }\n");
    assert_issue(&issues(&text), "events[0].trigger", "exactly one");
    let text = with("[[events]]\nid = \"e\"\ntrigger = { age = 60 }\n");
    assert_issue(&issues(&text), "events[0].trigger", "owner");
    let text = with("[[events]]\nid = \"e\"\ntrigger = { date = 2030-01-01, offset = 2 }\n");
    assert_issue(&issues(&text), "events[0].trigger", "offset");
}

#[test]
fn trigger_reference_rules() {
    let text = with("[[events]]\nid = \"e\"\ntrigger = { event = \"ghost\" }\n");
    assert_issue(&issues(&text), "events[0].trigger", "unknown event");
    let text = with("[[events]]\nid = \"e\"\ntrigger = { income = \"ghost\" }\n");
    assert_issue(&issues(&text), "events[0].trigger", "unknown income");
    let text = with("[[events]]\nid = \"e\"\ntrigger = { age = 60, owner = \"ghost\" }\n");
    assert_issue(&issues(&text), "events[0].trigger", "unknown person");
}

#[test]
fn trigger_cycles_are_detected() {
    let text = with(
        "[[events]]\nid = \"a\"\ntrigger = { event = \"b\" }\n[[events]]\nid = \"b\"\ntrigger = { event = \"a\" }\n",
    );
    assert_issue(&issues(&text), "events.a", "cycle");
}

#[test]
fn surplus_target_rules() {
    let text = BASE.replace("[household]", "surplus_to = \"ghost\"\n\n[household]");
    assert_issue(&issues(&text), "plan.surplus_to", "unknown account");
    let text = BASE.replace("kind = \"cash\"", "kind = \"401k\"");
    assert_issue(&issues(&text), "plan.surplus_to", "no cash account");
}

#[test]
fn settings_bounds() {
    let text = BASE.replace("inflation = 0.025", "inflation = 0.9");
    assert_issue(&issues(&text), "plan.inflation", "between");
    let text = BASE.replace("inflation = 0.025", "inflation = 0.025\nwage_growth = 0.9");
    assert_issue(&issues(&text), "plan.wage_growth", "between");
    let text = BASE.replace("start_year = 2026", "start_year = 1200");
    assert_issue(&issues(&text), "plan.start_year", "plausible");
    let text = BASE.replace(
        "inflation = 0.025",
        "inflation = 0.025\nwithdrawal_order = [\"taxable\", \"taxable\"]",
    );
    assert_issue(&issues(&text), "plan.withdrawal_order", "repeat");
}

#[test]
fn household_date_rules() {
    let text = BASE.replace("birth = 1980-01-01", "birth = 2030-01-01");
    assert_issue(&issues(&text), "household.people[0].birth", "before");
    let text = BASE.replace("horizon_age = 90", "horizon_age = 40");
    assert_issue(
        &issues(&text),
        "household.people[0]",
        "past plan.horizon_age",
    );
}

const MOVE: &str = "from = { date = 2035-01-01 }";

#[test]
fn a_residency_names_known_places() {
    let found = issues(&with("[[residency]]\ncountry = \"zz\"\n"));
    assert_issue(&found, "residency[0].country", "not an ISO 3166-1");
    let found = issues(&with("[[residency]]\ncountry = \"us\"\nstate = \"pr\"\n"));
    assert_issue(&found, "residency[0].state", "not a U.S. state code");
}

#[test]
fn a_state_is_given_exactly_where_the_country_has_them() {
    let found = issues(&with("[[residency]]\ncountry = \"us\"\n"));
    assert_issue(&found, "residency[0].state", "names its state");
    let found = issues(&with("[[residency]]\ncountry = \"pt\"\nstate = \"or\"\n"));
    assert_issue(&found, "residency[0].state", "only a `us` residency");
    let abroad =
        "[[residency]]\ncountry = \"us\"\nstate = \"or\"\n\n[[residency]]\ncountry = \"pt\"\n";
    assert!(issues(&with(&format!("{abroad}{MOVE}\n"))).is_empty());
}

#[test]
fn residencies_have_one_beginning() {
    assert!(issues(BASE).is_empty(), "none is federal tax only");
    let oregon = "[[residency]]\ncountry = \"us\"\nstate = \"or\"\n";
    let washington = "[[residency]]\ncountry = \"us\"\nstate = \"wa\"\n";
    let found = issues(&with(&format!("{oregon}\n{washington}")));
    assert_issue(&found, "residency[1].from", "only one residency begins");
    let found = issues(&with(&format!("{oregon}{MOVE}\n")));
    assert_issue(&found, "residency", "starts nowhere");
    assert!(issues(&with(&format!("{oregon}\n{washington}{MOVE}\n"))).is_empty());
}

#[test]
fn place_codes_are_read_in_any_case_and_kept_lowercase() {
    let plan = Plan::from_toml_str(&with("[[residency]]\ncountry = \"US\"\nstate = \"Or\"\n"));
    let plan = plan.unwrap();
    assert!(plan.validate().is_empty());
    let saved = plan.to_toml_string().unwrap();
    assert!(
        saved.contains("country = \"us\"\nstate = \"or\"\n"),
        "{saved}"
    );
}

#[test]
fn only_social_security_may_leave_out_its_amount() {
    let salary = with(
        r#"
[[income]]
id = "income-4"
kind = "salary"
owner = "me"
"#,
    );
    assert_issue(&issues(&salary), "income[0].amount", "required");
    let benefit = with(
        r#"
[[income]]
id = "income-5"
kind = "social-security"
owner = "me"
start = { age = 67, owner = "me" }
"#,
    );
    assert!(issues(&benefit).is_empty(), "{:?}", issues(&benefit));
}

#[test]
fn earnings_must_not_be_negative() {
    let text = BASE.replace(
        "birth = 1980-01-01",
        "birth = 1980-01-01\nearnings = { 2019 = 50000, 2020 = -1 }",
    );
    assert_issue(
        &issues(&text),
        "household.people[0].earnings.2020",
        "must not be negative",
    );
}
