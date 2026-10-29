//! The window's chrome over the shared markers: a frame's hairline and its
//! title, a rule's line and its title, and each tab's box and label, which
//! a click chooses as a terminal's does.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Add, Commands, Component, Entity, Has, IntoScheduleConfigs, On, Query, Res, With,
};
use bevy_ui::{
    BackgroundColor, BorderColor, BorderRadius, FlexDirection, JustifyContent, Node, PositionType,
    UiRect, Val,
};
use plurimus::bui::bevy_picking::events::PointerClick;
use plurimus::bui::bevy_ui_widgets::ValueChange;
use plurimus::core::ratatui_core::style::Style;
use plurimus::core::ratatui_core::text::Line;
use plurimus::ui::{Checked, InteractionDisabled, UiLabel};

use crate::layout::Rule;
use crate::pane::{Framed, Lit};
use crate::tabbar::BarTab;
use crate::theme::{Repainted, Theme, say, text};

/// The one pixel the window names: a line thinner than any cell, drawn
/// where a terminal draws a cell of box glyphs.
#[expect(
    clippy::disallowed_methods,
    reason = "a hairline is thinner than any cell"
)]
pub(super) const HAIRLINE: Val = Val::Px(1.0);
pub(super) const HALF_CELL_WIDE: Val = Val::Em(0.5);
const HALF_CELL_TALL: Val = Val::Rem(0.5);

pub fn plugin(app: &mut App) {
    app.add_observer(line_the_frame);
    app.add_observer(line_the_rule);
    app.add_observer(box_the_tab);
    app.add_systems(
        Update,
        (paint_frames, paint_rules, paint_tabs).in_set(Repainted),
    );
}

/// The line and the title a frame or a rule is drawn with.
#[derive(Component, Debug)]
struct Lined {
    line: Entity,
    title: Entity,
}

/// Hangs a `line` under `parent` with its title over the line's top edge,
/// `title_left` in from where the parent starts.
fn line(commands: &mut Commands, parent: Entity, line: Node, title_left: Val) {
    let line = commands.spawn((line, ChildOf(parent))).id();
    let title = commands
        .spawn((
            text(),
            Node {
                position_type: PositionType::Absolute,
                left: title_left,
                top: Val::ZERO,
                ..Node::default()
            },
            ChildOf(parent),
        ))
        .id();
    commands.entity(parent).insert(Lined { line, title });
}

/// A frame's line is inset half a cell on every side, so what the frame
/// lays out inside its cell of padding is where a terminal lays it; the
/// title sits over the top line, a cell in.
fn line_the_frame(added: On<Add<Framed>>, mut commands: Commands) {
    let inset = Node {
        position_type: PositionType::Absolute,
        left: HALF_CELL_WIDE,
        right: HALF_CELL_WIDE,
        top: HALF_CELL_TALL,
        bottom: HALF_CELL_TALL,
        border: UiRect::all(HAIRLINE),
        border_radius: BorderRadius::all(HALF_CELL_WIDE),
        ..Node::default()
    };
    line(&mut commands, added.entity, inset, Val::Em(1.0));
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
    line(&mut commands, added.entity, across, Val::ZERO);
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
    commands
        .entity(lined.title)
        .insert(BackgroundColor(theme.ground()));
    say(commands, lined.title, heading, theme);
}

/// A frame that covers what it stands over is on the ground.
fn paint_frames(
    theme: Res<Theme>,
    frames: Query<(Entity, Ref<Framed>, Ref<Lit>, Ref<Lined>)>,
    mut commands: Commands,
) {
    for (frame, framed, lit, lined) in &frames {
        if !theme.is_changed() && !framed.is_changed() && !lit.is_changed() && !lined.is_added() {
            continue;
        }
        if framed.is_covering {
            commands
                .entity(frame)
                .insert(BackgroundColor(theme.ground()));
        }
        let heading = framed.title_line(&theme);
        paint(&mut commands, &lined, theme.edge(lit.0), heading, &theme);
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

/// A tab's label, and how the tab was last drawn.
#[derive(Component, Debug)]
struct Boxed {
    label: Entity,
    is_lit: bool,
    is_dead: bool,
}

/// A tab is a box as tall as the row, its label on the middle row a cell
/// in from the line, as a terminal's bordered tab is.
fn box_the_tab(added: On<Add<BarTab>>, mut commands: Commands) {
    let tab = added.entity;
    let label = commands.spawn((text(), ChildOf(tab))).id();
    commands
        .entity(tab)
        .insert((
            Node {
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                padding: UiRect::horizontal(Val::Em(1.0)),
                border: UiRect::all(HAIRLINE),
                border_radius: BorderRadius::all(HALF_CELL_WIDE),
                ..Node::default()
            },
            Boxed {
                label,
                is_lit: false,
                is_dead: false,
            },
        ))
        .observe(choose_on_click);
}

/// A click on a live tab is the choice a terminal's click makes: the bar
/// says which of its items was chosen, and the shell turns to it.
fn choose_on_click(
    click: On<PointerClick>,
    tabs: Query<(&ChildOf, Has<InteractionDisabled>), With<BarTab>>,
    mut commands: Commands,
) {
    let tab = click.entity;
    let Ok((bar, is_dead)) = tabs.get(tab) else {
        return;
    };
    if is_dead {
        return;
    }
    commands.trigger(ValueChange {
        source: bar.parent(),
        value: tab,
        is_final: true,
    });
}

fn paint_tabs(
    theme: Res<Theme>,
    mut tabs: Query<
        (
            Entity,
            Ref<UiLabel>,
            Has<Checked>,
            Has<InteractionDisabled>,
            &mut Boxed,
        ),
        With<BarTab>,
    >,
    mut commands: Commands,
) {
    for (tab, label, is_lit, is_dead, mut boxed) in &mut tabs {
        let is_as_drawn = boxed.is_lit == is_lit && boxed.is_dead == is_dead;
        if is_as_drawn && !theme.is_changed() && !label.is_changed() && !boxed.is_added() {
            continue;
        }
        boxed.is_lit = is_lit;
        boxed.is_dead = is_dead;
        commands
            .entity(tab)
            .insert(BorderColor::all(theme.ink(theme.edge(is_lit))));
        let line = if is_lit {
            label.0.clone().patch_style(theme.focused())
        } else if is_dead {
            label.0.clone().patch_style(theme.dimmed())
        } else {
            label.0.clone()
        };
        say(&mut commands, boxed.label, line, &theme);
    }
}
