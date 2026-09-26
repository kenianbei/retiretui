//! A form's fields scroll in a column over its foot: the form stands as
//! tall as the rows on show, the body capping it, and the keyboard's row
//! is kept in view.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Changed, IntoScheduleConfigs, Query, Ref, Res, With, Without};
use bevy_input_focus::InputFocus;
use bevy_ui::{ComputedNode, Display, Node, ScrollPosition};
use plurimus::bui::ComputedNodeRect;
use plurimus::core::UiWidget;
use plurimus::widgets::ratatui_widgets::scrollbar::{
    Scrollbar, ScrollbarOrientation, ScrollbarState,
};

use super::build::{BELOW_FIELDS, FormBar, FormFields};
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
            draw_bars,
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

/// Each form's bar shows where its fields are scrolled to, drawn as a
/// table's is, and nothing while they fit. It reads the last layout's.
fn draw_bars(
    columns: Query<(Ref<ComputedNode>, &ChildOf), With<FormFields>>,
    forms: Query<&Children>,
    mut bars: Query<&mut UiWidget, With<FormBar>>,
) {
    for (column, form) in &columns {
        if !column.is_changed() {
            continue;
        }
        let hidden = (column.content_size.y - column.size.y).max(0.0);
        let drawn = if hidden < 1.0 {
            UiWidget::default()
        } else {
            let state =
                ScrollbarState::new(hidden as usize).position(column.scroll_position.y as usize);
            UiWidget::stateful(Scrollbar::new(ScrollbarOrientation::VerticalRight), state)
        };
        let mut held = forms.get(form.parent()).into_iter().flatten();
        let bar = held.find(|&&child| bars.contains(child));
        if let Some(mut bar) = bar.and_then(|&bar| bars.get_mut(bar).ok()) {
            *bar = drawn;
        }
    }
}
