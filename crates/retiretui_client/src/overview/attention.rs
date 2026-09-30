//! What needs attention: the draft's issues, the years the plan pays
//! Medicare's surcharges, the contributions it could not make as stated, a
//! benefit estimated without the record it is computed from, amounts too
//! large to be likely, and the historical starts the plan does not survive.

use std::collections::BTreeSet;

use serde::Serialize;

use retiretui_engine::market::Runs;
use retiretui_engine::plan::{Dollars, Item, Plan};
use retiretui_engine::project::YearRow;

use super::Row;
use crate::actions::{Held, held_contributions};
use crate::draft::Draft;
use crate::forms::DomainId;
use crate::issues::{issue_place, issue_words};
use crate::present::compact_money;
use crate::searches::markets::Markets;
use crate::session::Projected;
use crate::table::{account_name, basis_amount, count};

/// What a surface says where nothing needs attention.
pub const NOTHING: &str = "No plan issues";

/// Each of the draft's issues in the forms' words, in validation order,
/// at the item it is about.
#[must_use]
pub fn issue_rows(draft: &Draft) -> Vec<Row> {
    let issues = draft.issues().iter();
    issues
        .map(|issue| Row {
            place: issue_place(&issue.path),
            ..Row::plain(issue_words(issue, draft))
        })
        .collect()
}

/// What in the projection wants looking at, what has no year first and the
/// rest by year.
#[must_use]
pub fn attention(projected: &Projected, nominal: bool) -> Vec<Row> {
    let plan = &projected.plan;
    let years = &projected.projection.years;
    let dollars = |row: &YearRow, amount: Dollars| basis_amount(amount, row.deflator, nominal);
    let mut found: Vec<Row> = [
        surcharged(years, &dollars),
        held(plan, years),
        unrecorded(plan),
        implausible(plan),
    ]
    .into_iter()
    .flatten()
    .collect();
    found.sort_by_key(|row| row.year);
    found
}

/// The worst historical start the plan does not survive, as the Overview
/// lists it.
#[derive(Serialize, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FailingStart {
    /// What the Historical tool keeps its run by.
    pub key: String,
    /// The row: the start, and how many fail.
    pub said: String,
}

/// The start the Historical tool lists first, where the plan does not
/// survive it; none where every start survives.
#[must_use]
pub fn failing_start(runs: &Runs) -> Option<FailingStart> {
    let worst = runs.listed().into_iter().next()?;
    if worst.run.is_success {
        return None;
    }
    let starts = runs.runs.len();
    let failed = count(starts - runs.successes);
    let said = format!(
        "Fails from a {} start · {failed} of {} fail",
        worst.first,
        count(starts)
    );
    Some(FailingStart {
        key: worst.key,
        said,
    })
}

/// The first year Medicare's surcharges or a cliff cost anything, and in
/// how many years they do.
fn surcharged(years: &[YearRow], dollars: &impl Fn(&YearRow, Dollars) -> Dollars) -> Vec<Row> {
    let mut paying = years.iter().filter(|row| row.medicare > 0);
    let Some(first) = paying.next() else {
        return Vec::new();
    };
    let mut text = format!(
        "Medicare surcharges, {}",
        compact_money(dollars(first, first.medicare))
    );
    let more = paying.count();
    if more > 0 {
        text = format!("{text}, {} years in all", more + 1);
    }
    vec![Row::dated(first.year, text, (DomainId::Household, None))]
}

/// Each account's contributions held back, once for each way they were,
/// in the first year they were.
fn held(plan: &Plan, years: &[YearRow]) -> Vec<Row> {
    let mut seen = BTreeSet::new();
    let mut found = Vec::new();
    for row in years {
        for (account, how) in held_contributions(row) {
            if !seen.insert((account, how)) {
                continue;
            }
            let into = plan
                .contributions
                .iter()
                .position(|paid| paid.to == account);
            let opens = (DomainId::Contributions, into);
            let text = format!("{}: {}", account_name(plan, account), held_words(how));
            found.push(Row::dated(row.year, text, opens));
        }
    }
    found
}

const fn held_words(how: Held) -> &'static str {
    match how {
        Held::ToLimit => "held to its limit",
        Held::PhasedOut => "over the Roth IRA income limit",
        Held::NotDeducted => "not all deductible",
    }
}

/// Each person whose benefit the engine computes with no earnings record,
/// from the career it estimates at their salary.
fn unrecorded(plan: &Plan) -> Vec<Row> {
    let each = plan.household.people.iter().enumerate();
    each.filter(|(_, person)| person.earnings.is_empty())
        .filter(|(_, person)| {
            (plan.income.iter())
                .any(|income| income.is_benefit_of(&person.id) && income.is_derived())
        })
        .map(|(at, person)| Row {
            place: Some((DomainId::People, Some(at))),
            ..Row::plain(format!(
                "{}'s Social Security is computed from an estimated career at their salary",
                person.display_name()
            ))
        })
        .collect()
}

/// A yearly amount from which a stated one is more likely a slip than a
/// plan: an income, an expense, a contribution, a transfer or a conversion.
const IMPLAUSIBLE_FLOW: Dollars = 10_000_000;
/// A balance from which a stated one is more likely a slip than a plan.
const IMPLAUSIBLE_BALANCE: Dollars = 1_000_000_000;

/// Each amount the plan states at or past what is likely, at its item; the
/// projection runs on them all the same.
fn implausible(plan: &Plan) -> Vec<Row> {
    use DomainId::{Accounts, Contributions, Conversions, Expenses, Income, Transfers};
    let (flow, balance) = (IMPLAUSIBLE_FLOW, IMPLAUSIBLE_BALANCE);
    [
        stated(Accounts, &plan.accounts, ("a balance", balance), |it| {
            Some(it.balance)
        }),
        stated(Accounts, &plan.accounts, ("a basis", balance), |it| {
            it.basis
        }),
        stated(Income, &plan.income, ("an income", flow), |it| it.amount),
        stated(Expenses, &plan.expenses, ("an expense", flow), |it| {
            Some(it.amount)
        }),
        stated(
            Contributions,
            &plan.contributions,
            ("a contribution", flow),
            |it| it.amount,
        ),
        stated(Transfers, &plan.transfers, ("a transfer", flow), |it| {
            it.amount
        }),
        stated(
            Conversions,
            &plan.conversions,
            ("a conversion", flow),
            |it| Some(it.amount),
        ),
    ]
    .concat()
}

/// A row for each of `items` whose `amount` is at or past `line`, said as
/// `what` it is.
fn stated<T: Item>(
    domain: DomainId,
    items: &[T],
    (what, line): (&str, Dollars),
    amount: fn(&T) -> Option<Dollars>,
) -> Vec<Row> {
    let each = items.iter().enumerate();
    each.filter_map(|(at, item)| {
        let amount = amount(item).filter(|&amount| amount >= line)?;
        let text = format!(
            "{}: {what} of {} - check the amount",
            item.display_name(),
            compact_money(amount)
        );
        Some(Row {
            place: Some((domain, Some(at))),
            ..Row::plain(text)
        })
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use retiretui_engine::market::RunName;
    use retiretui_engine::params::TaxTables;

    use crate::overview::Place;
    use crate::overview::tests::{TEST_PLAN, projected_from, test_projected};

    /// The test plan spending past its means, paying into its 401(k) past
    /// the limit, over a cliff every year, and claiming a computed benefit.
    fn wanting() -> String {
        let plan = TEST_PLAN.replace("amount = 60000", "amount = 400000");
        format!(
            "{plan}
[[contributions]]
id = \"deferral\"
to = \"k\"
amount = 50000

[[cliffs]]
id = \"aca\"
magi_over = 50000
cost = 5000

[[income]]
id = \"ss\"
kind = \"social-security\"
owner = \"me\"
start = {{ age = 67, owner = \"me\" }}
"
        )
    }

    fn said(projected: &Projected) -> Vec<(String, Option<Place>)> {
        let rows = attention(projected, true);
        (rows.iter()).map(|row| (row.line(), row.place)).collect()
    }

    #[test]
    fn a_plan_with_nothing_wanting_has_no_rows() {
        assert_eq!(attention(&test_projected(), false), []);
    }

    #[test]
    fn each_kind_says_its_first_year_and_opens_its_item() {
        let projected = projected_from(&wanting());
        assert_eq!(projected.plan.validate(), []);
        let first = &projected.projection.years[0];
        let expected = [
            (
                format!("Medicare surcharges, {}", compact_money(first.medicare)),
                Some((DomainId::Household, None)),
            ),
            (
                "k: held to its limit".to_owned(),
                Some((DomainId::Contributions, Some(0))),
            ),
        ];
        let said = said(&projected);
        let unrecorded = (
            "me's Social Security is computed from an estimated career at their salary".to_owned(),
            Some((DomainId::People, Some(0))),
        );
        assert_eq!(said[0], unrecorded, "what has no year leads");
        for (at, (text, place)) in expected.into_iter().enumerate() {
            assert!(
                said[at + 1].0.starts_with(&format!("2026 {text}")),
                "{said:?}"
            );
            assert_eq!(said[at + 1].1, place, "{said:?}");
        }
        assert!(said[1].0.ends_with("years in all"), "{said:?}");
        assert_eq!(said.len(), 3, "short every year, and said elsewhere");
    }

    #[test]
    fn an_amount_past_the_likely_is_a_row_at_its_item_and_refuses_nothing() {
        let plan = TEST_PLAN
            .replace("balance = 200000", "balance = 2000000000")
            .replace("amount = 60000", "amount = 10000000");
        let projected = projected_from(&plan);
        assert_eq!(projected.plan.validate(), []);
        let account = projected.plan.accounts[1].display_name();
        let expense = projected.plan.expenses[0].display_name();
        let said = said(&projected);
        let expected = [
            (
                format!("{account}: a balance of $2.00B - check the amount"),
                Some((DomainId::Accounts, Some(1))),
            ),
            (
                format!("{expense}: an expense of $10.00M - check the amount"),
                Some((DomainId::Expenses, Some(0))),
            ),
        ];
        for row in expected {
            assert!(said.contains(&row), "{row:?} in {said:?}");
        }
    }

    #[test]
    fn each_issue_is_a_row_at_its_item() {
        let plan = TEST_PLAN.replace("balance = 200000", "balance = -1");
        let plan = Plan::from_toml_str(&plan).unwrap();
        let draft = Draft::validated(plan, &TaxTables::embedded(), false);
        let rows = issue_rows(&draft);
        assert_eq!(rows.len(), draft.issues().len());
        let row = &rows[0];
        assert_eq!(row.place, Some((DomainId::Accounts, Some(1))));
        assert_eq!(row.text, issue_words(&draft.issues()[0], &draft));
    }

    fn starts(plan_text: &str) -> Runs {
        use retiretui_engine::market::{History, Progress, historical};
        let plan = projected_from(plan_text).plan;
        let tables = TaxTables::embedded();
        historical(&plan, &tables, History::embedded(), &Progress::default()).unwrap()
    }

    #[test]
    fn the_worst_failing_start_is_said_with_how_many_fail() {
        let runs = starts(&TEST_PLAN.replace("amount = 60000", "amount = 70000"));
        let worst = &runs.runs[runs.worst_first()[0]];
        let RunName::Start(year) = worst.name else {
            panic!("a historical run is named by its start");
        };
        let failed = runs.runs.len() - runs.successes;
        assert!(failed > 0 && runs.successes > 0, "some starts fail");
        let said = format!(
            "Fails from a {year} start · {failed} of {} fail",
            runs.runs.len()
        );
        let key = year.to_string();
        assert_eq!(failing_start(&runs), Some(FailingStart { key, said }));
        let modest = TEST_PLAN.replace("amount = 60000", "amount = 20000");
        assert_eq!(
            failing_start(&starts(&modest)),
            None,
            "every start survives"
        );
    }
}
