//! What a Roth account gives up when it is drawn before the draw is
//! qualified, and what of it is taxed and penalized.

mod common;

use retiretui_engine::plan::Dollars;
use retiretui_engine::project::Projection;

use common::{born_in, drawn, run};

const ROTH_FIRST: &str = "inflation = 0.0\nwithdrawal_order = [\"roth\", \"taxable\"]";

fn free(amount: Dollars) -> [Dollars; 3] {
    [amount, 0, 0]
}

/// A plan for `me`, born in June of `birth_year`, holding `cash` and the
/// accounts of `body`, the Roth class drained first.
fn holding(birth_year: i16, cash: Dollars, body: &str) -> String {
    let body = format!(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = {cash}
{body}"#
    );
    born_in(birth_year, &body).replace("inflation = 0.0", ROTH_FIRST)
}

fn spending(year: i16, amount: Dollars) -> String {
    format!(
        "\n[[expenses]]\nid = \"spent-{year}\"\namount = {amount}\non = {{ date = {year}-01-01 }}\n"
    )
}

/// A Roth IRA of 50,000 that earns nothing, 30,000 of it paid in, with
/// 10,000 converted into it in 2026, and 45,000 spent in `year`.
fn converted_then_spent(birth_year: i16, cash: Dollars, year: i16) -> Projection {
    let body = format!(
        r#"
[[accounts]]
id = "ira"
kind = "ira"
owner = "me"
balance = 10000

[[accounts]]
id = "roth"
kind = "ira"
roth = true
owner = "me"
balance = 50000
basis = 30000

[[conversions]]
id = "converted"
from = "ira"
to = "roth"
amount = 10000
on = {{ date = 2026-01-01 }}
{}"#,
        spending(year, 45_000)
    );
    run(&holding(birth_year, cash, &body))
}

#[test]
fn a_roth_ira_gives_up_what_was_paid_in_then_conversions_then_earnings() {
    // Born June 1980: 49 in 2029. The 30,000 paid in is free, the 10,000
    // converted three years before pays the penalty untaxed, and the rest is
    // earnings: 5,000 of spending and the penalty itself, 6,667, a tenth of
    // 16,667.
    let early = converted_then_spent(1980, 0, 2029);
    let expected = [46_667, 6_667, 1_667];
    assert_eq!(drawn(&early, "roth", 2029), expected);
}

#[test]
fn a_conversion_is_penalized_inside_its_five_years_and_free_from_the_sixth() {
    let fourth_year = converted_then_spent(1980, 0, 2030);
    assert_eq!(drawn(&fourth_year, "roth", 2030)[2], 1_667);
    // From 2031 only the earnings pay: 5,556 of them, a tenth of which is
    // the penalty.
    let sixth_year = converted_then_spent(1980, 0, 2031);
    let expected = [45_556, 5_556, 556];
    assert_eq!(drawn(&sixth_year, "roth", 2031), expected);
}

#[test]
fn a_roth_held_five_years_is_free_from_59_and_a_half() {
    // Born June 1966: 59 and a half in December 2025.
    let qualified = converted_then_spent(1966, 0, 2029);
    assert_eq!(drawn(&qualified, "roth", 2029), free(45_000));
}

#[test]
fn the_first_pass_takes_from_a_roth_ira_only_what_leaves_it_free() {
    let inside = converted_then_spent(1980, 100_000, 2029);
    assert_eq!(drawn(&inside, "roth", 2029), free(30_000));
    assert_eq!(drawn(&inside, "cash", 2029)[0], 15_000);
    let seasoned = converted_then_spent(1980, 100_000, 2031);
    assert_eq!(drawn(&seasoned, "roth", 2031), free(40_000));
    assert_eq!(drawn(&seasoned, "cash", 2031)[0], 5_000);
    // Born June 1966: no penalty holds a Roth back past 59 and a half.
    let qualified = converted_then_spent(1966, 100_000, 2029);
    assert_eq!(drawn(&qualified, "roth", 2029), free(45_000));
}

#[test]
fn a_persons_roth_iras_give_up_what_was_paid_in_as_one() {
    // Born June 1980. Nothing was paid into the first, 20,000 into the
    // second.
    let body = format!(
        r#"
[[accounts]]
id = "earned"
kind = "ira"
roth = true
owner = "me"
balance = 20000
basis = 0

[[accounts]]
id = "paid-in"
kind = "ira"
roth = true
owner = "me"
balance = 20000
{}{}"#,
        spending(2026, 15_000),
        spending(2027, 10_000)
    );
    let projection = run(&holding(1980, 0, &body));
    assert_eq!(drawn(&projection, "earned", 2026), free(15_000));
    // 5,000 of what was paid in is left, and the first account holds
    // 5,000: the other 5,000, and the penalty on it, are earnings.
    let rest = [5_000, 5_556, 556];
    assert_eq!(drawn(&projection, "earned", 2027), rest);
    assert_eq!(drawn(&projection, "paid-in", 2027)[0], 5_556);
}

#[test]
fn a_roth_ira_that_lost_money_holds_no_earnings() {
    let body = format!(
        r#"
[[accounts]]
id = "roth"
kind = "ira"
roth = true
owner = "me"
balance = 50000
expected_return = -0.5
{}"#,
        spending(2026, 25_000)
    );
    let projection = run(&holding(1980, 0, &body));
    assert_eq!(drawn(&projection, "roth", 2026), free(25_000));
}

/// A Roth account of `kind` that opens empty and earns 10% a year, beside
/// what `funded` pays into it in 2026.
fn opened_in_2026(birth_year: i16, funded: &str, spent: &[(i16, Dollars)]) -> Projection {
    let spent: String = spent
        .iter()
        .map(|&(year, amount)| spending(year, amount))
        .collect();
    let body = format!(
        r#"
[[accounts]]
id = "roth"
kind = "ira"
roth = true
owner = "me"
balance = 0
expected_return = 0.1
{funded}{spent}"#
    );
    run(&holding(birth_year, 0, &body))
}

const CONVERTED: &str = r#"
[[accounts]]
id = "ira"
kind = "ira"
owner = "me"
balance = 10000
basis = 4000

[[conversions]]
id = "converted"
from = "ira"
to = "roth"
amount = 10000
on = { date = 2026-01-01 }
"#;

#[test]
fn a_roth_opened_by_a_conversion_is_taxed_on_its_earnings_for_five_years() {
    // Born June 1966, past 59 and a half. 10,000 converted in 2026 is
    // 11,000 in 2027, 1,000 of it earnings, and 500 of what is left is 666
    // by 2030.
    let spent = [(2027, 10_500), (2030, 100), (2031, 100)];
    let projection = opened_in_2026(1966, CONVERTED, &spent);
    assert_eq!(drawn(&projection, "roth", 2027), [10_500, 500, 0]);
    assert_eq!(drawn(&projection, "roth", 2030), [100, 100, 0]);
    assert_eq!(drawn(&projection, "roth", 2031), free(100));
}

#[test]
fn a_roth_ira_is_five_years_old_once_its_owners_first_is() {
    // Born June 1966, and holding a Roth IRA, all of it earnings, since
    // before the plan.
    let held = format!(
        "{CONVERTED}\n[[accounts]]\nid = \"held\"\nkind = \"ira\"\nroth = true\nowner = \"me\"\nbalance = 1000\nbasis = 0\n"
    );
    let projection = opened_in_2026(1966, &held, &[(2027, 10_500)]);
    assert_eq!(drawn(&projection, "roth", 2027), free(10_500));
}

#[test]
fn what_a_conversion_carried_untaxed_is_paid_in() {
    // Born June 1980. 4,000 of the 10,000 converted was after-tax basis.
    let spent = [(2027, 4_000), (2028, 1_000)];
    let projection = opened_in_2026(1980, CONVERTED, &spent);
    assert_eq!(drawn(&projection, "roth", 2027), free(4_000));
    // The next 1,000, and the penalty on it, are the conversion's taxed
    // part: 1,111, a tenth of which is the penalty.
    let converted = [1_111, 0, 111];
    assert_eq!(drawn(&projection, "roth", 2028), converted);
}

#[test]
fn a_roth_opened_by_a_contribution_is_taxed_on_its_earnings_for_five_years() {
    // Born June 1966. 7,000 paid in at the close of 2026 is 8,470 in 2028.
    let contributed = r#"
[[income]]
id = "salary"
kind = "salary"
owner = "me"
amount = 50000
end = { date = 2026-12-31 }

[[contributions]]
id = "paid-in"
to = "roth"
amount = 7000
end = { date = 2026-12-31 }
"#;
    let spent = [(2028, 7_500), (2031, 500)];
    let projection = opened_in_2026(1966, contributed, &spent);
    let earnings = [7_500, 500, 0];
    assert_eq!(drawn(&projection, "roth", 2028), earnings);
    assert_eq!(drawn(&projection, "roth", 2031), free(500));
}

#[test]
fn a_conversion_into_a_roth_workplace_plan_is_paid_in_whole() {
    // Born June 1980. Nothing of the 10,000 converted is earnings.
    let body = format!(
        r#"
[[accounts]]
id = "ira"
kind = "ira"
owner = "me"
balance = 10000

[[accounts]]
id = "k"
kind = "401k"
roth = true
owner = "me"
balance = 0

[[conversions]]
id = "converted"
from = "ira"
to = "k"
amount = 10000
on = {{ date = 2026-01-01 }}
{}"#,
        spending(2027, 8_000)
    );
    let projection = run(&holding(1980, 0, &body));
    assert_eq!(drawn(&projection, "k", 2027), free(8_000));
}

const K401: &str = "\"401k\"";
const K457: &str = "\"457b\"";

/// A Roth account whose table opens with `stated`, its kind, half of its
/// 100,000 paid in, earning nothing, with 10,000 spent in 2026.
fn workplace(birth_year: i16, cash: Dollars, stated: &str) -> Projection {
    let body = format!(
        r#"
[[accounts]]
id = "k"
kind = {stated}
roth = true
owner = "me"
balance = 100000
basis = 50000
{}"#,
        spending(2026, 10_000)
    );
    run(&holding(birth_year, cash, &body))
}

#[test]
fn a_roth_workplace_plan_gives_up_what_was_paid_in_pro_rata() {
    // Born June 1976: 50 in 2026. Half of every dollar drawn is earnings,
    // and a tenth of those the penalty: 10,526 drawn to spend 10,000.
    let penalized = [10_526, 5_263, 526];
    assert_eq!(drawn(&workplace(1976, 0, K401), "k", 2026), penalized);
    let taxed = [10_000, 5_000, 0];
    assert_eq!(drawn(&workplace(1976, 0, K457), "k", 2026), taxed);
    // Born June 1970: 56 in 2026, the year the job is left.
    let left = "\"401k\"\nseparated = { date = 2026-03-01 }";
    assert_eq!(drawn(&workplace(1970, 0, left), "k", 2026), taxed);
    // Born June 1966: past 59 and a half, the plan held since before 2026.
    assert_eq!(drawn(&workplace(1966, 0, K401), "k", 2026), free(10_000));
}

#[test]
fn a_roth_workplace_plan_that_pays_the_penalty_drains_last() {
    let penalized = workplace(1976, 100_000, K401);
    assert_eq!(drawn(&penalized, "k", 2026)[0], 0);
    assert_eq!(drawn(&penalized, "cash", 2026), free(10_000));
    let exempt = workplace(1976, 100_000, K457);
    assert_eq!(drawn(&exempt, "k", 2026)[0], 10_000);
}

/// A Roth 401(k) of 20,000, half of it paid in, rolled whole in 2026 into
/// a Roth IRA that opens empty and earns 10% a year.
fn rolled_over(birth_year: i16, spent: &[(i16, Dollars)]) -> Projection {
    let rollover = r#"
[[accounts]]
id = "k"
kind = "401k"
roth = true
owner = "me"
balance = 20000
basis = 10000

[[transfers]]
id = "rollover"
from = "k"
to = "roth"
on = { date = 2026-01-01 }
"#;
    opened_in_2026(birth_year, rollover, spent)
}

#[test]
fn a_qualified_rollover_is_paid_in_and_starts_the_five_years_of_what_it_opens() {
    // Born June 1966. The 20,000 is 22,000 in 2027, and the 1,000 left is
    // 1,331 by 2030.
    let projection = rolled_over(1966, &[(2027, 21_000), (2030, 1_000), (2031, 300)]);
    assert_eq!(drawn(&projection, "roth", 2027), [21_000, 1_000, 0]);
    assert_eq!(drawn(&projection, "roth", 2030), [1_000, 1_000, 0]);
    assert_eq!(drawn(&projection, "roth", 2031), free(300));
}

#[test]
fn an_early_rollover_carries_what_was_paid_in() {
    // Born June 1980. Half of the 20,000 rolled over was paid in.
    let projection = rolled_over(1980, &[(2027, 10_000)]);
    assert_eq!(drawn(&projection, "roth", 2027), free(10_000));
}
