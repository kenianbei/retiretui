//! The Plans pane: a row per plan, led by the colour of its line on the
//! chart, its figures in the Overview's words - as many columns as the
//! pane is wide enough for, the most wanted first.

use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Commands, Component, Entity, Query, With};
use bevy_ecs::system::SystemParam;
use bevy_ui::{FlexDirection, Node, Val};
use plurimus::core::ratatui_core::layout::Constraint;
use plurimus::core::ratatui_core::style::{Color, Style};
use plurimus::ui::{ComputedWidgetArea, ScrollArea, UiStyle};
use plurimus::widgets::{ActiveDescendant, TableColumns};
use retiretui_engine::plan::Dollars;
use retiretui_engine::project::Summary;

use super::Plans;
use crate::commands::compare::Metric;
use crate::commands::tui::edit::table_bundle;
use crate::commands::tui::hints::Hints;
use crate::commands::tui::layout::{self, CURSOR_COLS, filling, fixed, placed};
use crate::commands::tui::nav::FocusStop;
use crate::commands::tui::pane::{Framed, Pane};
use crate::commands::tui::present::{
    self, ENDS_WITH, LIFETIME_TAXES, MONEY_LASTS, PEAKS_AT, compact_money, signed_money,
};
use crate::commands::tui::success::Success;
use crate::commands::tui::tabulate::{self, SWATCH_COLS};

const TITLE: &str = "Plans";
const PLAN: &str = "Plan";
/// Cells between columns past the one the table leaves.
const GAP: u16 = 1;
/// The most of the page the row takes, in percent: the chart keeps the
/// rest.
const MOST_OF_PAGE: f32 = 50.0;
/// Rows the pane spends besides one per plan: its borders and the header.
const FRAME_ROWS: usize = 3;
/// The fewest rows the row takes, borders included, so the Changes pane
/// beside the table has room however few plans there are.
const LEAST_ROWS: usize = 8;

/// A figure the table can show of each plan.
pub(super) struct Column {
    pub(super) header: &'static str,
    figure: Figure,
}

impl Column {
    /// The header; the charted metric's in the cursor year names both.
    fn heading(&self, metric: Metric, year: i16) -> String {
        match self.figure {
            Figure::InYear => format!("{} {year}", metric.title()),
            _ => self.header.to_owned(),
        }
    }
}

enum Figure {
    Money(fn(&Summary) -> Dollars),
    Lasts,
    Success,
    /// The charted metric in the cursor year.
    InYear,
    Peaks,
}

/// What a row's figures are read from.
struct Figured {
    summary: Summary,
    success: Success,
    in_year: String,
}

impl Figure {
    /// The figure of `own`, or its difference from `base`'s.
    fn cell(&self, own: &Figured, base: Option<&Figured>) -> String {
        let summary = &own.summary;
        match (self, base) {
            (Self::Money(of), None) => compact_money(of(summary)),
            (Self::Money(of), Some(base)) => signed_money(of(summary) - of(&base.summary)),
            (Self::Lasts, None) => present::money_lasts(summary),
            (Self::Lasts, Some(base)) => present::money_lasts_against(summary, &base.summary),
            (Self::Success, None) => own.success.text(),
            (Self::Success, Some(base)) => own.success.against(base.success),
            (Self::InYear, _) => own.in_year.clone(),
            (Self::Peaks, None) => present::peaks_at(summary),
            (Self::Peaks, Some(base)) => present::peaks_at_against(summary, &base.summary),
        }
    }
}

/// Every column after the plan's name, most wanted first: a narrow pane
/// drops them from the end.
pub(super) const COLUMNS: [Column; 10] = [
    Column {
        header: ENDS_WITH,
        figure: Figure::Money(|summary| summary.final_net_worth),
    },
    Column {
        header: MONEY_LASTS,
        figure: Figure::Lasts,
    },
    Column {
        header: "Success",
        figure: Figure::Success,
    },
    Column {
        header: "",
        figure: Figure::InYear,
    },
    Column {
        header: LIFETIME_TAXES,
        figure: Figure::Money(|summary| summary.lifetime_taxes),
    },
    Column {
        header: PEAKS_AT,
        figure: Figure::Peaks,
    },
    Column {
        header: "Medicare",
        figure: Figure::Money(|summary| summary.lifetime_medicare),
    },
    Column {
        header: "Lifetime conversions",
        figure: Figure::Money(|summary| summary.lifetime_conversions),
    },
    Column {
        header: "Unfunded",
        figure: Figure::Money(|summary| summary.lifetime_unfunded),
    },
    Column {
        header: "Pre-tax at end",
        figure: Figure::Money(|summary| summary.final_deferred),
    },
];

#[derive(Component)]
pub(crate) struct PlansTable;

/// A plan's row, by its place: the document is 0, then the compared files.
#[derive(Component, Clone, Copy)]
pub(crate) struct PlanRow(pub(super) usize);

/// The Plans pane, the Changes pane beside it, in a row of their own
/// under `view`.
pub(super) fn spawn(commands: &mut Commands, view: Entity) {
    let row = Node {
        flex_direction: FlexDirection::Row,
        max_height: Val::Percent(MOST_OF_PAGE),
        ..fixed(rows_tall(0))
    };
    let row = commands.spawn((row, ChildOf(view))).id();
    let pane = Pane::new(TITLE).sharing(2.0).spawn(commands, row);
    super::changes::spawn(commands, row);
    commands.spawn((
        table_bundle(),
        PlansTable,
        Hints(&[("↑↓", "plan"), ("⏎", "open")]),
        layout::Rests,
        FocusStop,
        filling(),
        placed(),
        ChildOf(pane),
    ));
}

/// The row's height with `compared` files beside the document.
fn rows_tall(compared: usize) -> f32 {
    (compared + 1 + FRAME_ROWS).max(LEAST_ROWS) as f32
}

/// Where the table's cursor rests.
#[derive(SystemParam)]
pub(crate) struct Cursor<'w, 's> {
    tables: Query<'w, 's, &'static ActiveDescendant, With<PlansTable>>,
    rows: Query<'w, 's, &'static PlanRow>,
}

impl Cursor<'_, '_> {
    /// The highlighted plan's place: the document is 0.
    pub fn place(&self) -> usize {
        let row = self.tables.iter().find_map(|cursor| cursor.0);
        row.and_then(|row| self.rows.get(row).ok())
            .map_or(0, |row| row.0)
    }
}

/// The table, and the pane and row it stands in.
#[derive(SystemParam)]
pub(super) struct PlansParts<'w, 's> {
    tables: Query<
        'w,
        's,
        (
            Entity,
            Ref<'static, ComputedWidgetArea>,
            &'static mut ScrollArea,
            &'static ChildOf,
        ),
        With<PlansTable>,
    >,
    frames: Query<'w, 's, (&'static mut Framed, &'static ChildOf)>,
    nodes: Query<'w, 's, &'static mut Node>,
}

/// Respawns the rows whenever a plan, the basis, the baseline, a success
/// or the width moves, keeping the cursor on the same place; the
/// baseline's row is dimmed.
pub(super) fn refresh_plans(
    plans: Plans,
    cursor: Cursor,
    mut parts: PlansParts,
    mut commands: Commands,
) {
    let kept = cursor.place();
    let is_moved = plans.is_changed() || plans.successes.is_changed();
    for (table, area, mut scroll, pane) in &mut parts.tables {
        if !is_moved && !area.is_changed() {
            continue;
        }
        let (header, rows) = laid(&plans, scroll.content_width(area.0.width));
        scroll.content_size.height = u16::try_from(rows.len() + 1).unwrap_or(u16::MAX);
        commands.entity(table).despawn_related::<Children>();
        let keys = keys(&plans);
        let spawned = tabulate::fill_keyed(&mut commands, table, (&header, &rows), GAP, &keys);
        for (place, &row) in spawned.iter().enumerate() {
            commands.entity(row).insert(PlanRow(place));
        }
        if let Some(&baseline) = spawned.get(plans.compared.baseline_at()) {
            commands
                .entity(baseline)
                .insert(UiStyle(plans.theme.dimmed()));
        }
        let on = spawned.get(kept).or(spawned.last()).copied();
        commands.entity(table).insert(ActiveDescendant(on));
        if let Ok((mut framed, row)) = parts.frames.get_mut(pane.parent()) {
            Framed::retitle(&mut framed, &title(&plans));
            Framed::renote(&mut framed, &plans.compared.failure().unwrap_or_default());
            let tall = Val::Px(rows_tall(plans.compared.docs.len()));
            if let Ok(mut node) = parts.nodes.get_mut(row.parent())
                && node.height != tall
            {
                node.height = tall;
            }
        }
    }
}

/// Each row's swatch colour and name style: a file that failed to re-read
/// is named in the colour of what went wrong.
fn keys(plans: &Plans) -> Vec<(Color, Style)> {
    let failed = plans.compared.docs.iter().map(|doc| doc.failure.is_some());
    std::iter::once(false)
        .chain(failed)
        .enumerate()
        .map(|(at, is_failed)| {
            let named = if is_failed {
                plans.theme.exceeded()
            } else {
                Style::new()
            };
            (plans.theme.series(at), named)
        })
        .collect()
}

/// "Plans", and what they are measured against and in.
fn title(plans: &Plans) -> String {
    let basis = present::basis_name(plans.shown.basis.nominal);
    match plans.against() {
        Some((_, name, _)) => format!("{TITLE} · against {name} · {basis}"),
        None => format!("{TITLE} · {basis}"),
    }
}

/// The header and a row per plan, cut to the columns that fit in `given`
/// cells.
fn laid(plans: &Plans, given: u16) -> (Vec<String>, Vec<Vec<String>>) {
    let (deflated, year) = (!plans.shown.basis.nominal, plans.year());
    let metric = plans.charted.metric;
    let header: Vec<String> = std::iter::once(PLAN.to_owned())
        .chain(COLUMNS.iter().map(|column| column.heading(metric, year)))
        .collect();
    let in_year = plans.figures_in(&plans.amounts(metric), year);
    let figured: Vec<Figured> = (plans.each().zip(plans.each_plan()).zip(in_year))
        .map(|(((_, projection), plan), in_year)| Figured {
            summary: projection.summary(deflated),
            success: plans.successes.of(plan),
            in_year,
        })
        .collect();
    let against = plans.against().map(|(at, ..)| at);
    let mut rows: Vec<Vec<String>> = (plans.each().enumerate())
        .map(|(place, (name, _))| {
            let own = &figured[place];
            let base = against.filter(|&at| at != place).map(|at| &figured[at]);
            let cells = COLUMNS.iter().map(|column| column.figure.cell(own, base));
            std::iter::once(name.into_owned()).chain(cells).collect()
        })
        .collect();
    let count = fitting(&tabulate::gapped_columns((&header, &rows), GAP), given);
    for row in &mut rows {
        row.truncate(count);
    }
    (header[..count].to_vec(), rows)
}

/// How many leading columns fit in `given` cells beside the cursor and
/// the swatch; the plan's name always does.
pub(super) fn fitting(measured: &TableColumns, given: u16) -> usize {
    let mut used = CURSOR_COLS + SWATCH_COLS;
    let mut count = 0;
    for constraint in &measured.0 {
        let Constraint::Length(width) = *constraint else {
            break;
        };
        let spacing = u16::from(count > 0);
        let needed = used.saturating_add(width).saturating_add(spacing);
        if count > 0 && needed > given {
            break;
        }
        used = needed;
        count += 1;
    }
    count
}
