//! The years a chart points out: a dotted rule up the graph, and the
//! year's name under the axis.

use plurimus::core::ratatui_core::buffer::Buffer;
use plurimus::core::ratatui_core::layout::Rect;
use plurimus::core::ratatui_core::style::Style;

use super::SeriesChart;
use crate::commands::tui::theme::Theme;

#[derive(Clone)]
pub struct Mark {
    pub year: i16,
    pub label: String,
    pub style: Style,
}

impl Mark {
    /// The year cursor, in the accent.
    pub fn cursor(year: i16, theme: &Theme) -> Self {
        Self {
            year,
            label: year.to_string(),
            style: theme.accented(),
        }
    }
}

const RULE: &str = "┊";
const ZERO_RULE: &str = "┈";
/// Braille dots a cell holds down its height.
const DOTS_DOWN: f64 = 4.0;

/// A rule across the graph at zero, where the scale runs below it; drawn
/// first, so the marks and the series cross it.
pub(super) fn draw_zero(chart: &SeriesChart, area: Rect, buf: &mut Buffer) {
    let [bottom, top] = chart.y_bounds;
    if bottom >= 0.0 || top <= bottom {
        return;
    }
    let Some(graph) = chart.graph(area).filter(|graph| !graph.is_empty()) else {
        return;
    };
    let dots = f64::from(graph.height) * DOTS_DOWN - 1.0;
    let dot = (top / (top - bottom) * dots).round();
    let row = graph.y + (dot / DOTS_DOWN) as u16;
    for column in graph.left()..graph.right() {
        if let Some(cell) = buf.cell_mut((column, row)) {
            cell.set_symbol(ZERO_RULE).set_style(chart.axis);
        }
    }
}

/// Drawn before the chart, whose series then cross the rules; the first
/// mark wins a column two share.
pub(super) fn draw_rules(chart: &SeriesChart, area: Rect, buf: &mut Buffer) {
    let Some(graph) = chart.graph(area) else {
        return;
    };
    for mark in chart.marks.iter().rev() {
        let Some(column) = chart.column_in(graph, mark.year) else {
            continue;
        };
        for row in graph.top()..graph.bottom() {
            if let Some(cell) = buf.cell_mut((column, row)) {
                cell.set_symbol(RULE).set_style(mark.style);
            }
        }
    }
}

fn middle_year(chart: &SeriesChart) -> Mark {
    let year = f64::midpoint(chart.x_bounds[0], chart.x_bounds[1]).round() as i16;
    Mark {
        year,
        label: year.to_string(),
        style: chart.axis,
    }
}

/// Each mark's label centred under its rule, then the middle year's, kept
/// clear of the end years; one that would touch an earlier one is left out.
pub(super) fn draw_labels(chart: &SeriesChart, area: Rect, buf: &mut Buffer) {
    let Some(graph) = chart.graph(area).filter(|graph| !graph.is_empty()) else {
        return;
    };
    let last_label = chart.x_labels.last().map_or(0, |label| label.len() as u16);
    let (low, high) = (graph.x + 1, graph.right().saturating_sub(last_label + 1));
    let middle = middle_year(chart);
    let mut taken: Vec<(u16, u16)> = Vec::new();
    for mark in chart.marks.iter().chain([&middle]) {
        let length = mark.label.chars().count() as u16;
        let Some(column) = chart.column_in(graph, mark.year) else {
            continue;
        };
        if chart.x_labels.contains(&mark.label) || low + length > high {
            continue;
        }
        let start = column.saturating_sub(length / 2).clamp(low, high - length);
        let end = start + length;
        if taken.iter().any(|&(from, to)| start <= to && from <= end) {
            continue;
        }
        buf.set_string(start, area.bottom() - 1, &mark.label, mark.style);
        taken.push((start, end));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tui::chart::Series;
    use crate::commands::tui::session::YearCursor;
    use crate::commands::tui::support::{
        SIZE, cell_fg, click_year, composed_frame, frame_to_string, headless_app, overview_chart,
    };
    use plurimus::core::ratatui_core::layout::Position;
    use plurimus::core::ratatui_core::style::Color;
    use plurimus::core::ratatui_core::widgets::Widget;
    use plurimus::ui::ComputedWidgetArea;

    const AREA: Rect = Rect::new(1, 2, 60, 10);

    fn marked(marks: &[(i16, &str)]) -> SeriesChart {
        let points = (2026..=2050).map(|year| (f64::from(year), 1.0)).collect();
        let series = Series {
            label: "flat".to_owned(),
            color: Color::Reset,
            points,
        };
        let marks = marks.iter().map(|&(year, label)| Mark {
            year,
            label: label.to_owned(),
            style: Style::new(),
        });
        let mut chart = SeriesChart::of(vec![series], Style::new());
        chart.marks = marks.collect();
        chart
    }

    fn row_of(buf: &Buffer, row: u16) -> String {
        let frame = frame_to_string(buf);
        frame.lines().nth(usize::from(row)).unwrap().to_owned()
    }

    fn drawn(chart: &SeriesChart) -> Buffer {
        let mut buf = Buffer::empty(Rect::new(0, 0, 62, 13));
        chart.render(AREA, &mut buf);
        buf
    }

    #[test]
    fn a_year_is_drawn_in_the_column_that_reads_back_as_it() {
        let chart = marked(&[]);
        for year in 2026..=2050 {
            let column = chart.column_of(AREA, year).unwrap();
            let read = chart.year_at(AREA, Position::new(column, AREA.y));
            assert_eq!(read, Some(year));
        }
        assert_eq!(chart.column_of(AREA, 2025), None);
        assert_eq!(chart.column_of(AREA, 2051), None);
    }

    #[test]
    fn a_label_gives_way_to_an_earlier_one_and_keeps_its_rule() {
        let chart = marked(&[(2030, "2030"), (2031, "retire 2031"), (2049, "retire 2049")]);
        let buf = drawn(&chart);
        let labels = row_of(&buf, AREA.bottom() - 1);
        assert!(labels.contains(" 2030 "), "{labels}");
        assert!(!labels.contains("2031"), "{labels}");
        assert!(labels.contains(" 2038 "), "the middle year: {labels}");
        assert!(labels.ends_with("retire 2049 2050 "), "shifted: {labels}");
        // The flat series runs along the top row, so the rules are read under it.
        let rules = row_of(&buf, AREA.y + 4);
        assert_eq!(rules.matches(RULE).count(), 3, "{rules}");
        let dropped = chart.column_of(AREA, 2031).unwrap();
        assert_eq!(buf[(dropped, AREA.y + 4)].symbol(), RULE);
    }

    #[test]
    fn a_series_below_zero_lowers_the_scale_and_rules_zero_where_it_is_drawn() {
        let ends = |year| match year {
            2026 => -100_000.0,
            2050 => 100_000.0,
            _ => 0.0,
        };
        let points = (2026..=2050)
            .map(|year| (f64::from(year), ends(year)))
            .collect();
        let series = Series {
            label: "across".to_owned(),
            color: Color::Reset,
            points,
        };
        let chart = SeriesChart::of(vec![series], Style::new());
        assert_eq!(chart.y_labels, ["-105k", "105k"], "5% beyond each end");
        let buf = drawn(&chart);
        let column = chart.column_of(AREA, 2038).unwrap();
        let is_braille = |c: char| ('\u{2801}'..='\u{28ff}').contains(&c);
        let zero = (AREA.top()..AREA.bottom())
            .find(|&row| buf[(column, row)].symbol().chars().any(is_braille))
            .expect("the line at zero");
        let ruled = row_of(&buf, zero);
        assert!(ruled.contains(ZERO_RULE), "{}", frame_to_string(&buf));
        let mut others = (AREA.top()..AREA.bottom()).filter(|&row| row != zero);
        assert!(others.all(|row| !row_of(&buf, row).contains(ZERO_RULE)));

        let above = marked(&[]);
        assert_eq!(above.y_labels[0], "0");
        assert!(!frame_to_string(&drawn(&above)).contains(ZERO_RULE));
    }

    #[test]
    fn a_press_moves_the_accent_mark() {
        let mut app = headless_app(SIZE);
        app.update();
        let chart = overview_chart(&mut app);
        click_year(&mut app, chart, 2038);
        app.update();
        let year = app.world().resource::<YearCursor>().0.unwrap();
        // The pane's note names the year too, so it is looked for under the axis.
        let area = app.world().get::<ComputedWidgetArea>(chart).unwrap().0;
        let row = area.bottom() - 1;
        let frame = composed_frame(&app);
        let labels = frame.lines().nth(usize::from(row)).unwrap();
        let at = labels
            .find(&format!(" {year} "))
            .expect("the year under the axis");
        let column = labels[..at].chars().count() as u16 + 1;
        let accent = Theme::terminal().accented().fg;
        assert_eq!(cell_fg(&app, column, row), accent, "{year}: {frame}");
    }
}
