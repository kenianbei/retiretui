//! How the picker's rows are written over the ones already listed.

use bevy_ecs::hierarchy::Children;
use bevy_ecs::prelude::{Entity, World};
use plurimus::ui::UiStyle;

use super::{Offered, Row, place_rows};
use crate::theme::Theme;

fn listed(world: &World, results: Entity) -> Vec<(Entity, usize, bool)> {
    let children = world.get::<Children>(results).map_or(&[][..], |c| c);
    children
        .iter()
        .map(|&row| {
            let id = world.get::<Row>(row).map_or(usize::MAX, |row| row.0);
            (row, id, world.get::<UiStyle>(row).is_some())
        })
        .collect()
}

#[test]
fn rows_are_written_over_the_ones_listed_and_the_rest_despawned() {
    let mut world = World::new();
    let results = world.spawn_empty().id();
    let theme = Theme::terminal();
    let wide = [Offered::new(0, "zero").dimmed(), Offered::new(1, "one")];
    let first = place_rows(&mut world, results, &wide, &theme);
    let before = listed(&world, results);
    assert_eq!(first, Some(before[0].0));
    assert_eq!(
        before
            .iter()
            .map(|&(_, id, dim)| (id, dim))
            .collect::<Vec<_>>(),
        [(0, true), (1, false)]
    );
    let narrow = [Offered::new(7, "seven")];
    place_rows(&mut world, results, &narrow, &theme);
    let after = listed(&world, results);
    assert_eq!(
        after,
        [(before[0].0, 7, false)],
        "kept, rewritten, undimmed"
    );
    assert!(
        world.get_entity(before[1].0).is_err(),
        "the extra row is gone"
    );
    assert_eq!(place_rows(&mut world, results, &[], &theme), None);
    assert!(listed(&world, results).is_empty());
}
