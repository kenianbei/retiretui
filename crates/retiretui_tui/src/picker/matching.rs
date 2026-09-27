//! Which offered rows a query leaves, best first, and where each matched.

use std::cmp::Reverse;

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use plurimus::core::ratatui_core::style::Style;
use plurimus::core::ratatui_core::text::{Line, Span};

use super::Offered;

/// The rows `query` leaves, best match first and offered order on a tie,
/// each carrying where it matched. An empty query leaves them all.
#[must_use]
pub fn ranked(query: &str, offered: impl IntoIterator<Item = Offered>) -> Vec<Offered> {
    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
    let mut left: Vec<(u32, usize, Offered)> = offered
        .into_iter()
        .enumerate()
        .filter_map(|(at, mut row)| {
            let (score, indices) = found(&pattern, &row.text, &mut matcher)?;
            row.indices = indices;
            Some((score, at, row))
        })
        .collect();
    left.sort_by_key(|(score, at, _)| (Reverse(*score), *at));
    left.into_iter().map(|(_, _, row)| row).collect()
}

// The matcher appends indices unsorted and may repeat one.
fn found(pattern: &Pattern, text: &str, matcher: &mut Matcher) -> Option<(u32, Vec<u32>)> {
    let mut haystack = Vec::new();
    let mut indices = Vec::new();
    let score = pattern.indices(Utf32Str::new(text, &mut haystack), matcher, &mut indices)?;
    indices.sort_unstable();
    indices.dedup();
    Some((score, indices))
}

/// `text` with the characters at `indices` drawn in `lit`.
#[must_use]
pub fn lit_line(text: &str, indices: &[u32], lit: Style) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut run = String::new();
    let mut is_run_lit = false;
    for (at, character) in text.chars().enumerate() {
        let is_lit = u32::try_from(at).is_ok_and(|at| indices.binary_search(&at).is_ok());
        if is_lit != is_run_lit && !run.is_empty() {
            spans.push(styled(std::mem::take(&mut run), is_run_lit, lit));
        }
        is_run_lit = is_lit;
        run.push(character);
    }
    if !run.is_empty() {
        spans.push(styled(run, is_run_lit, lit));
    }
    Line::from(spans)
}

fn styled(run: String, is_lit: bool, lit: Style) -> Span<'static> {
    if is_lit {
        Span::styled(run, lit)
    } else {
        Span::raw(run)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offered(texts: &[&str]) -> Vec<Offered> {
        texts
            .iter()
            .enumerate()
            .map(|(id, text)| Offered::new(id, *text))
            .collect()
    }

    #[test]
    fn an_empty_query_leaves_every_row_in_the_order_offered() {
        let rows = ranked("", offered(&["save", "quit"]));
        let ids: Vec<usize> = rows.iter().map(|row| row.id).collect();
        assert_eq!(ids, [0, 1]);
    }

    #[test]
    fn a_query_drops_what_it_does_not_match_and_ranks_the_rest() {
        let rows = ranked("led", offered(&["reload", "ledger", "tab-next"]));
        assert_eq!(rows[0].text, "ledger", "a prefix outranks a scatter");
        assert!(rows.iter().all(|row| row.text != "tab-next"));
        assert_eq!(rows[0].indices, [0, 1, 2]);
    }

    #[test]
    fn matched_characters_are_the_ones_lit() {
        let lit = Style::new().fg(plurimus::core::ratatui_core::style::Color::Cyan);
        let line = lit_line("ledger", &[0, 1, 4], lit);
        let drawn: Vec<(&str, bool)> = line
            .spans
            .iter()
            .map(|span| (span.content.as_ref(), span.style == lit))
            .collect();
        assert_eq!(
            drawn,
            [("le", true), ("dg", false), ("e", true), ("r", false)]
        );
    }
}
