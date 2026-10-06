//! The Over the plan and Rests on panes: the client's lifetime totals and
//! assumptions as rows, each leading where the client says. A total says
//! what it is made of on a line of its own where the pane has the rows,
//! and has it read out beneath the list while highlighted where not.

use retiretui_client::overview::{Leads, Tool, Total, View};
use retiretui_client::searches::markets::Assumption;

use super::rows::Entry;
use crate::edit::page_of;
use crate::layout;
use crate::nav::Page;

/// The cells between a label and what it labels.
const GAP: usize = 2;

/// The cells a column of `labels` is set in: the longest and the gap.
fn column<'a>(labels: impl Iterator<Item = &'a str>) -> usize {
    let longest = labels.map(layout::cells_of).max().unwrap_or(0);
    usize::from(longest) + GAP
}

/// Whether a list `rows` tall holds every total of `view` with what it is
/// made of beneath it.
pub(super) fn fits_in(view: &View, rows: u16) -> bool {
    let wanted = view.totals.len() * 2;
    usize::from(rows) >= wanted
}

const fn page_of_tool(tool: Tool) -> Page {
    match tool {
        Tool::RothConversions => Page::RothConversions,
        Tool::WithdrawalOrder => Page::WithdrawalOrder,
        Tool::TaxTables => Page::TaxTables,
    }
}

/// `entry` leading where `leads` says.
fn led(entry: Entry, leads: Option<Leads>) -> Entry {
    match leads {
        Some(Leads::Place((domain, index))) => Entry {
            opens: Some((page_of(domain), index)),
            ..entry
        },
        Some(Leads::Tool(tool)) => Entry {
            leads: Some((page_of_tool(tool), None)),
            ..entry
        },
        Some(Leads::Year(year)) => Entry {
            year: Some(year),
            ..entry
        },
        None => entry,
    }
}

/// A row for each total, and where `is_roomy` a quiet one under it saying
/// what it is made of, leading where the total does.
pub(super) fn entries(totals: &[Total], is_roomy: bool) -> Vec<Entry> {
    let mut entries = Vec::with_capacity(totals.len() * 2);
    let width = column(totals.iter().map(|total| total.label));
    for total in totals {
        let figure = Entry {
            label: format!("{:<width$}", total.label),
            ..Entry::plain(total.amount.clone())
        };
        let figure = led(figure, total.leads);
        if !is_roomy {
            entries.push(Entry {
                beneath: Some(total.made_of.clone()),
                ..figure
            });
            continue;
        }
        entries.push(figure);
        if !total.made_of.is_empty() {
            let made_of = Entry {
                label: layout::CONTINUED.to_owned(),
                ..Entry::quiet(&total.made_of)
            };
            entries.push(led(made_of, total.leads));
        }
    }
    entries
}

/// A row for each assumption, opening the page it is edited on.
pub(super) fn assumed(rests_on: &[Assumption]) -> Vec<Entry> {
    let width = column(rests_on.iter().map(|row| row.label));
    let rows = rests_on.iter().map(|row| Entry {
        label: format!("{:<width$}", row.label),
        opens: Some((page_of(row.domain), None)),
        ..Entry::plain(row.value.clone())
    });
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::super::rows::Tone;
    use super::*;
    use crate::support::test_projected;

    fn view() -> View {
        View::new(&test_projected(), true)
    }

    #[test]
    fn a_roomy_pane_says_each_make_up_under_its_total_and_a_tight_one_beneath_the_list() {
        let view = view();
        assert!(fits_in(&view, 14) && !fits_in(&view, 13));
        let roomy = entries(&view.totals, true);
        let income = format!("Income       {}", view.totals[0].amount);
        assert_eq!(roomy[0].lines(40), [income]);
        assert_eq!(roomy[1].lines(40), ["  salary 100%"]);
        assert_eq!(
            (roomy[1].tone, roomy[1].opens),
            (Tone::Quiet, roomy[0].opens)
        );
        assert!(roomy.iter().all(|entry| entry.beneath.is_none()));

        let tight = entries(&view.totals, false);
        assert_eq!(tight.len(), 7);
        assert_eq!(tight[0].beneath.as_deref(), Some("salary 100%"));
        assert_eq!(
            tight[5].beneath.as_deref(),
            Some(""),
            "a row keeps its lines"
        );
    }

    #[test]
    fn each_total_leads_to_its_page_its_tool_or_its_year() {
        let tight = entries(&view().totals, false);
        assert_eq!(tight[0].opens, Some((Page::Income, None)));
        assert_eq!(tight[1].leads, Some((Page::WithdrawalOrder, None)));
        assert_eq!(tight[3].leads, Some((Page::TaxTables, None)));
        assert_eq!(tight[4].leads, Some((Page::RothConversions, None)));

        let dated = led(Entry::plain("Required".to_owned()), Some(Leads::Year(2048)));
        assert_eq!(dated.year, Some(2048));
        assert_eq!(dated.lines(40), ["Required"], "the year leads nothing");
    }

    #[test]
    fn each_assumption_opens_the_page_it_is_edited_on() {
        let rows = assumed(&view().rests_on);
        assert_eq!(rows[0].lines(40), ["Runs through   2050 · to age 70"]);
        assert_eq!(
            rows[0].lines(25),
            ["Runs through   2050 · to", "               age 70"],
            "a value too wide hangs under itself"
        );
        assert_eq!(rows[0].opens, Some((Page::Settings, None)));
        assert_eq!(rows[2].opens, Some((Page::Market, None)));
    }
}
