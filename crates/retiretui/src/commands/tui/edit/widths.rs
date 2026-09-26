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
    /// The widest of its cells, the header aside.
    content: u16,
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
                let content = widest(rows, at);
                Dealt {
                    width: content.max(cells_of(&header)),
                    content,
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

/// The widest of the cells in the column at `at`.
fn widest(rows: &[sort::Item], at: usize) -> u16 {
    let cells = rows.iter().filter_map(|(_, cells)| cells.get(at));
    cells.fold(0, |widest, cell| widest.max(cells_of(&cell.text)))
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
        None => headed_off(columns, wanted - usable).unwrap_or_else(|| squeezed(columns, usable)),
    }
}

/// What a column of words is wider than its cells for its header alone,
/// down to the floor.
fn header_excess(column: &Dealt) -> u16 {
    if column.is_numeric {
        return 0;
    }
    column.width - column.content.max(FLOOR).min(column.width)
}

/// The columns `short` cells narrower where their headers alone can give
/// them up, the widest excess first: a clipped header still names its
/// column, where a clipped cell loses what the row holds.
fn headed_off(columns: &[Dealt], short: u16) -> Option<Vec<u16>> {
    let excess: Vec<u16> = columns.iter().map(header_excess).collect();
    let total = excess
        .iter()
        .fold(0u16, |total, cut| total.saturating_add(*cut));
    let room = total.checked_sub(short)?;
    let kept = level(excess.clone(), room);
    let levelled = excess
        .iter()
        .fold(0u16, |total, cut| total + (*cut).min(kept));
    let mut left = room - levelled;
    let given = columns.iter().zip(excess).map(|(column, cut)| {
        let extra = u16::from(cut > kept && left > 0);
        left -= extra;
        column.width - cut + cut.min(kept) + extra
    });
    Some(given.collect())
}

/// Every column as wide as it asked, the spare dealt over the columns of
/// words a cell at a time from the left and capped, so only what is past
/// the cap stays empty.
fn widened(columns: &[Dealt], spare: u16) -> Vec<u16> {
    let words = columns.iter().filter(|column| !column.is_numeric).count();
    let words = u16::try_from(words).unwrap_or(u16::MAX);
    let dealt = spare.min(words.saturating_mul(SPARE));
    let mut nth_word = 0;
    let mut given = |column: &Dealt| {
        if column.is_numeric {
            return column.width;
        }
        let extra = dealt / words + u16::from(nth_word < dealt % words);
        nth_word += 1;
        column.width + extra
    };
    columns.iter().map(&mut given).collect()
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
            content: wanted,
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
    fn the_spare_past_an_even_share_goes_to_the_first_words() {
        let columns = [word(10), number(8), word(6), word(4)];
        let spare = 5;
        let given = 10 + 8 + 6 + 4 + layout::CURSOR_COLS + GAP * 3 + spare;
        assert_eq!(share(&columns, given), [12, 8, 8, 5]);
    }

    #[test]
    fn a_squeeze_clips_long_headers_before_any_cell() {
        let headed = |content, header| Dealt {
            content,
            ..word(header)
        };
        let columns = [word(11), headed(8, 12), headed(8, 10), word(15)];
        let given = 11 + 12 + 10 + 15 + layout::CURSOR_COLS + GAP * 3 - 3;
        assert_eq!(
            share(&columns, given),
            [11, 10, 9, 15],
            "the widest header gives way first and every cell stays whole"
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
