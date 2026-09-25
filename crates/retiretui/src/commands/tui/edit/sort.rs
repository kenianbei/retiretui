//! The order a table shows its items in. It is the view's alone: a row
//! keeps the index of the item it stands for, so the plan's own order is
//! what every edit addresses, and what the file keeps.

use std::cmp::Ordering;

use bevy_ecs::prelude::{On, Query};
use plurimus::widgets::TableHeaderClick;

use super::cells::Cell;
use super::commands::FocusedTable;
use super::table::DomainTable;
use crate::commands::tui::command::Outcome;

const ASCENDING: &str = "▲";
const DESCENDING: &str = "▼";

/// The column a table is ordered by, and which way.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Sort {
    column: usize,
    is_descending: bool,
}

impl Sort {
    const fn ascending(column: usize) -> Self {
        Self {
            column,
            is_descending: false,
        }
    }

    /// What a press on `column`'s header makes of the order held: up, then
    /// down, then the plan's own.
    fn pressed(held: Option<Self>, column: usize) -> Option<Self> {
        match held {
            Some(sort) if sort.column == column && sort.is_descending => None,
            Some(sort) if sort.column == column => Some(Self {
                is_descending: true,
                ..sort
            }),
            _ => Some(Self::ascending(column)),
        }
    }

    /// What the `sort` command makes of the order held: each of `columns`
    /// up and then down, and after the last the plan's own.
    fn stepped(held: Option<Self>, columns: usize) -> Option<Self> {
        let next = match held {
            None => Self::ascending(0),
            Some(sort) if sort.is_descending => Self::ascending(sort.column + 1),
            Some(sort) => Self {
                is_descending: true,
                ..sort
            },
        };
        (next.column < columns).then_some(next)
    }

    /// What column `at`'s header says of the order, where it is the one
    /// ordered by.
    pub fn marks(self, at: usize) -> Option<&'static str> {
        let glyph = if self.is_descending {
            DESCENDING
        } else {
            ASCENDING
        };
        (self.column == at).then_some(glyph)
    }

    /// Orders `items`, each the index of an item and its cells. A column
    /// whose every stated cell is a number is ordered as numbers; cells
    /// that say nothing come last either way, and items that tie keep the
    /// plan's order.
    pub fn order(self, items: Vec<Item>) -> Vec<Item> {
        let column = self.column;
        let is_numeric = items
            .iter()
            .map(|item| cell(item, column))
            .all(|said| said.text.is_empty() || said.number.is_some());
        let mut keyed: Vec<(Key, Item)> = items
            .into_iter()
            .map(|item| (Key::of(cell(&item, column), is_numeric), item))
            .collect();
        keyed.sort_by(|(left, (left_at, _)), (right, (right_at, _))| {
            left.compare(right, self.is_descending)
                .then(left_at.cmp(right_at))
        });
        keyed.into_iter().map(|(_, item)| item).collect()
    }
}

/// An item's index in the plan, and its cells.
pub type Item = (usize, Vec<Cell>);

const UNSTATED: &Cell = &Cell {
    text: String::new(),
    number: None,
};

fn cell(item: &Item, column: usize) -> &Cell {
    item.1.get(column).unwrap_or(UNSTATED)
}

enum Key {
    Number(f64),
    Text(String),
    Unstated,
}

impl Key {
    /// A number is ordered as the number it is rather than as the text
    /// that shows it, which `$90,000` and `$450,000` would order wrongly.
    fn of(said: &Cell, is_numeric: bool) -> Self {
        match (said.text.is_empty(), is_numeric) {
            (true, _) => Self::Unstated,
            (false, true) => said.number.map_or(Self::Unstated, Self::Number),
            (false, false) => Self::Text(said.text.clone()),
        }
    }

    fn compare(&self, other: &Self, is_descending: bool) -> Ordering {
        let stated = match (self, other) {
            (Self::Number(left), Self::Number(right)) => left.total_cmp(right),
            (Self::Text(left), Self::Text(right)) => left.cmp(right),
            (Self::Unstated, Self::Unstated) => return Ordering::Equal,
            (Self::Unstated, _) => return Ordering::Greater,
            (_, Self::Unstated) => return Ordering::Less,
            (Self::Number(_), Self::Text(_)) | (Self::Text(_), Self::Number(_)) => Ordering::Equal,
        };
        if is_descending {
            stated.reverse()
        } else {
            stated
        }
    }
}

pub fn handle_header_click(click: On<TableHeaderClick>, mut tables: Query<&mut DomainTable>) {
    if let Ok(mut table) = tables.get_mut(click.entity) {
        table.sort = Sort::pressed(table.sort, click.column);
    }
}

/// The `sort` command, on the table the keyboard is in or whose item it
/// is in.
pub fn sort(mut focused: FocusedTable) -> Outcome {
    let acting = focused.acting().map(|(entity, ..)| entity);
    let Some((mut table, _)) = acting.and_then(|table| focused.tables.get_mut(table).ok()) else {
        return Outcome::Refused("nothing to sort here".to_owned());
    };
    table.sort = Sort::stepped(table.sort, table.list.columns.len());
    Outcome::Done
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tui::present::parse_money;

    /// One-column items, each cell a number where its text is money or
    /// digits, as a table's own cells are.
    fn items(cells: &[&str]) -> Vec<Item> {
        let rows = cells.iter().map(|text| {
            let number = parse_money(text).map(|amount| amount as f64);
            vec![Cell {
                text: (*text).to_owned(),
                number,
            }]
        });
        rows.enumerate().collect()
    }

    fn order_of(items: &[Item]) -> Vec<usize> {
        items.iter().map(|(at, _)| *at).collect()
    }

    #[test]
    fn a_column_of_numbers_is_ordered_as_numbers() {
        let balances = Sort::ascending(0).order(items(&["$40,000", "$300,000", "", "$9,000"]));
        assert_eq!(order_of(&balances), [3, 0, 1, 2], "by amount, not as text");
        let descending = Sort::pressed(Some(Sort::ascending(0)), 0).unwrap();
        let balances = descending.order(balances);
        assert_eq!(order_of(&balances), [1, 0, 3, 2], "the unstated stay last");
    }

    #[test]
    fn a_column_with_a_word_in_it_is_ordered_as_text_and_ties_keep_plan_order() {
        let kinds = Sort::ascending(0).order(items(&["ira", "401k", "ira", "cash"]));
        assert_eq!(order_of(&kinds), [1, 3, 0, 2]);
    }

    #[test]
    fn a_header_goes_up_then_down_then_back_to_the_plan_s_order() {
        let up = Sort::pressed(None, 2);
        assert_eq!(up, Some(Sort::ascending(2)));
        let down = Sort::pressed(up, 2);
        assert!(down.is_some_and(|sort| sort.is_descending));
        assert_eq!(Sort::pressed(down, 2), None);
        assert_eq!(Sort::pressed(down, 1), Some(Sort::ascending(1)));
    }

    #[test]
    fn the_command_walks_every_column_and_ends_on_the_plan_s_order() {
        let mut held = None;
        let mut seen = Vec::new();
        for _ in 0..4 {
            held = Sort::stepped(held, 2);
            seen.push(held.map(|sort| (sort.column, sort.is_descending)));
        }
        let walked = [(0, false), (0, true), (1, false), (1, true)].map(Some);
        assert_eq!(seen, walked);
        assert_eq!(Sort::stepped(held, 2), None);
    }
}
