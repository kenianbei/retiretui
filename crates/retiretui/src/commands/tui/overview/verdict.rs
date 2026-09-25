//! The verdict strip: whether the money lasts, how often it would across
//! random markets, what it ends with, and what it pays in tax. ⏎ on the
//! Success tile opens the Monte Carlo page.

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, Query, Res};
use bevy_input_focus::InputFocus;
use bevy_ui::{FlexDirection, Node, Val};
use plurimus::core::UiWidget;
use plurimus::core::ratatui_core::style::{Modifier, Style};
use plurimus::core::ratatui_core::text::Line;
use plurimus::widgets::ratatui_widgets::paragraph::Paragraph;

use crate::commands::tui::hints::Hints;
use crate::commands::tui::layout::{fixed, placed};
use crate::commands::tui::nav::{FocusStop, Page};
use crate::commands::tui::present::{self, ENDS_WITH, LIFETIME_TAXES, MONEY_LASTS, compact_money};
use crate::commands::tui::session::{Basis, Projected};
use crate::commands::tui::success::{Success, Successes};
use crate::commands::tui::theme::Theme;
use crate::commands::tui::tools::{EnterRuns, count_text, handle_enter};

/// A tile's label over its value.
const STRIP_ROWS: f32 = 2.0;
const TILE_COUNT: usize = 4;
const SUCCESS: &str = "Success";
/// The Success tile's place in the strip.
const SUCCESS_AT: usize = 1;

/// A tile of the strip, by its place.
#[derive(Component, Clone, Copy)]
pub(super) struct TileAt(usize);

pub(super) fn spawn(commands: &mut Commands, view: Entity) {
    let strip = Node {
        flex_direction: FlexDirection::Row,
        ..fixed(STRIP_ROWS)
    };
    let strip = commands.spawn((strip, ChildOf(view))).id();
    for place in 0..TILE_COUNT {
        let node = Node {
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            ..Node::default()
        };
        let mut tile = commands.spawn((
            TileAt(place),
            node,
            UiWidget::default(),
            placed(),
            ChildOf(strip),
        ));
        if place == SUCCESS_AT {
            let page = Page::MonteCarlo.label();
            tile.insert((FocusStop, EnterRuns(page), Hints(&[("⏎", "markets")])))
                .observe(handle_enter);
        }
    }
}

struct Tile {
    label: &'static str,
    value: String,
    is_warning: bool,
}

impl Tile {
    fn plain(label: &'static str, value: String) -> Self {
        Self {
            label,
            value,
            is_warning: false,
        }
    }

    /// The label is lit while the tile holds the keyboard.
    fn paragraph(&self, theme: &Theme, is_focused: bool) -> Paragraph<'static> {
        let value = if self.is_warning {
            theme.exceeded()
        } else {
            Style::new()
        };
        let label = if is_focused {
            theme.accented()
        } else {
            theme.dimmed()
        };
        Paragraph::new(vec![
            Line::styled(format!(" {}", self.label), label),
            Line::styled(
                format!(" {}", self.value),
                value.add_modifier(Modifier::BOLD),
            ),
        ])
    }
}

/// The share of markets survived, and of how many.
fn success_text(success: Success, projected: &Projected) -> String {
    match success {
        Success::Rate(_) => {
            let trials = usize::try_from(projected.plan.market().trials()).unwrap_or_default();
            format!("{} of {}", success.text(), count_text(trials))
        }
        _ => success.text(),
    }
}

fn tiles(projected: &Projected, nominal: bool, success: Success) -> [Tile; TILE_COUNT] {
    let summary = projected.projection.summary(!nominal);
    let lasts = Tile {
        label: MONEY_LASTS,
        value: present::money_lasts(&summary),
        is_warning: summary.first_unfunded_year.is_some(),
    };
    [
        lasts,
        Tile::plain(SUCCESS, success_text(success, projected)),
        Tile::plain(ENDS_WITH, compact_money(summary.final_net_worth)),
        Tile::plain(LIFETIME_TAXES, compact_money(summary.lifetime_taxes)),
    ]
}

/// Rewrites each tile whenever what it reads moves: the plan, the basis
/// and the theme for every one, and the Success tile also its answer and
/// whether it holds the keyboard.
pub(super) fn refresh(
    (projected, basis, theme): (Res<Projected>, Res<Basis>, Res<Theme>),
    (successes, focus): (Res<Successes>, Res<InputFocus>),
    mut parts: Query<(Entity, &TileAt, &mut UiWidget)>,
) {
    let is_moved = projected.is_changed() || basis.is_changed() || theme.is_changed();
    let is_success_moved = is_moved || successes.is_changed() || focus.is_changed();
    if !is_success_moved {
        return;
    }
    let success = successes.of(&projected.plan);
    let tiles = tiles(&projected, basis.nominal, success);
    for (tile, TileAt(place), mut widget) in &mut parts {
        let is_stale = if *place == SUCCESS_AT {
            is_success_moved
        } else {
            is_moved
        };
        if is_stale {
            let is_focused = focus.get() == Some(tile);
            *widget = UiWidget::new(tiles[*place].paragraph(&theme, is_focused));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tui::support::test_projected;
    use retiretui_engine::params::TaxTables;
    use retiretui_engine::project::project;

    fn values(projected: &Projected, nominal: bool, success: Success) -> Vec<String> {
        let tiles = tiles(projected, nominal, success);
        tiles.into_iter().map(|tile| tile.value).collect()
    }

    #[test]
    fn the_tiles_follow_the_basis_and_the_success_answered() {
        let projected = test_projected();
        let todays = values(&projected, false, Success::Rate(0.997));
        let nominal = values(&projected, true, Success::Rate(0.997));
        assert_eq!(todays[1], "99.7% of 1,000");
        assert_ne!(todays[2], nominal[2], "basis must change the figures");
        assert!(todays[2].starts_with('$'), "{}", todays[2]);
        let running = Success::Running {
            done: 340,
            total: 1000,
        };
        assert_eq!(
            values(&projected, false, running)[1],
            "running 340 of 1,000"
        );
    }

    #[test]
    fn the_shortfall_tile_warns_only_of_a_plan_that_runs_short() {
        let mut projected = test_projected();
        let lasting = &tiles(&projected, false, Success::Waiting)[0];
        assert_eq!(lasting.value, "Never short");
        assert!(!lasting.is_warning);
        projected.plan.expenses[0].amount = 400_000;
        projected.projection = project(&projected.plan, &TaxTables::embedded());
        let summary = projected.projection.summary(true);
        let year = summary.first_unfunded_year.expect("runs short");
        let short = &tiles(&projected, false, Success::Waiting)[0];
        assert_eq!(
            short.value,
            format!(
                "Short {} from {year}",
                compact_money(summary.lifetime_unfunded)
            )
        );
        assert!(short.is_warning);
    }
}
