//! What needs attention: the draft's issues, the years the plan runs
//! short or pays Medicare's surcharges, the contributions it could not make
//! as stated, and a benefit estimated without the record it is computed
//! from.

use std::collections::BTreeSet;

use retiretui_engine::plan::{Dollars, Item, Plan};
use retiretui_engine::project::YearRow;

use super::Row;
use crate::actions::{Held, held_contributions};
use crate::draft::Draft;
use crate::forms::DomainId;
use crate::issues::{issue_field, issue_place, issue_words};
use crate::present::compact_money;
use crate::session::Projected;
use crate::table::{account_name, basis_amount, money};

/// What a surface says where nothing needs attention.
pub const NOTHING: &str = "No plan issues";

/// Each of the draft's issues in the forms' words, in validation order,
/// at the field it is about.
#[must_use]
pub fn issue_rows(draft: &Draft) -> Vec<Row> {
    let issues = draft.issues().iter();
    issues
        .map(|issue| Row {
            place: issue_place(&issue.path),
            field: issue_field(&issue.path),
            is_issue: true,
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
        unfunded(years, &dollars),
        surcharged(years, &dollars),
        held(plan, years),
        unrecorded(plan),
    ]
    .into_iter()
    .flatten()
    .collect();
    found.sort_by_key(|row| row.year);
    found
}

/// The first year spending outruns the money, and by how much.
fn unfunded(years: &[YearRow], dollars: &impl Fn(&YearRow, Dollars) -> Dollars) -> Vec<Row> {
    let short = years.iter().find(|row| row.unfunded > 0);
    short
        .map(|row| {
            let short = compact_money(dollars(row, row.unfunded));
            Row {
                year: Some(row.year),
                ..Row::plain(format!("on: unfunded, {short} a year"))
            }
        })
        .into_iter()
        .collect()
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
        money(dollars(first, first.medicare))
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

#[cfg(test)]
mod tests {
    use super::*;
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
                format!("on: unfunded, {} a year", compact_money(first.unfunded)),
                None,
            ),
            (
                format!("Medicare surcharges, {}", money(first.medicare)),
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
        assert!(said[2].0.ends_with("years in all"), "{said:?}");
        assert_eq!(said.len(), 4, "{said:?}");
    }

    #[test]
    fn each_issue_is_a_row_at_its_field() {
        let plan = TEST_PLAN.replace("balance = 200000", "balance = -1");
        let plan = Plan::from_toml_str(&plan).unwrap();
        let draft = Draft::validated(plan, &TaxTables::embedded(), false);
        let rows = issue_rows(&draft);
        assert_eq!(rows.len(), draft.issues().len());
        let row = &rows[0];
        assert!(row.is_issue);
        assert_eq!(row.place, Some((DomainId::Accounts, Some(1))));
        assert_eq!(row.field, Some("balance"));
        assert_eq!(row.text, issue_words(&draft.issues()[0], &draft));
    }
}
