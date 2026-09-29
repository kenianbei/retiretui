//! A form's fields scroll in a column over its foot: the form stands as
//! tall as the rows on show, the body capping it, and the keyboard's row
//! is kept in view.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Changed, Entity, IntoScheduleConfigs, Query, Ref, Res, With};
use bevy_input_focus::InputFocus;
use bevy_ui::{ComputedNode, Display, Node, ScrollPosition};
use plurimus::bui::ComputedNodeRect;
use plurimus::core::UiWidget;
use plurimus::widgets::ratatui_widgets::scrollbar::{
    Scrollbar, ScrollbarOrientation, ScrollbarState,
};

use super::build::{BELOW_FIELDS, FormBar, FormFields};
use super::group::{self, Dependent};
use crate::overlay::{self, Centred};

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            fit_forms
                .after(group::place_dependents)
                .in_set(super::EditSystems::Place),
            reveal_focused,
            draw_bars,
        ),
    );
}

/// Each form stands as tall as its rows on show over its foot.
fn fit_forms(
    moved: Query<(), (Changed<Node>, With<Dependent>)>,
    columns: Query<(Ref<FormFields>, &ChildOf, &Children)>,
    rows: Query<&Node>,
    mut boxes: Query<&mut Centred>,
) {
    let is_moved = !moved.is_empty();
    for (column, form, held) in &columns {
        if !is_moved && !column.is_added() {
            continue;
        }
        let is_shown = |row| {
            rows.get(row)
                .is_ok_and(|node| node.display != Display::None)
        };
        let shown = held.iter().filter(|&&row| is_shown(row)).count();
        let Ok(mut centred) = boxes.get_mut(form.parent()) else {
            continue;
        };
        let shown = u16::try_from(shown).unwrap_or(u16::MAX);
        overlay::hold(&mut centred, shown.saturating_add(BELOW_FIELDS));
    }
}

/// Scrolls a form's fields by the least that shows the row holding the
/// keyboard. Every row is a cell tall, so a row's place is the rows on show
/// above it: a rect cannot say where a row scrolled above the screen is.
fn reveal_focused(
    focus: Res<InputFocus>,
    parents: Query<&ChildOf>,
    nodes: Query<&Node>,
    mut columns: Query<(&ComputedNodeRect, &Children, &mut ScrollPosition), With<FormFields>>,
) {
    if !focus.is_changed() {
        return;
    }
    let Some(widget) = focus.get() else {
        return;
    };
    let mut held = std::iter::once(widget).chain(parents.iter_ancestors(widget));
    let Some((row, column)) = held.find_map(|entity| {
        let column = parents.get(entity).ok()?.parent();
        columns.contains(column).then_some((entity, column))
    }) else {
        return;
    };
    let Ok((view, rows, mut scroll)) = columns.get_mut(column) else {
        return;
    };
    let is_shown = |each: &&Entity| {
        nodes
            .get(**each)
            .is_ok_and(|node| node.display != Display::None)
    };
    let above = rows
        .iter()
        .take_while(|&&each| each != row)
        .filter(is_shown)
        .count();
    let (top, height) = (above as f32, f32::from(view.content.height));
    if top < scroll.0.y {
        scroll.0.y = top;
    } else if top + 1.0 > scroll.0.y + height {
        scroll.0.y = top + 1.0 - height;
    }
}

/// Each form's bar shows where its fields are scrolled to, as a table's
/// does. Layout moves a column's scroll unmarked, so the last layout's is
/// compared with what the bar last drew.
fn draw_bars(
    columns: Query<(&ComputedNode, &FormFields)>,
    mut bars: Query<(&mut FormBar, &mut UiWidget)>,
) {
    for (column, fields) in &columns {
        let Ok((mut bar, mut widget)) = bars.get_mut(fields.bar) else {
            continue;
        };
        let hidden = (column.content_size.y - column.size.y).max(0.0) as usize;
        let drawn = (hidden > 0).then(|| (hidden, column.scroll_position.y.max(0.0) as usize));
        if bar.drawn == drawn {
            continue;
        }
        bar.drawn = drawn;
        *widget = drawn.map_or_else(UiWidget::default, |(hidden, above)| {
            // An offset from none to every hidden row is a place.
            let state = ScrollbarState::new(hidden + 1).position(above);
            UiWidget::stateful(Scrollbar::new(ScrollbarOrientation::VerticalRight), state)
        });
    }
}
