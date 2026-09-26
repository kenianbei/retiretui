//! A form's fields scroll in a column over its foot: the form stands as
//! tall as the rows on show, the body capping it, and the keyboard's row
//! is kept in view.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Changed, IntoScheduleConfigs, Query, Ref, Res, With, Without};
use bevy_input_focus::InputFocus;
use bevy_ui::{Display, Node, ScrollPosition};
use plurimus::bui::ComputedNodeRect;

use super::build::{BELOW_FIELDS, FormFields};
use super::group::{self, Dependent};
use crate::commands::tui::overlay::{self, Centred};

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            fit_forms
                .after(group::place_dependents)
                .in_set(super::EditSystems::Place),
            reveal_focused,
        ),
    );
}

/// Each form stands as tall as its rows on show over its foot.
fn fit_forms(
    moved: Query<(), (Changed<Node>, With<Dependent>, Without<Centred>)>,
    columns: Query<(Ref<FormFields>, &ChildOf, &Children)>,
    rows: Query<&Node, Without<Centred>>,
    mut boxes: Query<(&mut Centred, &mut Node)>,
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
        let Ok((mut centred, mut node)) = boxes.get_mut(form.parent()) else {
            continue;
        };
        let shown = u16::try_from(shown).unwrap_or(u16::MAX);
        overlay::hold(&mut centred, &mut node, shown.saturating_add(BELOW_FIELDS));
    }
}

/// Scrolls a form's fields by the least that shows the row holding the
/// keyboard. The rects are the last layout's.
fn reveal_focused(
    focus: Res<InputFocus>,
    parents: Query<&ChildOf>,
    rects: Query<&ComputedNodeRect>,
    mut columns: Query<(&ComputedNodeRect, &mut ScrollPosition), With<FormFields>>,
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
    let (Ok(row), Ok((view, mut scroll))) = (rects.get(row), columns.get_mut(column)) else {
        return;
    };
    let (row, view) = (row.rect, view.content);
    if row.y < view.y {
        scroll.0.y -= f32::from(view.y - row.y);
    } else if row.bottom() > view.bottom() {
        scroll.0.y += f32::from(row.bottom() - view.bottom());
    }
}
