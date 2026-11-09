//! The window's chrome over the shared markers: the page's margin, a
//! frame's hairline with its title in a header row - a box over the page a
//! sheet with a shadow, a card raised without one - a rule's line and its
//! title.

use bevy_app::{App, Update};
use bevy_color::{Alpha, Color};
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Add, Commands, Component, Entity, Has, IntoScheduleConfigs, On, Query, Res,
};
use bevy_ui::{
    BackgroundColor, BorderColor, BorderRadius, BoxShadow, Node, PositionType, UiRect, Val, ZIndex,
};
use plurimus::core::ratatui_core::style::Modifier;
use plurimus::core::ratatui_core::style::Style;
use plurimus::core::ratatui_core::text::Line;

use super::card::{Card, pad_the_card};
use crate::guard::MIN_SIZE;
use crate::layout::{Body, Rule};
use crate::pane::{BORDERS, FrameRows, Framed, JOIN, Lit};
use crate::theme::{Repainted, Theme, say, text};
use retiretui_client::present::basis_name;

/// The one pixel the window names: a line thinner than any cell, drawn
/// where a terminal draws a cell of box glyphs.
pub(super) const HAIRLINE_PX: f32 = 1.0;
#[expect(
    clippy::disallowed_methods,
    reason = "a hairline is thinner than any cell"
)]
pub(super) const HAIRLINE: Val = Val::Px(HAIRLINE_PX);
pub(super) const HALF_CELL_WIDE: Val = Val::Em(0.5);
const HALF_ROW: f32 = 0.5;
const HALF_CELL_TALL: Val = Val::Rem(HALF_ROW);
/// A frame's line is inset a cell across and half a row down from its
/// node, so panes sit two cells and a row apart; what it holds is three
/// cells in, three rows down - a two-row header under the line, half a
/// row clear under the header's hairline - and a row up.
const LINE_INSET: f32 = 1.0;
pub(super) const FRAME_PADDING: (f32, f32, f32) = (3.0, 3.0, 1.0);
/// The title's place: in from the line as far as what the frame holds,
/// on the header's middle row.
pub(super) const TITLE_AT: (Val, Val) = (Val::Em(FRAME_PADDING.0 - LINE_INSET), Val::Rem(1.0));
/// Rows a frame takes over the terminal's borders, which a pane sized in
/// rows grows by.
pub(super) const HEADER_GROWTH: u16 = (FRAME_PADDING.1 + FRAME_PADDING.2) as u16 - BORDERS;
/// A pane's corner, and a sheet's.
const PANE_RADIUS: Val = Val::Em(1.0);
const SHEET_RADIUS: Val = Val::Em(1.25);
/// How far a sheet's shadow falls, and how far it blurs.
const SHADOW_FALL: Val = Val::Rem(0.5);
const SHADOW_BLUR: Val = Val::Rem(2.0);
const SHADOW_ALPHA: f32 = 0.55;
/// Under every sibling, whenever they were spawned: a sheet's fill is on
/// its line, and what the sheet holds comes later.
const BENEATH: ZIndex = ZIndex(-1);

/// The cells the window keeps clear around the body on each side: across,
/// and down.
pub(super) const MARGIN: (u16, u16) = (3, 1);

pub fn plugin(app: &mut App) {
    app.insert_resource(FrameRows(BORDERS + HEADER_GROWTH));
    app.add_observer(margin_the_body);
    app.add_observer(line_the_frame);
    app.add_observer(line_the_rule);
    app.add_systems(
        Update,
        (paint_ruled, paint_frames, paint_rules).in_set(Repainted),
    );
}

/// A node whose border is a hairline in the border colour, whatever the
/// theme.
#[derive(Component, Debug)]
pub(super) struct Ruled;

/// Painted as it appears, and again with the theme.
fn paint_ruled(theme: Res<Theme>, ruled: Query<(Entity, Ref<Ruled>)>, mut commands: Commands) {
    for (node, ruled) in &ruled {
        if theme.is_changed() || ruled.is_added() {
            commands
                .entity(node)
                .insert(BorderColor::all(theme.ink(theme.bordered())));
        }
    }
}

/// Boxes `node` in a hairline with a half-cell radius, `padding` cells
/// clear either side of what it holds.
pub(super) fn box_in(node: &mut Node, padding: f32) {
    node.border = UiRect::all(HAIRLINE);
    node.border_radius = BorderRadius::all(HALF_CELL_WIDE);
    node.padding = UiRect::horizontal(Val::Em(padding));
}

/// The window's floor in cells: the shell's, the margin around it and the
/// header's rows, so every page and the shortest form keep the cells the
/// guard promised them.
pub(super) const fn floor_cells() -> (u16, u16) {
    (
        MIN_SIZE.cols + 2 * MARGIN.0,
        MIN_SIZE.rows + 2 * MARGIN.1 + HEADER_GROWTH,
    )
}

fn margin_the_body(added: On<Add<Body>>, mut nodes: Query<&mut Node>) {
    if let Ok(mut node) = nodes.get_mut(added.entity) {
        node.padding = UiRect::axes(Val::Em(f32::from(MARGIN.0)), Val::Rem(f32::from(MARGIN.1)));
    }
}

/// The line and the title a frame or a rule is drawn with.
#[derive(Component, Debug, Clone, Copy)]
struct Lined {
    line: Entity,
    title: Entity,
}

/// Hangs a `line` under `parent` with its title `title_at` - across, and
/// down - from where the parent starts, and gives back the line.
fn line(commands: &mut Commands, parent: Entity, line: Node, title_at: (Val, Val)) -> Entity {
    let line = commands.spawn((line, BENEATH, ChildOf(parent))).id();
    let title = commands
        .spawn((
            text(),
            Node {
                position_type: PositionType::Absolute,
                left: title_at.0,
                top: title_at.1,
                ..Node::default()
            },
            ChildOf(parent),
        ))
        .id();
    commands.entity(parent).insert(Lined { line, title });
    line
}

/// A frame's line is inset from its node, and its first rows are a header:
/// the title centred on them, in from the line as far as what the frame
/// holds, and a hairline under them. A frame that covers what it stands
/// over is a sheet: rounder, filled, and casting a shadow.
fn line_the_frame(
    added: On<Add<Framed>>,
    frames: Query<(&Framed, Has<Card>)>,
    mut nodes: Query<&mut Node>,
    mut commands: Commands,
) {
    let frame = added.entity;
    let (Ok((dressed, is_card)), Ok(mut node)) = (frames.get(frame), nodes.get_mut(frame)) else {
        return;
    };
    let is_sheet = dressed.is_covering;
    let edge = Node {
        position_type: PositionType::Absolute,
        left: Val::Em(LINE_INSET),
        right: Val::Em(LINE_INSET),
        top: HALF_CELL_TALL,
        bottom: HALF_CELL_TALL,
        border: UiRect::all(HAIRLINE),
        border_radius: BorderRadius::all(if is_sheet { SHEET_RADIUS } else { PANE_RADIUS }),
        ..Node::default()
    };
    let line = line(&mut commands, frame, edge, TITLE_AT);
    if is_card {
        pad_the_card(&mut commands, &mut node, line, dressed.title.is_empty());
        return;
    }
    pad_the_frame(&mut node);
    if is_sheet {
        let shadow = Color::BLACK.with_alpha(SHADOW_ALPHA);
        commands.entity(line).insert(BoxShadow::new(
            shadow,
            Val::ZERO,
            SHADOW_FALL,
            Val::ZERO,
            SHADOW_BLUR,
        ));
    }
    let header = Node {
        position_type: PositionType::Absolute,
        left: Val::Em(LINE_INSET),
        right: Val::Em(LINE_INSET),
        top: Val::Rem(FRAME_PADDING.1 - HALF_ROW),
        border: UiRect::top(HAIRLINE),
        ..Node::default()
    };
    commands.spawn((header, Ruled, ChildOf(frame)));
}

/// A frame keeps its padding clear around what it holds, and one sized in
/// rows grows by what its header takes.
fn pad_the_frame(node: &mut Node) {
    node.padding = UiRect::axes(Val::Em(FRAME_PADDING.0), Val::Rem(FRAME_PADDING.1))
        .with_bottom(Val::Rem(FRAME_PADDING.2));
    for size in [&mut node.height, &mut node.min_height] {
        if let Val::Rem(rows) = size {
            *rows += f32::from(HEADER_GROWTH);
        }
    }
}

fn line_the_rule(added: On<Add<Rule>>, mut commands: Commands) {
    let across = Node {
        position_type: PositionType::Absolute,
        left: Val::ZERO,
        right: Val::ZERO,
        top: HALF_CELL_TALL,
        border: UiRect::top(HAIRLINE),
        ..Node::default()
    };
    line(&mut commands, added.entity, across, (Val::ZERO, Val::ZERO));
}

fn paint(
    commands: &mut Commands,
    lined: &Lined,
    edge: Style,
    heading: Line<'static>,
    theme: &Theme,
) {
    commands
        .entity(lined.line)
        .insert(BorderColor::all(theme.ink(edge)));
    say(commands, lined.title, heading, theme);
}

/// `heading` without the basis the shared title says, which the window
/// shows in its top bar instead.
fn without_basis(mut heading: Line<'static>) -> Line<'static> {
    for span in &mut heading.spans {
        for basis in [basis_name(false), basis_name(true)] {
            let said = format!("{JOIN}{basis}");
            if span.content.contains(&said) {
                span.content = span.content.replace(&said, "").into();
            }
        }
    }
    heading
}

/// A sheet is filled with the ground inside its line and never lit: what
/// stands over the page is what holds the keyboard. A card's title is
/// dimmed; a sheet's, and any other's until the frame is lit, is bold in
/// the ink, unless the page styled it.
fn paint_frames(
    theme: Res<Theme>,
    frames: Query<(Ref<Framed>, Ref<Lit>, Ref<Lined>, Has<Card>)>,
    mut commands: Commands,
) {
    for (framed, lit, lined, is_card) in &frames {
        if !theme.is_changed() && !framed.is_changed() && !lit.is_changed() && !lined.is_added() {
            continue;
        }
        if framed.is_covering {
            commands
                .entity(lined.line)
                .insert(BackgroundColor(theme.ground()));
        }
        let is_lit = lit.0 && !framed.is_covering;
        let mut heading = without_basis(framed.title_line(&theme));
        if is_card {
            heading = heading.patch_style(theme.dimmed());
        } else if framed.title_style.is_none() && !is_lit {
            heading = heading.patch_style(Style::new().fg(theme.fg).add_modifier(Modifier::BOLD));
        }
        paint(&mut commands, &lined, theme.edge(is_lit), heading, &theme);
    }
}

fn paint_rules(theme: Res<Theme>, rules: Query<(Ref<Rule>, Ref<Lined>)>, mut commands: Commands) {
    for (rule, lined) in &rules {
        if !theme.is_changed() && !rule.is_changed() && !lined.is_added() {
            continue;
        }
        let dimmed = theme.dimmed();
        let heading = Line::styled(rule.heading(), dimmed);
        paint(&mut commands, &lined, dimmed, heading, &theme);
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::hierarchy::Children;
    use bevy_ecs::observer::Observer;
    use bevy_ecs::prelude::{With, World};
    use bevy_ecs::system::RunSystemOnce;
    use bevy_text::{FontWeight, TextColor, TextFont};

    use super::super::card::Raised;
    use super::*;
    use crate::layout::fixed;
    /// A world with the chrome's observers, and the theme the paint reads.
    fn chromed() -> World {
        let mut world = World::new();
        world.insert_resource(Theme::default());
        world.spawn(Observer::new(margin_the_body));
        world
    }

    #[test]
    fn the_body_keeps_the_margin_and_the_floor_grows_by_it() {
        let mut world = chromed();
        let body = world.spawn((Body, Node::default())).id();
        world.flush();
        let padding = world.get::<Node>(body).unwrap().padding;
        assert_eq!((padding.left, padding.top), (Val::Em(3.0), Val::Rem(1.0)));
        assert_eq!(
            floor_cells(),
            (MIN_SIZE.cols + 6, MIN_SIZE.rows + 4),
            "a margin each side, and the header's rows for the shortest form"
        );
    }

    #[test]
    fn a_frame_s_line_is_inset_under_a_two_row_header_and_a_sheet_casts_a_shadow() {
        let mut world = chromed();
        world.spawn(Observer::new(line_the_frame));
        let pane = world.spawn((Framed::titled("Milestones"), fixed(5.0))).id();
        let sheet = world.spawn((Framed::over("Edit"), Node::default())).id();
        world.flush();
        let header = world
            .query_filtered::<(&Node, &ChildOf), With<Ruled>>()
            .iter(&world)
            .find(|(_, parent)| parent.parent() == pane)
            .map(|(node, _)| (node.top, node.border))
            .unwrap();
        assert_eq!(header, (Val::Rem(2.5), UiRect::top(HAIRLINE)));
        let node = world.get::<Node>(pane).unwrap();
        assert_eq!(
            (node.padding.left, node.padding.top),
            (Val::Em(3.0), Val::Rem(3.0))
        );
        assert_eq!(node.height, Val::Rem(7.0), "grown by the header's rows");
        let lined = |frame| world.get::<Lined>(frame).unwrap();
        let title = world.get::<Node>(lined(pane).title).unwrap();
        assert_eq!((title.left, title.top), (Val::Em(2.0), Val::Rem(1.0)));
        let line = world.get::<Node>(lined(pane).line).unwrap();
        assert_eq!((line.top, line.left), (Val::Rem(0.5), Val::Em(1.0)));
        assert_eq!(line.border_radius, BorderRadius::all(PANE_RADIUS));
        assert!(!world.entity(lined(pane).line).contains::<BoxShadow>());
        let line = world.get::<Node>(lined(sheet).line).unwrap();
        assert_eq!(line.border_radius, BorderRadius::all(SHEET_RADIUS));
        assert!(world.entity(lined(sheet).line).contains::<BoxShadow>());
        assert_eq!(world.get::<ZIndex>(lined(sheet).line), Some(&BENEATH));
    }

    #[test]
    fn a_hairline_that_appears_after_the_theme_settled_is_still_painted() {
        let mut world = chromed();
        world.spawn(Observer::new(line_the_frame));
        world.run_system_cached(paint_ruled).unwrap();
        let pane = world
            .spawn((Framed::titled("Milestones"), Node::default()))
            .id();
        world.flush();
        world.run_system_cached(paint_ruled).unwrap();
        world.flush();
        let theme = world.resource::<Theme>().clone();
        let header = world
            .query_filtered::<(Entity, &ChildOf), With<Ruled>>()
            .iter(&world)
            .find(|(_, parent)| parent.parent() == pane)
            .map(|(header, _)| header)
            .unwrap();
        let painted = world.get::<BorderColor>(header).map(|border| border.top);
        assert_eq!(painted, Some(theme.ink(theme.bordered())));
    }

    #[test]
    fn a_card_has_no_header_row_and_is_raised_with_its_title_on_a_row_of_its_own() {
        let mut world = chromed();
        world.spawn(Observer::new(line_the_frame));
        let card = world
            .spawn((Framed::titled("Ends with"), Card, Node::default()))
            .id();
        let band = world
            .spawn((Framed::titled(""), Card, Node::default()))
            .id();
        world.flush();
        let headers = world
            .query_filtered::<&ChildOf, With<Ruled>>()
            .iter(&world)
            .filter(|parent| parent.parent() == card)
            .count();
        assert_eq!(headers, 0, "no header hairline");
        let lined = *world.get::<Lined>(card).unwrap();
        assert!(world.entity(lined.line).contains::<Raised>());
        let padding = world.get::<Node>(card).unwrap().padding;
        assert_eq!((padding.left, padding.top), (Val::Em(3.0), Val::Rem(2.0)));
        let title = world.get::<Node>(lined.title).unwrap();
        assert_eq!((title.left, title.top), (Val::Em(2.0), Val::Rem(1.0)));
        let band = world.get::<Node>(band).unwrap().padding;
        assert_eq!(
            band.top,
            Val::Rem(1.0),
            "an untitled card keeps no title row"
        );
    }

    #[test]
    fn a_card_s_title_is_dimmed_and_an_unlit_pane_s_or_a_sheet_s_bold_in_the_ink() {
        let mut world = chromed();
        world.spawn(Observer::new(line_the_frame));
        let card = world
            .spawn((Framed::titled("Ends with"), Card, Node::default()))
            .id();
        let pane = world
            .spawn((Framed::titled("Milestones"), Node::default()))
            .id();
        let sheet = world
            .spawn((Framed::over("Edit"), Lit(true), Node::default()))
            .id();
        world.flush();
        world.run_system_once(paint_frames).unwrap();
        world.flush();
        let theme = world.resource::<Theme>().clone();
        let said = |frame| {
            let title = world.get::<Lined>(frame).unwrap().title;
            let span = world.get::<Children>(title).unwrap()[0];
            let color = world.get::<TextColor>(span).unwrap().0;
            let weight = world.get::<TextFont>(span).unwrap().weight;
            (color, weight)
        };
        assert_eq!(said(card), (theme.ink(theme.dimmed()), FontWeight::NORMAL));
        assert_eq!(said(pane), (theme.ink(Style::new()), FontWeight::BOLD));
        assert_eq!(said(sheet), (theme.ink(Style::new()), FontWeight::BOLD));
    }

    #[test]
    fn a_heading_is_said_without_the_basis() {
        let said = |title: &str| {
            without_basis(Line::from(title.to_owned()))
                .spans
                .into_iter()
                .map(|span| span.content.into_owned())
                .collect::<String>()
        };
        assert_eq!(
            said(" Ledger · today's dollars · Balances by treatment "),
            " Ledger · Balances by treatment "
        );
        assert_eq!(said(" Net worth · future dollars "), " Net worth ");
        assert_eq!(said(" Accounts "), " Accounts ");
    }
}
