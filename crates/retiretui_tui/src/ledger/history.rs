//! The histories under the money panes: each pane's two figures as lines
//! across every year, the cursor year ruled, on a page tall enough to hold
//! them. A press on a year moves the cursor there, as on every chart.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, IntoScheduleConfigs, Query, Res, With};
use bevy_ui::{FlexDirection, Node, Val};
use plurimus::core::UiWidget;
use plurimus::core::ratatui_core::style::Style;
use plurimus::core::ratatui_core::text::{Line, Span};
use plurimus::widgets::ratatui_widgets::paragraph::Paragraph;
use retiretui_client::ledger::History;

use super::LedgerSystems;
use super::arrange::Roomy;
use crate::chart::{Legend, Mark, Series, SeriesChart};
use crate::layout::{filling, fixed, placed, set_display};
use crate::pane::Pane;
use crate::session::Shown;
use crate::theme::Theme;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, (show, refresh).in_set(LedgerSystems::Draw));
}

/// The rows the histories keep, borders included, where they are drawn.
const HISTORY_LEAST: f32 = 8.0;
const SWATCH: &str = "━ ";
const KEY_GAP: &str = "  ";

/// The row the histories stand in.
#[derive(Component)]
struct HistoryRow;

/// The line under a history naming its two lines.
#[derive(Component)]
struct HistoryKey(History);

pub(super) fn spawn_row(commands: &mut Commands, detail: Entity) {
    let row = Node {
        flex_direction: FlexDirection::Row,
        flex_grow: 1.0,
        flex_basis: Val::Px(0.0),
        min_height: Val::Px(HISTORY_LEAST),
        ..Node::default()
    };
    let row = commands.spawn((row, HistoryRow, ChildOf(detail))).id();
    for history in History::ALL {
        let pane = Pane::new(history.title()).sharing(1.0).spawn(commands, row);
        commands.spawn((
            Charted(history),
            SeriesChart::default(),
            filling(),
            ChildOf(pane),
        ));
        commands.spawn((
            HistoryKey(history),
            UiWidget::default(),
            fixed(1.0),
            placed(),
            ChildOf(pane),
        ));
    }
}

/// Which history a chart draws.
#[derive(Component, Clone, Copy)]
struct Charted(History);

/// The histories take room only on a page that has it.
fn show(roomy: Res<Roomy>, mut rows: Query<&mut Node, With<HistoryRow>>) {
    if !roomy.is_changed() {
        return;
    }
    for mut node in &mut rows {
        set_display(&mut node, roomy.0);
    }
}

/// The two lines of `history` over what the Ledger shows, in the theme's
/// first two series colours, the cursor year ruled.
fn charted(history: History, shown: &Shown, theme: &Theme) -> SeriesChart {
    let points = history.points(&shown.ledger().projection, shown.basis.nominal);
    let series = history.lines().into_iter().enumerate().map(|(at, label)| {
        let line = points.iter();
        Series {
            label: label.to_owned(),
            color: theme.series(at),
            points: line
                .map(|&(year, figures)| (f64::from(year), figures[at] as f64))
                .collect(),
        }
    });
    let mut chart = SeriesChart::of(series.collect(), theme.dimmed());
    chart.legend = Legend::Hidden;
    chart.marks = vec![Mark::cursor(shown.year(), theme)];
    chart
}

/// A history's two lines named, each after a stroke in its colour.
fn key(history: History, theme: &Theme) -> Line<'static> {
    let named = history.lines().into_iter().enumerate();
    let spans = named.flat_map(|(at, label)| {
        let gap = if at == 0 { " " } else { KEY_GAP };
        [
            Span::raw(gap),
            Span::styled(SWATCH, Style::new().fg(theme.series(at))),
            Span::styled(label, theme.dimmed()),
        ]
    });
    Line::from(spans.collect::<Vec<_>>())
}

/// Redraws each history whenever what the Ledger shows, the basis, the
/// cursor year or the theme moves.
fn refresh(
    (shown, theme): (Shown, Res<Theme>),
    mut charts: Query<(&Charted, &mut SeriesChart)>,
    mut keys: Query<(&HistoryKey, &mut UiWidget)>,
) {
    let is_first = charts.iter().any(|(_, chart)| chart.series.is_empty());
    if !shown.is_changed() && !theme.is_changed() && !is_first {
        return;
    }
    for (&Charted(history), mut chart) in &mut charts {
        *chart = charted(history, &shown, &theme);
    }
    if !theme.is_changed() && !is_first {
        return;
    }
    for (&HistoryKey(history), mut widget) in &mut keys {
        *widget = UiWidget::new(Paragraph::new(key(history, &theme)));
    }
}
