//! The file picker in the window. plurimus's picker steps its list as it
//! does in the terminal and bevy edits and draws its field; the window says
//! the prompt as text, lays the list out under the field as the picker
//! places it in cells so the two agree, and gives it a node for the row
//! skin.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Add, Commands, Component, Entity, IntoScheduleConfigs, On, Query, Res};
use bevy_ui::{Node, PositionType, Val};
use plurimus_filepicker::{FilePickerLook, FilePickerParts};

use crate::theme::{Repainted, Theme, say, text};

pub fn plugin(app: &mut App) {
    app.add_observer(place_the_parts);
    app.add_systems(Update, paint_prompts.in_set(Repainted));
}

/// The prompt's node under a picker.
#[derive(Component, Debug)]
struct Prompt(Entity);

/// The list on the rows under the field, as `split_area` cuts the
/// picker's cells; the prompt before the field.
fn place_the_parts(
    added: On<Add<FilePickerParts>>,
    pickers: Query<&FilePickerParts>,
    mut commands: Commands,
) {
    let picker = added.entity;
    let Ok(parts) = pickers.get(picker) else {
        return;
    };
    commands
        .entity(parts.list)
        .entry::<Node>()
        .or_default()
        .and_modify(|mut node| {
            node.position_type = PositionType::Absolute;
            node.top = Val::Rem(1.0);
            node.left = Val::ZERO;
            node.right = Val::ZERO;
            node.bottom = Val::ZERO;
        });
    let prompt = commands
        .spawn((text(), on_the_first_row(), ChildOf(picker)))
        .id();
    commands.entity(picker).insert(Prompt(prompt));
}

fn on_the_first_row() -> Node {
    Node {
        position_type: PositionType::Absolute,
        top: Val::ZERO,
        left: Val::ZERO,
        height: Val::Rem(1.0),
        ..Node::default()
    }
}

fn paint_prompts(
    theme: Res<Theme>,
    pickers: Query<(Ref<Prompt>, Ref<FilePickerLook>)>,
    mut commands: Commands,
) {
    for (prompt, look) in &pickers {
        if !theme.is_changed() && !look.is_changed() && !prompt.is_added() {
            continue;
        }
        say(&mut commands, prompt.0, look.prompt.clone(), &theme);
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::hierarchy::Children;
    use plurimus::bui::bevy_ui_widgets::TextInput as Input;
    use plurimus::core::UiArea;
    use plurimus::core::ratatui_core::layout::Rect;

    use super::*;
    use crate::support::{Headless, SIZE, headless_app_at, is_browsing, scratch_workspace};

    /// The window skin over a shell launched on a directory, which opens
    /// the picker.
    fn browsing() -> Headless {
        let mut app = headless_app_at(scratch_workspace(""), SIZE);
        app.add_plugins((plugin, super::super::widgets::plugin));
        app.update();
        app.update();
        assert!(is_browsing(&app));
        app
    }

    fn parts(app: &mut Headless) -> (Entity, FilePickerParts) {
        let world = app.world_mut();
        world
            .query::<(Entity, &FilePickerParts)>()
            .single(world)
            .map(|(picker, parts)| (picker, *parts))
            .expect("one picker is open")
    }

    fn area(app: &Headless, entity: Entity) -> Rect {
        match app.world().get::<UiArea>(entity) {
            Some(UiArea::Fixed(rect)) => *rect,
            other => panic!("{entity} has no fixed area: {other:?}"),
        }
    }

    #[test]
    fn the_parts_lie_where_the_picker_places_them_and_the_field_is_upstream_s() {
        let mut app = browsing();
        let (picker, parts) = parts(&mut app);
        let whole = area(&app, picker);
        let field = area(&app, parts.field);
        let list = area(&app, parts.list);
        assert!(!whole.is_empty(), "{whole:?}");
        assert_eq!(
            (field.y, field.height, field.right()),
            (whole.y, 1, whole.right())
        );
        assert_eq!(
            (list.x, list.y, list.width, list.height),
            (whole.x, whole.y + 1, whole.width, whole.height - 1)
        );
        let world = app.world();
        assert!(
            world.entity(parts.field).contains::<Input>(),
            "the field is upstream's, which bevy edits and draws"
        );
        let rows = world.get::<Children>(parts.list).expect("rows");
        assert!(rows.len() > 1);
        assert!(rows.iter().all(|&row| world.entity(row).contains::<Node>()));
    }
}
