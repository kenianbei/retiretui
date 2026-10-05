//! The verdict strip: whether the money lasts, how often it would across
//! random markets in the colour of its zone, the least the household holds
//! once it stops earning, and what it ends with, each in a pane titled
//! for what it says. ⏎ on the Success tile opens the Monte Carlo page.

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, Query, Res};
use bevy_ui::{FlexDirection, Node};
use plurimus::core::UiWidget;
use plurimus::core::ratatui_core::style::{Modifier, Style};
use plurimus::core::ratatui_core::text::Line;
use plurimus::widgets::ratatui_widgets::paragraph::Paragraph;
use retiretui_client::overview::{STRIP, View};
use retiretui_client::searches::markets::{self, Markets};
use retiretui_engine::market::MonteCarlo;

use crate::hints::Hints;
use crate::layout::{fixed, growing, placed};
use crate::nav::{FocusStop, Page};
use crate::pane::{BORDERS, Pane};
use crate::session::Projected;
use crate::success::{Success, Successes};
use crate::theme::Theme;
use crate::tools::markets::zone_style;
use crate::tools::{EnterRuns, handle_enter};

use super::Viewed;

/// A tile's value between its pane's borders.
const STRIP_ROWS: f32 = BORDERS as f32 + 1.0;
const TILE_COUNT: usize = STRIP.len();
/// The Success tile's place in the strip.
const SUCCESS_AT: usize = 1;

/// A tile's value, by its place in the strip.
#[derive(Component, Clone, Copy)]
pub(super) struct TileAt(usize);

pub(super) fn spawn(commands: &mut Commands, view: Entity) {
    let strip = Node {
        flex_direction: FlexDirection::Row,
        ..fixed(STRIP_ROWS)
    };
    let strip = commands.spawn((strip, ChildOf(view))).id();
    for (place, label) in STRIP.into_iter().enumerate() {
        let framed = Pane::new(label).sharing(1.0).spawn(commands, strip);
        let mut tile = commands.spawn((
            TileAt(place),
            growing(),
            UiWidget::default(),
            placed(),
            ChildOf(framed),
        ));
        if place == SUCCESS_AT {
            let page = Page::MonteCarlo.label();
            tile.insert((FocusStop, EnterRuns(page), Hints(&[("⏎", "markets")])))
                .observe(handle_enter);
        }
    }
}

struct Tile {
    value: String,
    /// What the value is drawn in, past bold.
    style: Style,
}

impl Tile {
    fn paragraph(&self) -> Paragraph<'static> {
        Paragraph::new(Line::styled(
            format!(" {}", self.value),
            self.style.add_modifier(Modifier::BOLD),
        ))
    }
}

/// The share of markets survived, and of how many.
fn success_text(success: Success, projected: &Projected) -> String {
    match success {
        Success::Rate(rate) => {
            let trials = usize::try_from(projected.plan.market().trials()).unwrap_or_default();
            markets::verdict_of(rate, trials, <MonteCarlo as Markets>::RUN_NOUN)
        }
        _ => success.text(),
    }
}

/// The strip's tiles in the client's order: the money's last year in the
/// tone of a warning where it runs short, and the success in its zone's.
fn tiles(view: &View, success: (Success, String), theme: &Theme) -> [Tile; TILE_COUNT] {
    let (success, said) = success;
    let lasts = if view.shortfall.is_some() {
        theme.exceeded()
    } else {
        Style::new()
    };
    let surely = match success {
        Success::Rate(rate) => zone_style(rate, theme),
        _ => Style::new(),
    };
    [
        (view.money_lasts.clone(), lasts),
        (said, surely),
        (view.low_point.clone(), Style::new()),
        (view.ends_with.clone(), Style::new()),
    ]
    .map(|(value, style)| Tile { value, style })
}

/// Rewrites each tile whenever what it reads moves: the page as the
/// client says it and the theme for every one, and the Success tile also
/// its answer.
pub(super) fn refresh(
    (viewed, projected, theme): (Res<Viewed>, Res<Projected>, Res<Theme>),
    successes: Res<Successes>,
    mut parts: Query<(&TileAt, &mut UiWidget)>,
) {
    let is_moved = viewed.is_changed() || theme.is_changed();
    let is_success_moved = is_moved || successes.is_changed();
    let Some(view) = &viewed.0 else {
        return;
    };
    if !is_success_moved {
        return;
    }
    let success = successes.of(&projected.plan);
    let tiles = tiles(view, (success, success_text(success, &projected)), &theme);
    for (TileAt(place), mut widget) in &mut parts {
        let is_stale = if *place == SUCCESS_AT {
            is_success_moved
        } else {
            is_moved
        };
        if is_stale {
            *widget = UiWidget::new(tiles[*place].paragraph());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{TEST_PLAN, projected_from, test_projected};

    fn tiles_of(projected: &Projected, nominal: bool, success: Success) -> [Tile; TILE_COUNT] {
        let view = View::new(projected, nominal);
        let said = success_text(success, projected);
        tiles(&view, (success, said), &Theme::terminal())
    }

    #[test]
    fn the_tiles_follow_the_basis_and_the_success_answered() {
        let projected = test_projected();
        let values = |nominal, success| tiles_of(&projected, nominal, success).map(|it| it.value);
        let todays = values(false, Success::Rate(0.997));
        let nominal = values(true, Success::Rate(0.997));
        assert_eq!(todays[1], "99.7% of 1,000 markets");
        assert_ne!(todays[2], nominal[2], "the low point follows the basis");
        assert_ne!(todays[3], nominal[3], "what it ends with follows the basis");
        assert!(todays[3].starts_with('$'), "{}", todays[3]);
        let running = Success::Running {
            done: 340,
            total: 1000,
        };
        assert_eq!(values(false, running)[1], "running 340 of 1,000");
    }

    #[test]
    fn the_money_lasts_tile_warns_only_of_a_plan_that_runs_short() {
        let theme = Theme::terminal();
        let lasting = &tiles_of(&test_projected(), false, Success::Waiting)[0];
        assert_eq!(
            (lasting.value.as_str(), lasting.style),
            ("Never short", Style::new())
        );
        let short = projected_from(&TEST_PLAN.replace("amount = 60000", "amount = 95000"));
        let year = short.projection.summary(true).first_unfunded_year.unwrap();
        let tile = &tiles_of(&short, false, Success::Waiting)[0];
        assert_eq!(tile.value, format!("Through {}", year - 1));
        assert_eq!(tile.style, theme.exceeded());
    }

    #[test]
    fn the_success_is_drawn_in_its_zone() {
        let theme = Theme::terminal();
        let projected = test_projected();
        let styled = |success| tiles_of(&projected, false, success)[SUCCESS_AT].style;
        assert_eq!(styled(Success::Rate(0.95)), Style::new().fg(theme.good));
        assert_eq!(styled(Success::Rate(0.2)), theme.exceeded());
        assert_eq!(styled(Success::Waiting), Style::new());
    }
}
