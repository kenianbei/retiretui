//! How a table's width is shared out over its columns, and what a cell
//! reads as once it has one. A column is as wide as what it holds, so a
//! long name is never clipped beside a column of spare cells.

use plurimus::core::ratatui_core::layout::Constraint;
use plurimus::core::ratatui_core::text::Line;
use plurimus::widgets::{TableColumns, TableLayout};

use super::cells::Cell;
use super::sort;
use super::table::DomainTable;
use crate::commands::tui::layout::{self, cells_of, clipped};

/// The most a column is widened past what it holds. A wide terminal
/// keeps a row's values within a glance of each other rather than
/// strewing them across the screen.
const SPARE: u16 = 8;

/// The narrowest a column of words is squeezed to before its cells are
/// clipped instead.
const FLOOR: u16 = 6;

/// One column: what it asked for, until the share says what it gets.
struct Dealt {
    width: u16,
    header: String,
    /// A number keeps its width whatever the squeeze: a clipped figure
    /// reads as a different figure, which a clipped word does not.
    is_numeric: bool,
}

/// A table's columns, laid out over the width it was given.
pub struct Laid(Vec<Dealt>);

impl Laid {
    /// Lays `table`'s columns out over `given` cells, wide enough for
    /// `rows` where the width allows.
    pub fn of(table: &DomainTable, rows: &[sort::Item], given: u16) -> Self {
        let fields = table.ops.fields;
        let mut columns: Vec<Dealt> = table
            .list
            .columns
            .iter()
            .enumerate()
            .map(|(at, column)| {
                let header = heading(column.header(fields), table.sort, at);
                Dealt {
                    width: widest(&header, rows, at),
                    header,
                    is_numeric: column.is_numeric(fields),
                }
            })
            .collect();
        let shared = share(&columns, given);
        for (column, width) in columns.iter_mut().zip(shared) {
            column.width = width;
        }
        Self(columns)
    }

    pub fn columns(&self) -> TableColumns {
        let widths = self.0.iter().map(|column| Constraint::Length(column.width));
        TableColumns(widths.collect())
    }

    pub fn header(&self) -> Vec<Line<'static>> {
        let headers = self.0.iter().map(|column| column.header.clone());
        self.lines(headers)
    }

    pub fn row(&self, cells: Vec<Cell>) -> Vec<Line<'static>> {
        self.lines(cells.into_iter().map(|cell| cell.text))
    }

    /// One line per column, each cut to the width its column was given.
    fn lines(&self, texts: impl Iterator<Item = String>) -> Vec<Line<'static>> {
        self.0
            .iter()
            .zip(texts)
            .map(|(column, text)| {
                let line = Line::from(clipped(text, column.width));
                if column.is_numeric {
                    line.right_aligned()
                } else {
                    line
                }
            })
            .collect()
    }
}

/// The column name, with the glyph saying which way the table is ordered
/// by it.
fn heading(label: &str, sort: Option<super::sort::Sort>, at: usize) -> String {
    match sort.and_then(|sort| sort.marks(at)) {
        Some(glyph) => format!("{label} {glyph}"),
        None => label.to_owned(),
    }
}

/// The widest the column at `at` has to be to show everything in it.
fn widest(header: &str, rows: &[sort::Item], at: usize) -> u16 {
    let cells = rows.iter().filter_map(|(_, cells)| cells.get(at));
    cells.fold(cells_of(header), |widest, cell| {
        widest.max(cells_of(&cell.text))
    })
}

/// Shares `given` cells out over the columns: what each holds where they
/// all fit, and a squeeze on the columns of words where they do not.
fn share(columns: &[Dealt], given: u16) -> Vec<u16> {
    let gaps = u16::try_from(columns.len().saturating_sub(1)).unwrap_or_default();
    let spacing = gaps.saturating_mul(TableLayout::default().column_spacing);
    let usable = given.saturating_sub(layout::CURSOR_COLS.saturating_add(spacing));
    let wanted = columns
        .iter()
        .fold(0u16, |total, column| total.saturating_add(column.width));
    match usable.checked_sub(wanted) {
        Some(spare) => widened(columns, spare),
        None => squeezed(columns, usable),
    }
}

/// Every column as wide as it asked, the spare dealt evenly over the
/// columns of words and capped, so what is left over stays empty.
fn widened(columns: &[Dealt], spare: u16) -> Vec<u16> {
    let words = columns.iter().filter(|column| !column.is_numeric).count();
    let each = match u16::try_from(words) {
        Ok(words) if words > 0 => (spare / words).min(SPARE),
        _ => 0,
    };
    let given = |column: &Dealt| column.width + u16::from(!column.is_numeric) * each;
    columns.iter().map(given).collect()
}

/// The numbers keep their width and the words give up theirs, down to
/// [`FLOOR`], below which the cells are clipped. The first column of
/// words is the item's name, which is how a row is told from the next, so
/// it is kept whole while the others can still stand at the floor; the
/// rest give way widest first, so a short column is not cut to spare a
/// long one.
fn squeezed(columns: &[Dealt], usable: u16) -> Vec<u16> {
    let numbers = columns
        .iter()
        .filter(|column| column.is_numeric)
        .fold(0u16, |total, column| total.saturating_add(column.width));
    let left = usable.saturating_sub(numbers);
    let name = columns.iter().position(|column| !column.is_numeric);
    let others: Vec<u16> = columns
        .iter()
        .enumerate()
        .filter(|(at, column)| !column.is_numeric && Some(*at) != name)
        .map(|(_, column)| column.width)
        .collect();
    let floors = others
        .iter()
        .fold(0u16, |total, width| total.saturating_add(FLOOR.min(*width)));
    let kept = name.map_or(0, |at| {
        floored(columns[at].width, left.saturating_sub(floors))
    });
    let level = level(others, left.saturating_sub(kept));
    let given = |(at, column): (usize, &Dealt)| {
        if column.is_numeric {
            column.width
        } else if Some(at) == name {
            kept
        } else {
            floored(column.width, level)
        }
    };
    columns.iter().enumerate().map(given).collect()
}

/// `wanted` cut to `room`, never past the floor and never wider than it
/// asked for.
fn floored(wanted: u16, room: u16) -> u16 {
    room.clamp(FLOOR.min(wanted), wanted)
}

/// The width the widest of `widths` are cut to so that together they
/// take `room`, the narrower ones being left as they are.
fn level(mut widths: Vec<u16>, mut room: u16) -> u16 {
    widths.sort_unstable();
    for (at, width) in widths.iter().enumerate() {
        let sharing = u16::try_from(widths.len() - at).unwrap_or(u16::MAX);
        let share = room / sharing;
        if *width > share {
            return share;
        }
        room -= width;
    }
    u16::MAX
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cell the table keeps between two columns.
    const GAP: u16 = 1;

    fn column(wanted: u16, is_numeric: bool) -> Dealt {
        Dealt {
            width: wanted,
            header: String::new(),
            is_numeric,
        }
    }

    fn word(wanted: u16) -> Dealt {
        column(wanted, false)
    }

    fn number(wanted: u16) -> Dealt {
        column(wanted, true)
    }

    #[test]
    fn a_table_with_room_gives_every_column_what_it_holds() {
        let columns = [word(10), number(8)];
        let given = 10 + 8 + layout::CURSOR_COLS + GAP;
        assert_eq!(share(&columns, given), [10, 8]);
    }

    #[test]
    fn the_spare_goes_to_the_words_and_is_capped() {
        let columns = [word(10), number(8)];
        let roomy = 10 + 8 + layout::CURSOR_COLS + GAP + 4;
        assert_eq!(share(&columns, roomy), [14, 8], "the spare widens the word");
        let vast = roomy + 400;
        assert_eq!(
            share(&columns, vast),
            [10 + SPARE, 8],
            "and stops at the cap"
        );
    }

    #[test]
    fn a_squeeze_takes_from_the_words_and_never_from_the_numbers() {
        let columns = [word(20), word(10), number(8)];
        let tight = 8 + 20 + layout::CURSOR_COLS + GAP * 2;
        let shared = share(&columns, tight);
        assert_eq!(shared[2], 8, "the figure keeps every digit");
        assert!(shared[0] < 20 && shared[1] < 10, "{shared:?}");
        assert!(shared[0] > shared[1], "the wider column keeps more");
    }

    #[test]
    fn a_squeeze_keeps_the_name_and_cuts_the_widest_of_the_rest() {
        let columns = [word(12), number(8), word(20), word(18), word(7)];
        let given = 12 + 8 + 12 + 12 + 7 + layout::CURSOR_COLS + GAP * 4;
        assert_eq!(
            share(&columns, given),
            [12, 8, 12, 12, 7],
            "the two long columns meet, and the short one is left alone"
        );
    }

    #[test]
    fn a_squeezed_column_stops_at_the_floor() {
        let columns = [word(30), number(40)];
        let shared = share(&columns, 44);
        assert_eq!(shared, [FLOOR, 40]);
    }

    #[test]
    fn a_column_narrower_than_the_floor_is_not_widened_by_it() {
        let columns = [word(3), number(40)];
        assert_eq!(share(&columns, 44), [3, 40], "a short word stays short");
    }
}
