//! How the Ledger's height is split: the cursor year's detail as tall as
//! its longest pane needs, up to half the page, and the years in the rest.

use bevy_app::{App, Update};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{
    Commands, Component, Entity, IntoScheduleConfigs, Query, SystemSet, With, Without,
};
use bevy_ui::{FlexDirection, Node, Val};
use plurimus::ui::ScrollArea;
use plurimus::widgets::WidgetSystems;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        fit_detail.after(DetailFilled).before(WidgetSystems::Layout),
    );
}

/// The most of the page's height the detail takes, the table keeping the
/// rest.
const DETAIL_MOST: f32 = 50.0;
/// A pane's top and bottom borders.
const PANE_BORDERS: u16 = 2;

/// Where the detail's panes are filled for the cursor year, ahead of the
/// detail being fitted to them.
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) struct DetailFilled;

/// The row of panes under the table, over the cursor year.
#[derive(Component)]
struct LedgerDetail;

/// The row the detail's panes are spawned into, under the years in `view`.
pub(super) fn spawn_detail(commands: &mut Commands, view: Entity) -> Entity {
    let detail = Node {
        flex_direction: FlexDirection::Row,
        height: Val::Px(0.0),
        max_height: Val::Percent(DETAIL_MOST),
        min_height: Val::Px(0.0),
        ..Node::default()
    };
    commands.spawn((detail, LedgerDetail, ChildOf(view))).id()
}

/// Makes the detail as tall as the cursor year's longest pane needs - its
/// tables' rows, header included, and whatever else it holds - which the
/// cap may cut to half the page.
fn fit_detail(
    mut details: Query<(&mut Node, &Children), With<LedgerDetail>>,
    panes: Query<&Children>,
    held: Query<(Option<&ScrollArea>, &Node), Without<LedgerDetail>>,
) {
    let rows_of = |part: Entity| match held.get(part) {
        Ok((Some(scroll), _)) => scroll.content_size.height,
        Ok((None, node)) => match node.height {
            Val::Px(rows) => rows as u16,
            _ => 0,
        },
        Err(_) => 0,
    };
    for (mut node, children) in &mut details {
        let wanted = children
            .iter()
            .map(|&pane| {
                let parts = panes.get(pane).into_iter().flatten();
                parts.map(|&part| rows_of(part)).sum::<u16>()
            })
            .max()
            .unwrap_or(0)
            .saturating_add(PANE_BORDERS);
        let height = Val::Px(f32::from(wanted));
        if node.height != height {
            node.height = height;
        }
    }
}
