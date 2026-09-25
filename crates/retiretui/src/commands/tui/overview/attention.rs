//! The Needs attention pane: what in the plan wants looking at - its
//! issues, the years it runs short or pays Medicare's surcharges, the
//! contributions it could not make as stated, a benefit estimated
//! without the record it is computed from, and the historical starts it
//! does not survive.

use std::collections::BTreeSet;

use retiretui_engine::market::{RunName, Runs};
use retiretui_engine::plan::{Dollars, Plan};
use retiretui_engine::project::YearRow;

#[cfg(test)]
use super::rows::Target;
use super::rows::{Entry, Tone};
use crate::commands::actions::{Held, held_contributions};
use crate::commands::table::{account_name, basis_amount, money};
use crate::commands::tui::edit::{self, Draft};
use crate::commands::tui::nav::Page;
use crate::commands::tui::present::{compact_money, issue_count};
use crate::commands::tui::session::Projected;
use crate::commands::tui::tools::count_text;

pub(super) const NOTHING: &str = "No plan issues";

/// Every kind in turn, what has no year first and the rest by year, or a
/// line saying there is nothing; the starts are the plan's `historical`
/// runs, where they have been run.
pub(super) fn entries(
    projected: &Projected,
    draft: &Draft,
    (nominal, historical): (bool, Option<&Runs>),
) -> Vec<Entry> {
    let plan = &projected.plan;
    let years = &projected.projection.years;
    let dollars = |row: &YearRow, amount: Dollars| basis_amount(amount, row.deflator, nominal);
    let mut found: Vec<Entry> = [
        issues(draft),
        failing(historical),
        unfunded(years, &dollars),
        surcharged(years, &dollars),
        held(plan, years),
        unrecorded(plan),
    ]
    .into_iter()
    .flatten()
    .collect();
    found.sort_by_key(|entry| entry.year);
    if found.is_empty() {
        found.push(Entry {
            tone: Tone::Quiet,
            ..Entry::plain(NOTHING.to_owned())
        });
    }
    found
}

/// How many issues the draft has, and the first in the forms' words.
fn issues(draft: &Draft) -> Vec<Entry> {
    let issues = draft.issues();
    let Some(first) = issues.first() else {
        return Vec::new();
    };
    let text = format!(
        "{} · {}",
        issue_count(issues.len()),
        edit::issue_words(first, draft)
    );
    vec![Entry {
        opens: edit::issue_target(&first.path),
        tone: Tone::Warning,
        ..Entry::plain(text)
    }]
}

/// The worst historical start the plan does not survive - the one the
/// Historical page lists first - and how many it does not.
fn failing(historical: Option<&Runs>) -> Vec<Entry> {
    let Some(runs) = historical else {
        return Vec::new();
    };
    let worst = runs.worst_first().first().map(|&at| &runs.runs[at]);
    let Some(run) = worst.filter(|run| !run.is_success) else {
        return Vec::new();
    };
    let RunName::Start(year) = run.name else {
        return Vec::new();
    };
    let starts = runs.runs.len();
    let failed = count_text(starts - runs.successes);
    let text = format!(
        "Fails from a {year} start · {failed} of {} fail",
        count_text(starts)
    );
    vec![Entry::leading(text, (Page::Historical, None))]
}

/// The first year spending outruns the money, and by how much.
fn unfunded(years: &[YearRow], dollars: &impl Fn(&YearRow, Dollars) -> Dollars) -> Vec<Entry> {
    let short = years.iter().find(|row| row.unfunded > 0);
    short
        .map(|row| {
            let short = compact_money(dollars(row, row.unfunded));
            Entry {
                year: Some(row.year),
                ..Entry::plain(format!("on: unfunded, {short} a year"))
            }
        })
        .into_iter()
        .collect()
}

/// The first year Medicare's surcharges or a cliff cost anything, and in
/// how many years they do.
fn surcharged(years: &[YearRow], dollars: &impl Fn(&YearRow, Dollars) -> Dollars) -> Vec<Entry> {
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
    vec![Entry::dated(first.year, text, (Page::Household, None))]
}

/// Each account's contributions held back, once for each way they were,
/// in the first year they were.
fn held(plan: &Plan, years: &[YearRow]) -> Vec<Entry> {
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
            let opens = (Page::Contributions, into);
            let text = format!("{}: {}", account_name(plan, account), held_words(how));
            found.push(Entry::dated(row.year, text, opens));
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

/// Each person whose benefit the engine computes with no earnings record
/// to compute it from.
fn unrecorded(plan: &Plan) -> Vec<Entry> {
    let each = plan.household.people.iter().enumerate();
    each.filter(|(_, person)| person.earnings.is_empty())
        .filter(|(_, person)| {
            (plan.income.iter())
                .any(|income| income.is_benefit_of(&person.id) && income.is_derived())
        })
        .map(|(at, person)| Entry {
            opens: Some((Page::People, Some(at))),
            ..Entry::plain(format!(
                "{}'s Social Security is computed without an earnings record",
                person.display_name()
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tui::support::{TEST_PLAN, projected_from, test_projected};

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

    fn said(projected: &Projected, draft: &Draft) -> Vec<(String, Option<Target>)> {
        let entries = entries(projected, draft, (true, None));
        (entries.iter())
            .map(|entry| (entry.line(), entry.opens))
            .collect()
    }

    #[test]
    fn a_plan_with_nothing_wanting_says_so() {
        let projected = test_projected();
        let draft = Draft::new(projected.plan.clone(), false);
        let entries = entries(&projected, &draft, (false, None));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, NOTHING);
        assert_eq!(entries[0].tone, Tone::Quiet);
    }

    #[test]
    fn each_kind_says_its_first_year_and_opens_its_item() {
        let projected = projected_from(&wanting());
        assert_eq!(projected.plan.validate(), []);
        let draft = Draft::new(projected.plan.clone(), false);
        let first = &projected.projection.years[0];
        let expected = [
            (
                format!("on: unfunded, {} a year", compact_money(first.unfunded)),
                None,
            ),
            (
                format!("Medicare surcharges, {}", money(first.medicare)),
                Some((Page::Household, None)),
            ),
            (
                "k: held to its limit".to_owned(),
                Some((Page::Contributions, Some(0))),
            ),
        ];
        let said = said(&projected, &draft);
        let unrecorded = (
            "me's Social Security is computed without an earnings record".to_owned(),
            Some((Page::People, Some(0))),
        );
        assert_eq!(said[0], unrecorded, "what has no year leads");
        for (at, (text, opens)) in expected.into_iter().enumerate() {
            assert!(
                said[at + 1].0.starts_with(&format!("2026 {text}")),
                "{said:?}"
            );
            assert_eq!(said[at + 1].1, opens, "{said:?}");
        }
        assert!(said[2].0.ends_with("years in all"), "{said:?}");
        assert_eq!(said.len(), 4, "{said:?}");
    }

    fn starts(plan_text: &str) -> Runs {
        use retiretui_engine::market::{History, Progress, historical};
        use retiretui_engine::params::TaxTables;
        let plan = projected_from(plan_text).plan;
        let tables = TaxTables::embedded();
        historical(&plan, &tables, History::embedded(), &Progress::default()).unwrap()
    }

    #[test]
    fn the_worst_failing_start_leads_to_the_historical_page() {
        let runs = starts(&TEST_PLAN.replace("amount = 60000", "amount = 70000"));
        let worst = &runs.runs[runs.worst_first()[0]];
        let RunName::Start(year) = worst.name else {
            panic!("a historical run is named by its start");
        };
        let failed = runs.runs.len() - runs.successes;
        assert!(failed > 0 && runs.successes > 0, "some starts fail");
        let found = failing(Some(&runs));
        assert_eq!(found.len(), 1);
        let text = format!(
            "Fails from a {year} start · {failed} of {} fail",
            runs.runs.len()
        );
        assert_eq!(found[0].text, text);
        assert_eq!(found[0].year, None, "a start is no year of the plan's");
        assert_eq!(found[0].leads, Some((Page::Historical, None)));
        let modest = TEST_PLAN.replace("amount = 60000", "amount = 20000");
        assert!(
            failing(Some(&starts(&modest))).is_empty(),
            "every start survives"
        );
    }
}
