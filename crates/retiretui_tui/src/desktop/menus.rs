//! A select's menu in the window. plurimus's menu holds the keyboard and
//! its rows as it does in the terminal; the window draws a shadow of it -
//! a framed column of rows under the select, hung from the root rather
//! than from the select, whose scrolled column would clip a child - and
//! answers a click on a row as the terminal answers a press.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Add, Commands, Component, Entity, Has, IntoScheduleConfigs, On, Query, Remove, Res, With,
};
use bevy_input_focus::InputFocus;
use bevy_ui::{
    BackgroundColor, BorderColor, BorderRadius, ComputedNode, Display, FlexDirection, Node,
    PositionType, UiGlobalTransform, UiRect, Val, ZIndex,
};
use plurimus::bui::bevy_picking::events::PointerClick;
use plurimus::core::UiHidden;
use plurimus::core::ratatui_core::style::Style;
use plurimus::core::ratatui_core::text::{Line, Span};
use plurimus::ui::{ModalDismiss, UiLabel};
use plurimus::widgets::{Activate, MenuItem, MenuPopup};

use super::chrome::{HAIRLINE, HALF_CELL_WIDE};
use crate::layout::{self, Root};
use crate::theme::{Repainted, Theme, say, text};

/// Above the body and whatever overlay stands in it.
const MENU_LAYER: ZIndex = ZIndex(1);

/// A row's label sits a cell in from the frame, as plurimus draws it.
const ROW_MARGIN: &str = " ";

pub fn plugin(app: &mut App) {
    app.add_observer(shadow_the_popup);
    app.add_observer(unshadow_the_popup);
    app.add_observer(row_the_item);
    app.add_observer(unrow_the_item);
    app.add_systems(Update, (place_shadows, paint_rows).in_set(Repainted));
}

/// The window's node for a popup or an item.
#[derive(Component, Debug, Clone, Copy)]
struct Shadow(Entity);

/// The item a row stands for, and whether it was last drawn held.
#[derive(Component, Debug)]
struct Shadowed {
    of: Entity,
    drawn_held: Option<bool>,
}

fn shadow_the_popup(
    added: On<Add<MenuPopup>>,
    roots: Query<Entity, With<Root>>,
    theme: Res<Theme>,
    mut commands: Commands,
) {
    let Ok(root) = roots.single() else {
        return;
    };
    let popup = added.entity;
    let shadow = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(Val::Em(1.0), Val::Rem(1.0)),
                border: UiRect::all(HAIRLINE),
                border_radius: BorderRadius::all(HALF_CELL_WIDE),
                display: Display::None,
                ..Node::default()
            },
            MENU_LAYER,
            ChildOf(root),
        ))
        .id();
    colour(&mut commands, shadow, &theme);
    commands.entity(popup).insert(Shadow(shadow));
}

fn colour(commands: &mut Commands, shadow: Entity, theme: &Theme) {
    commands.entity(shadow).insert((
        BorderColor::all(theme.ink(Style::default())),
        BackgroundColor(theme.ground()),
    ));
}

/// A popup despawned with its form takes its shadow with it; an item
/// relisted takes its row. Either may already be gone with the other.
fn unshadow_the_popup(
    removed: On<Remove<MenuPopup>>,
    shadows: Query<&Shadow>,
    mut commands: Commands,
) {
    if let Ok(shadow) = shadows.get(removed.entity) {
        commands.entity(shadow.0).try_despawn();
    }
}

fn row_the_item(
    added: On<Add<MenuItem>>,
    parents: Query<&ChildOf>,
    shadows: Query<&Shadow>,
    mut commands: Commands,
) {
    let item = added.entity;
    let Ok(popup) = parents.get(item).map(ChildOf::parent) else {
        return;
    };
    let Ok(shadow) = shadows.get(popup) else {
        return;
    };
    let row = commands
        .spawn((
            Shadowed {
                of: item,
                drawn_held: None,
            },
            text(),
            Node {
                height: Val::Rem(1.0),
                flex_shrink: 0.0,
                ..Node::default()
            },
            ChildOf(shadow.0),
        ))
        .observe(choose_on_click)
        .id();
    commands.entity(item).insert(Shadow(row));
}

fn unrow_the_item(removed: On<Remove<MenuItem>>, shadows: Query<&Shadow>, mut commands: Commands) {
    if let Ok(shadow) = shadows.get(removed.entity) {
        commands.entity(shadow.0).try_despawn();
    }
}

/// A click on a row chooses its item and closes the menu, which is what
/// a terminal's press does; the dismissal hands the keyboard back to the
/// select.
fn choose_on_click(
    click: On<PointerClick>,
    rows: Query<&Shadowed>,
    parents: Query<&ChildOf>,
    mut commands: Commands,
) {
    let Ok(row) = rows.get(click.entity) else {
        return;
    };
    let Ok(popup) = parents.get(row.of).map(ChildOf::parent) else {
        return;
    };
    commands.trigger(Activate { entity: row.of });
    commands.trigger(ModalDismiss { entity: popup });
}

/// An open popup's shadow hangs under its select's node, in the cells
/// the node's em and rem resolve to, as wide as its rows; a hidden one is
/// laid out of the tree. The window places it
/// below alone, where the terminal's popover mirrors to the side or above
/// when short of room.
fn place_shadows(
    theme: Res<Theme>,
    popups: Query<(&Shadow, &ChildOf, Has<UiHidden>), With<MenuPopup>>,
    anchors: Query<(&ComputedNode, &UiGlobalTransform)>,
    mut nodes: Query<&mut Node>,
    mut commands: Commands,
) {
    for (shadow, anchor, is_hidden) in &popups {
        let Ok(mut node) = nodes.get_mut(shadow.0) else {
            continue;
        };
        if is_hidden {
            layout::set_display(&mut node, false);
            continue;
        }
        let Ok((computed, transform)) = anchors.get(anchor.parent()) else {
            continue;
        };
        let scale = computed.inverse_scale_factor();
        let half = computed.size() * scale / 2.0;
        let centre = transform.translation * scale;
        let left = Val::Em((centre.x - half.x) / computed.em_size.0);
        let top = Val::Rem((centre.y + half.y) / computed.rem_size.0);
        if node.left != left || node.top != top {
            node.left = left;
            node.top = top;
        }
        layout::set_display(&mut node, true);
        if theme.is_changed() {
            colour(&mut commands, shadow.0, &theme);
        }
    }
}

/// A row says its item's label a cell in, in the focused style while the
/// item holds the keyboard.
fn paint_rows(
    theme: Res<Theme>,
    focus: Res<InputFocus>,
    items: Query<Ref<UiLabel>, With<MenuItem>>,
    mut rows: Query<(Entity, &mut Shadowed)>,
    mut commands: Commands,
) {
    for (row, mut shadowed) in &mut rows {
        let Ok(label) = items.get(shadowed.of) else {
            continue;
        };
        let is_held = focus.get() == Some(shadowed.of);
        let is_stale =
            label.is_changed() || theme.is_changed() || shadowed.drawn_held != Some(is_held);
        if !is_stale {
            continue;
        }
        shadowed.drawn_held = Some(is_held);
        let mut line = Line::from(Span::raw(ROW_MARGIN));
        line.extend(label.0.spans.iter().cloned());
        line.push_span(Span::raw(ROW_MARGIN));
        say(
            &mut commands,
            row,
            line.patch_style(theme.held(is_held)),
            &theme,
        );
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::hierarchy::Children;
    use plurimus::term::KeyCode;

    use super::*;
    use crate::edit::tests::{open, tab_to_field};
    use crate::nav::Page;
    use crate::support::{SIZE, headless_app, press_key};

    /// Stops along an account's form to its kind, a select.
    const KIND_FIELD: usize = 2;

    #[test]
    fn an_open_menu_s_rows_are_shadowed_under_the_root_and_hidden_with_it() {
        let mut app = headless_app(SIZE);
        app.add_plugins(plugin);
        open(&mut app, Page::Accounts);
        tab_to_field(&mut app, KIND_FIELD);
        press_key(&mut app, KeyCode::Char(' '));
        let world = app.world_mut();
        let (popup, shadow) = world
            .query_filtered::<(Entity, &Shadow), With<MenuPopup>>()
            .iter(world)
            .find(|(popup, _)| !world.entity(*popup).contains::<UiHidden>())
            .map(|(popup, shadow)| (popup, shadow.0))
            .expect("the kind's menu is open");
        let items = world
            .get::<Children>(popup)
            .map_or(0, |children| children.len());
        let rows = world
            .get::<Children>(shadow)
            .map_or(0, |children| children.len());
        assert!(items > 1 && rows == items, "{rows} rows for {items} items");
        let parent = world.get::<ChildOf>(shadow).map(ChildOf::parent);
        assert!(
            parent.is_some_and(|parent| world.entity(parent).contains::<Root>()),
            "hung from the root, clear of the form's clip"
        );
        assert_eq!(
            world.get::<Node>(shadow).map(|node| node.display),
            Some(Display::Flex)
        );
        press_key(&mut app, KeyCode::Esc);
        let world = app.world();
        assert_eq!(
            world.get::<Node>(shadow).map(|node| node.display),
            Some(Display::None)
        );
    }
}
