//! The Overview's verdict in the window: over the strip the terminal
//! tiles, a band saying whether the money lasts, and four scoreboards -
//! what the plan ends with, the least it holds after the last salary,
//! what it spends over the plan and what it pays in tax - each a figure
//! twice the size over a line of context. The terminal's tiles are hidden
//! beneath, but the Success tile, whose ⏎ opens the Monte Carlo page,
//! which the band takes in so it lights as that tile is reached, its own
//! text hidden with the rest.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Commands, Component, Entity, IntoScheduleConfigs, Query, Res, With};
use bevy_ecs::system::SystemParam;
use bevy_text::{FontSize, LineHeight};
use bevy_ui::{BackgroundColor, BorderRadius, Display, FlexDirection, Node, Val};
use plurimus::core::ratatui_core::style::{Modifier, Style};
use plurimus::core::ratatui_core::text::{Line, Span};
use retiretui_client::overview::{LOW_POINT, SPENDING_TOTAL, TAXES_TOTAL, View};
use retiretui_client::present::{ENDS_WITH, basis_name};

use super::card::Card;
use super::{CELL, FONT_SIZE, Figure};
use crate::grid::HEADING;
use crate::layout::{fixed, growing};
use crate::overview::{SUCCESS_AT, TileAt, Viewed, success_text};
use crate::pane::{Framed, JOIN, Pane};
use crate::session::Shown;
use crate::success::Successes;
use crate::theme::{Hung, Repainted, Theme, hang, said, say, text};

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (raise_the_verdict, say_the_verdict)
            .chain()
            .in_set(Repainted),
    );
}

/// Rows the strip takes in the window: the band's and the scoreboards'.
const BAND_ROWS: f32 = 4.0;
const BOARD_ROWS: f32 = 6.0;
const STRIP_ROWS: f32 = BAND_ROWS + BOARD_ROWS;
/// A figure is this many times the size, and its line as many rows.
const FIGURE_SCALE: f32 = 2.0;

/// The money's verdict: its mark on a disc, and the words beside it.
const LASTS: &str = "The money lasts";
const RUNS_SHORT: &str = "Runs short in";
const LASTS_MARK: &str = " ✓ ";
const SHORT_MARK: &str = " ✗ ";
/// The disc is the mark's three cells rounded whole.
const DISC_RADIUS: Val = Val::Em(1.5);
/// The scoreboards' titles, in their order.
const BOARDS: [&str; 4] = [ENDS_WITH, LOW_POINT, "Spends over the plan", "Pays in tax"];

/// The band's pane, which the Success tile was moved into: the disc and
/// the words of its verdict, its context line, and each scoreboard's
/// figure and context in the boards' order.
#[derive(Component, Debug)]
struct Band {
    disc: Entity,
    title: Entity,
    context: Entity,
    boards: [(Entity, Entity); 4],
}

/// Raises the window's verdict over a strip not yet dressed: the strip
/// grown to hold it, each terminal tile's pane hidden, and the Success
/// tile moved into the band so its focus and ⏎ are the band's.
fn raise_the_verdict(
    tiles: Query<(Entity, &TileAt, &ChildOf, &Hung)>,
    panes: Query<&ChildOf, With<Framed>>,
    bands: Query<(), With<Band>>,
    mut nodes: Query<&mut Node>,
    mut commands: Commands,
) {
    if !bands.is_empty() {
        return;
    }
    let Some((tile, _, pane, said)) = tiles.iter().find(|(_, at, ..)| at.0 == SUCCESS_AT) else {
        return;
    };
    let Ok(strip) = panes.get(pane.parent()).map(ChildOf::parent) else {
        return;
    };
    for (_, _, pane, _) in &tiles {
        if let Ok(mut node) = nodes.get_mut(pane.parent()) {
            node.display = Display::None;
        }
    }
    if let Ok(mut node) = nodes.get_mut(said.0) {
        node.display = Display::None;
    }
    if let Ok(mut node) = nodes.get_mut(strip) {
        node.height = Val::Rem(STRIP_ROWS);
        node.flex_direction = FlexDirection::Column;
    }
    let band = Pane::new("")
        .tall(BAND_ROWS)
        .spawn_with(&mut commands, strip, Card);
    commands.entity(tile).insert((growing(), ChildOf(band)));
    let (disc, words) = spawn_title_row(&mut commands, band);
    let holder = commands.spawn((fixed(1.0), ChildOf(band))).id();
    let context = hang(&mut commands, holder);
    let boards = spawn_boards(&mut commands, strip);
    commands.entity(band).insert(Band {
        disc,
        title: words,
        context,
        boards,
    });
}

/// The band's title row: a disc for the verdict's mark, then its words.
fn spawn_title_row(commands: &mut Commands, band: Entity) -> (Entity, Entity) {
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Em(1.0),
                ..fixed(1.0)
            },
            ChildOf(band),
        ))
        .id();
    let disc = commands
        .spawn((
            text(),
            Node {
                border_radius: BorderRadius::all(DISC_RADIUS),
                ..Node::default()
            },
            ChildOf(row),
        ))
        .id();
    let title = commands.spawn((text(), ChildOf(row))).id();
    (disc, title)
}

/// Each scoreboard's figure and context under its title.
fn spawn_boards(commands: &mut Commands, strip: Entity) -> [(Entity, Entity); 4] {
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                ..fixed(BOARD_ROWS)
            },
            ChildOf(strip),
        ))
        .id();
    BOARDS.map(|title| {
        let pane = Pane::new(title)
            .sharing(1.0)
            .spawn_with(commands, row, Card);
        let figure = commands
            .spawn((
                Figure,
                text(),
                LineHeight::Px(CELL.y * FIGURE_SCALE),
                fixed(FIGURE_SCALE),
                ChildOf(pane),
            ))
            .id();
        let context = commands.spawn((fixed(1.0), ChildOf(pane))).id();
        (figure, hang(commands, context))
    })
}

/// Says a figure: one bold span at the figure's size.
fn say_figure(commands: &mut Commands, figure: Entity, value: &str, theme: &Theme) {
    let (span, color, mut font) = said(Span::styled(value.to_owned(), HEADING), theme);
    font.font_size = FontSize::Px(FONT_SIZE * FIGURE_SCALE);
    let mut root = commands.entity(figure);
    root.despawn_related::<Children>();
    root.with_child((Figure, span, color, font));
}

/// The band's context line: how long the money lasts, how surely across
/// the markets, and what the plan rests on first.
fn context_said(view: &View, success: &str, theme: &Theme) -> Line<'static> {
    let mut parts = vec![view.money_lasts.clone(), success.to_owned()];
    if let Some(assumption) = view.rests_on.first() {
        parts.push(format!("{} {}", assumption.label, assumption.value));
    }
    Line::styled(parts.join(JOIN), theme.dimmed())
}

/// Says the verdict: the mark in the ground's ink on a disc of the
/// verdict's colour, and its words bold beside it.
fn say_verdict(commands: &mut Commands, band: &Band, view: &View, theme: &Theme) {
    let (mark, disc, words) = match &view.shortfall {
        Some(short) => (
            SHORT_MARK,
            theme.exceeded(),
            format!("{RUNS_SHORT} {}", short.year),
        ),
        None => (LASTS_MARK, Style::new().fg(theme.good), LASTS.to_owned()),
    };
    let on_disc = Style::new().fg(theme.bg).add_modifier(Modifier::BOLD);
    commands
        .entity(band.disc)
        .insert(BackgroundColor(theme.ink(disc)));
    say(commands, band.disc, Line::styled(mark, on_disc), theme);
    say(commands, band.title, Line::styled(words, HEADING), theme);
}

/// Each scoreboard's figure and context in the boards' order.
fn boards_said(view: &View, is_nominal: bool) -> [(&str, &str); 4] {
    let total = |label: &str| {
        view.totals
            .iter()
            .find(|total| total.label == label)
            .map_or(("", ""), |total| (&*total.amount, &*total.made_of))
    };
    [
        (&view.ends_with, &view.money_lasts),
        (&view.low_point, basis_name(is_nominal)),
        total(SPENDING_TOTAL),
        total(TAXES_TOTAL),
    ]
}

#[derive(SystemParam)]
struct Read<'w> {
    viewed: Res<'w, Viewed>,
    shown: Shown<'w>,
    successes: Res<'w, Successes>,
    theme: Res<'w, Theme>,
}

/// Says the band and the boards again on what the terminal's tiles are
/// said again on: the page and the theme for every one, and the markets'
/// answer for the band's context alone.
fn say_the_verdict(read: Read, bands: Query<(&Band, Ref<Framed>)>, mut commands: Commands) {
    let Read {
        viewed,
        shown,
        successes,
        theme,
    } = read;
    let Some(view) = viewed.view() else {
        return;
    };
    let Ok((band, framed)) = bands.single() else {
        return;
    };
    let is_moved = viewed.is_changed() || theme.is_changed() || framed.is_added();
    if !is_moved && !successes.is_changed() {
        return;
    }
    let success = successes.of(&shown.projected.plan);
    let said = success_text(success, &shown.projected);
    let context = context_said(view, &said, &theme);
    say(&mut commands, band.context, context, &theme);
    if !is_moved {
        return;
    }
    say_verdict(&mut commands, band, view, &theme);
    let boards = boards_said(view, shown.basis.nominal);
    for (&(figure, context), (value, said)) in band.boards.iter().zip(boards) {
        say_figure(&mut commands, figure, value, &theme);
        let context_line = Line::styled(said.to_owned(), theme.dimmed());
        say(&mut commands, context, context_line, &theme);
    }
}

#[cfg(test)]
mod tests {
    use bevy_text::TextFont;

    use super::*;
    use crate::support::{Headless, SIZE, headless_app, said_by};

    fn raised() -> Headless {
        let mut app = headless_app(SIZE);
        app.add_plugins(plugin);
        app.update();
        app.update();
        app
    }

    #[test]
    fn the_band_takes_the_success_tile_in_and_the_other_tiles_are_hidden() {
        let mut app = raised();
        let world = app.world_mut();
        let view = world.resource::<Viewed>().view().unwrap().clone();
        let (band_pane, band) = world
            .query::<(Entity, &Band)>()
            .single(world)
            .map(|(pane, band)| (pane, (band.disc, band.title, band.context)))
            .unwrap();
        let (disc, title, band) = band;
        let theme = Theme::default();
        assert_eq!(said_by(world, title), LASTS);
        assert_eq!(said_by(world, disc), LASTS_MARK);
        assert_eq!(
            world.get::<BackgroundColor>(disc).unwrap().0,
            theme.ink(Style::new().fg(theme.good))
        );
        let context = said_by(world, band);
        assert!(
            context.starts_with(&format!("{}{JOIN}", view.money_lasts)),
            "{context}"
        );
        let tiles: Vec<(usize, Entity, Display)> = world
            .query::<(&TileAt, &ChildOf)>()
            .iter(world)
            .map(|(at, pane)| {
                let display = world.get::<Node>(pane.parent()).unwrap().display;
                (at.0, pane.parent(), display)
            })
            .collect();
        for (at, pane, display) in tiles {
            if at == SUCCESS_AT {
                assert_eq!(pane, band_pane, "the Success tile is the band's");
            } else {
                assert_eq!(display, Display::None, "tile {at} is hidden");
            }
        }
        let tile_said = world
            .query::<(&TileAt, &Hung)>()
            .iter(world)
            .find(|(at, _)| at.0 == SUCCESS_AT)
            .map(|(_, said)| said.0)
            .unwrap();
        assert_ne!(tile_said, band, "the terminal's tile says its own line");
        assert_eq!(world.get::<Node>(tile_said).unwrap().display, Display::None);
    }

    #[test]
    fn each_board_says_its_figure_at_twice_the_size_over_its_context() {
        let mut app = raised();
        let world = app.world_mut();
        let view = world.resource::<Viewed>().view().unwrap().clone();
        let boards = world.query::<&Band>().single(world).unwrap().boards;
        let said: Vec<(String, String)> = boards
            .iter()
            .map(|&(figure, context)| (said_by(world, figure), said_by(world, context)))
            .collect();
        let expected: Vec<(String, String)> = boards_said(&view, false)
            .into_iter()
            .map(|(figure, context)| (figure.to_owned(), context.to_owned()))
            .collect();
        assert_eq!(said, expected);
        let span = world.get::<Children>(boards[0].0).unwrap()[0];
        let font = world.get::<TextFont>(span).unwrap();
        assert_eq!(font.font_size, FontSize::Px(FONT_SIZE * FIGURE_SCALE));
        assert!(world.entity(span).contains::<Figure>(), "the pins leave it");
    }
}
