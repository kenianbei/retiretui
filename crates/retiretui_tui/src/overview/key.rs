//! The key under the Overview's chart: what each mark is, and under the
//! pointer what each reads that year.

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{Component, Query, Ref, Res, With};
use plurimus::core::UiWidget;
use plurimus::core::ratatui_core::style::Style;
use plurimus::core::ratatui_core::text::{Line, Span};
use plurimus::term::CursorCell;
use plurimus::ui::ComputedWidgetArea;
use plurimus::widgets::ratatui_widgets::paragraph::Paragraph;
use retiretui_client::present::money;
use retiretui_engine::plan::Dollars;

use super::charts::OverviewChart;
use crate::chart::{Series, SeriesChart};
use crate::present::compact_money;
use crate::theme::Theme;

/// How many of its glyph a mark is keyed by.
const KEY_SWATCH: usize = 2;
/// What a line is keyed by.
const LINE: &str = "─";

/// A mark the key names, and what it reads each year: one figure, or the
/// two a band lies between.
#[derive(Clone)]
pub(super) struct Keyed {
    pub swatch: Span<'static>,
    pub label: String,
    pub reads: Vec<(i16, Dollars, Dollars)>,
}

impl Keyed {
    pub fn of(swatch: Span<'static>, series: &Series) -> Self {
        let reads = series.points.iter();
        Self {
            swatch,
            label: series.label.clone(),
            reads: reads
                .map(|&(year, amount)| (year as i16, amount as Dollars, amount as Dollars))
                .collect(),
        }
    }

    /// The mark, its name, and what it reads in `year` where one is given,
    /// in the `said` form.
    fn spans(&self, year: Option<i16>, said: fn(Dollars) -> String) -> [Span<'static>; 2] {
        let read = year.and_then(|year| self.reads.iter().find(|&&(at, ..)| at == year));
        let figure = match read {
            None => String::new(),
            Some(&(_, low, high)) if low == high => format!(" {}", said(low)),
            Some(&(_, low, high)) => format!(" {} to {}", said(low), said(high)),
        };
        let named = format!(" {}{figure}  ", self.label);
        [self.swatch.clone(), Span::raw(named)]
    }
}

/// The line under the chart: what each mark is, or what the chart waits
/// on while it has none.
#[derive(Component, Default)]
pub(crate) struct ChartKey {
    pub(super) keyed: Vec<Keyed>,
    pub(super) waits_on: String,
}

/// Rewrites the key whenever what it names moves or the pointer does: its
/// marks' names, and under the pointer what each reads that year, in full
/// where the line holds them and compact where it does not.
pub(super) fn read_key(
    cursor: Res<CursorCell>,
    charts: Query<(&SeriesChart, &ComputedWidgetArea), With<OverviewChart>>,
    mut keys: Query<(Ref<ChartKey>, &ComputedWidgetArea, &mut UiWidget)>,
    theme: Res<Theme>,
) {
    let is_moved = cursor.is_changed() || theme.is_changed();
    let year = charts.iter().next().and_then(|(chart, area)| {
        let cell = cursor.0?;
        chart.year_at(area.0, cell)
    });
    for (key, area, mut widget) in &mut keys {
        if !is_moved && !key.is_changed() {
            continue;
        }
        let line = if key.keyed.is_empty() {
            Line::styled(key.waits_on.clone(), theme.dimmed())
        } else {
            fitted(&key.keyed, year, area.0.width)
        };
        *widget = UiWidget::new(Paragraph::new(line));
    }
}

/// The key at `year`: its figures whole where `width` holds them, and
/// compact where it does not.
fn fitted(keyed: &[Keyed], year: Option<i16>, width: u16) -> Line<'static> {
    let full = key_line(keyed, year, money);
    if full.width() <= usize::from(width) {
        full
    } else {
        key_line(keyed, year, compact_money)
    }
}

pub(super) fn key_line(
    keyed: &[Keyed],
    year: Option<i16>,
    said: fn(Dollars) -> String,
) -> Line<'static> {
    let spans = keyed.iter().flat_map(|mark| mark.spans(year, said));
    Line::from(spans.collect::<Vec<_>>())
}

/// A mark as the key shows it: `symbol` twice over, in `style`.
pub(super) fn swatch(symbol: &str, style: Style) -> Span<'static> {
    Span::styled(symbol.repeat(KEY_SWATCH), style)
}

/// A line as the key shows it, in `style`.
pub(super) fn line_swatch(style: Style) -> Span<'static> {
    swatch(LINE, style)
}

#[cfg(test)]
mod tests {
    use super::super::charts::{balances, income_taxes, lines, markets};
    use super::*;
    use crate::support::{projected_from, test_projected};

    fn said(line: &Line) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    #[test]
    fn the_key_names_each_mark_and_reads_it_at_the_pointer_s_year() {
        let projected = test_projected();
        let theme = Theme::terminal();
        let (_, keyed) = balances(&projected.projection, true, &theme);
        assert_eq!(
            said(&key_line(&keyed, None, money)),
            "██ HSA  ░░ pre-tax  ▒▒ Roth  ▓▓ taxable  "
        );
        let row = projected.projection.row(2040).unwrap();
        let read = said(&key_line(&keyed, Some(2040), money));
        let deferred = money(row.class_totals.deferred);
        assert!(read.contains(&format!("░░ pre-tax {deferred}  ")), "{read}");
        let compact = said(&key_line(&keyed, Some(2040), compact_money));
        assert!(compact.len() < read.len(), "{compact}");
        let width = u16::try_from(key_line(&keyed, Some(2040), money).width()).unwrap();
        assert_eq!(said(&fitted(&keyed, Some(2040), width)), read);
        assert_eq!(said(&fitted(&keyed, Some(2040), width - 1)), compact);
        assert_eq!(
            said(&key_line(&keyed, Some(1999), money)),
            said(&key_line(&keyed, None, money)),
            "a year the plan does not reach reads nothing"
        );
    }

    #[test]
    fn a_line_chart_is_keyed_by_its_lines() {
        let projected = test_projected();
        let theme = Theme::terminal();
        let (chart, keyed) = lines(income_taxes(&projected.projection, true, &theme), &theme);
        assert_eq!(chart.series.len(), 2);
        let row = projected.projection.row(2030).unwrap();
        assert_eq!(
            said(&key_line(&keyed, Some(2030), money)),
            format!(
                "── income {}  ── taxes {}  ",
                money(row.total_income),
                money(row.taxes.total)
            )
        );
    }

    #[test]
    fn the_markets_chart_keys_its_bands_and_reads_what_they_lie_between() {
        use retiretui_engine::market::{History, Progress, monte_carlo};
        use retiretui_engine::params::TaxTables;

        let projected = projected_from(&crate::support::test_plan_briefly_run());
        let (tables, history) = (TaxTables::embedded(), History::embedded());
        let found = monte_carlo(&projected.plan, &tables, history, &Progress::default());
        let runs = found.unwrap().runs;
        let (chart, keyed) = markets(&runs.bands, &Theme::terminal());
        assert_eq!((chart.series.len(), chart.shades.len()), (1, 2));
        assert_eq!(
            said(&key_line(&keyed, None, compact_money)),
            "░░ 10th to 90th  ▒▒ 25th to 75th  ── median  "
        );
        let last = runs.bands.last().unwrap();
        let read = said(&key_line(&keyed, Some(last.year), compact_money));
        let outer = format!(
            "░░ 10th to 90th {} to {}",
            compact_money(last.net_worth[0]),
            compact_money(last.net_worth[4])
        );
        assert!(read.starts_with(&outer), "{read}");
    }
}
