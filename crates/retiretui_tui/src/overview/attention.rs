//! The Needs attention pane: the client's rows - the draft's issues and
//! what in the projection wants looking at - and the historical starts the
//! plan does not survive.

use retiretui_client::overview::{NOTHING, attention, issue_rows};
use retiretui_engine::market::{RunName, Runs};

use super::rows::{Entry, Tone};
use crate::edit::Draft;
use crate::nav::Page;
use crate::session::Projected;
use crate::tools::count_text;

/// Every row, what has no year first and the rest by year, or a line
/// saying there is nothing; the starts are the plan's `historical` runs,
/// where they have been run.
pub(super) fn entries(
    projected: &Projected,
    draft: &Draft,
    (nominal, historical): (bool, Option<&Runs>),
) -> Vec<Entry> {
    let issues = issue_rows(draft).into_iter().map(|row| Entry {
        tone: Tone::Warning,
        ..Entry::from(row)
    });
    let wanting = attention(projected, nominal).into_iter().map(Entry::from);
    let mut found: Vec<Entry> = issues.chain(failing(historical)).chain(wanting).collect();
    found.sort_by_key(|entry| entry.year);
    if found.is_empty() {
        found.push(Entry {
            tone: Tone::Quiet,
            ..Entry::plain(NOTHING.to_owned())
        });
    }
    found
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{TEST_PLAN, projected_from, test_projected};

    #[test]
    fn a_plan_with_nothing_wanting_says_so() {
        let projected = test_projected();
        let draft = Draft::new(projected.plan.clone(), false);
        let entries = entries(&projected, &draft, (false, None));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, NOTHING);
        assert_eq!(entries[0].tone, Tone::Quiet);
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
