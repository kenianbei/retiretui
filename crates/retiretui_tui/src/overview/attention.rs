//! The Needs attention pane: the client's rows - the draft's issues and
//! what in the projection wants looking at - and the historical starts the
//! plan does not survive.

use retiretui_client::overview::{NOTHING, attention, failing_start, issue_rows};
use retiretui_engine::market::Runs;

use super::rows::{Entry, Tone};
use crate::edit::Draft;
use crate::nav::Page;
use crate::session::Projected;

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

/// The worst historical start the plan does not survive, leading to the
/// Historical page.
fn failing(historical: Option<&Runs>) -> Option<Entry> {
    let failing = failing_start(historical?)?;
    Some(Entry::leading(failing.said, (Page::Historical, None)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::test_projected;

    #[test]
    fn a_plan_with_nothing_wanting_says_so() {
        let projected = test_projected();
        let draft = Draft::new(projected.plan.clone(), false);
        let entries = entries(&projected, &draft, (false, None));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, NOTHING);
        assert_eq!(entries[0].tone, Tone::Quiet);
    }
}
