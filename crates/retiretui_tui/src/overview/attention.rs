//! The Needs attention pane: the client's rows - the draft's issues,
//! where the plan runs short, and what in the projection wants looking
//! at - and the historical starts the plan does not survive.

use retiretui_client::overview::{NOTHING, STALE, View, failing_start, issue_rows};
use retiretui_engine::market::Runs;

use super::rows::{Entry, Tone};
use crate::edit::Draft;
use crate::nav::Page;

/// Every row - the draft's issues and that the figures predate them, where
/// the plan runs short, the worst failing start, and what in the
/// projection wants looking at by year - or a line saying there is
/// nothing; the starts are the plan's `historical` runs, where they have
/// been run.
pub(super) fn entries(view: &View, draft: &Draft, historical: Option<&Runs>) -> Vec<Entry> {
    let warning = |entry| Entry {
        tone: Tone::Warning,
        ..entry
    };
    let mut found: Vec<Entry> = issue_rows(draft).into_iter().map(Entry::from).collect();
    if !found.is_empty() {
        found = found.into_iter().map(warning).collect();
        found.push(quiet(STALE));
    }
    found.extend(view.shortfall.as_ref().map(|shortfall| {
        warning(Entry {
            year: Some(shortfall.year),
            is_dated: false,
            opens: Some((Page::Expenses, None)),
            ..Entry::plain(shortfall.said.clone())
        })
    }));
    found.extend(failing(historical));
    let mut wanting: Vec<Entry> = view.attention.iter().cloned().map(Entry::from).collect();
    wanting.sort_by_key(|entry| entry.year);
    found.extend(wanting);
    if found.is_empty() {
        found.push(quiet(NOTHING));
    }
    found
}

fn quiet(text: &str) -> Entry {
    Entry {
        tone: Tone::Quiet,
        ..Entry::plain(text.to_owned())
    }
}

/// The worst historical start the plan does not survive, leading to the
/// Historical page.
fn failing(historical: Option<&Runs>) -> Option<Entry> {
    let failing = failing_start(historical?)?;
    Some(Entry::leading(failing.said, (Page::Historical, None)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{TEST_PLAN, projected_from, test_projected};

    #[test]
    fn a_plan_with_nothing_wanting_says_so() {
        let projected = test_projected();
        let draft = Draft::new(projected.plan.clone(), false);
        let entries = entries(&View::new(&projected, false), &draft, None);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, NOTHING);
        assert_eq!(entries[0].tone, Tone::Quiet);
    }

    #[test]
    fn a_plan_that_runs_short_says_from_when_and_leads_to_that_year_and_its_spending() {
        let short = projected_from(&TEST_PLAN.replace("amount = 60000", "amount = 95000"));
        let view = View::new(&short, false);
        let year = view.shortfall.as_ref().unwrap().year;
        let draft = Draft::new(short.plan.clone(), false);
        let first = &entries(&view, &draft, None)[0];
        assert!(first.text.starts_with(&format!("Runs short from {year}: ")));
        assert_eq!(first.line(), first.text, "the sentence names its own year");
        assert_eq!((first.year, first.tone), (Some(year), Tone::Warning));
        assert_eq!(first.opens, Some((Page::Expenses, None)));
    }
}
