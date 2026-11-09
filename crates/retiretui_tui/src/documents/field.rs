//! The file picker's path field as upstream's text input, in both skins:
//! plurimus draws it on a terminal's cells from its node, bevy in a window.
//! The field keeps Left, Right, Home and End for its caret, so the picker
//! climbs on a chord that passes it.

use bevy_ecs::prelude::{Add, Commands, On, Query};
use bevy_input::keyboard::Key;
use bevy_ui::{Node, Overflow, PositionType, Val};
use plurimus::bui::bevy_text::EditableText;
use plurimus::bui::bevy_ui_widgets::TextInput as FieldInput;
use plurimus::core::UiWidget;
use plurimus::ui::KeyBinding;
use plurimus::widgets::{TextField as CellField, TextInput};
use plurimus_filepicker::{FilePickerAction, FilePickerKeys, FilePickerLook, FilePickerParts};

/// The picker's own keys, with the parent on Alt+Up as a file manager's:
/// the field keeps Left and Right for its caret, where the picker climbs.
/// Bindings are scanned first to last, so the chord comes before plain Up.
pub(super) fn picker_keys() -> FilePickerKeys {
    let mut keys = FilePickerKeys::default();
    let parent = (
        KeyBinding::new(Key::ArrowUp).with_alt(),
        FilePickerAction::Parent,
    );
    keys.0.insert(0, parent);
    keys
}

/// The field the picker spawned as plurimus's cell field becomes
/// upstream's, holding what the cell field was opened on, laid out after
/// the prompt along the picker's first row as the picker places it. A
/// picker taken down as it opens has no field left to dress.
pub(super) fn dress_the_field(
    added: On<Add<FilePickerParts>>,
    pickers: Query<(&FilePickerParts, &FilePickerLook)>,
    engines: Query<&TextInput>,
    mut commands: Commands,
) {
    let Ok((parts, look)) = pickers.get(added.entity) else {
        return;
    };
    let opened = engines
        .get(parts.field)
        .map(|engine| engine.value().to_owned())
        .unwrap_or_default();
    commands
        .entity(parts.field)
        .try_remove::<(CellField, UiWidget)>()
        .try_insert((
            FieldInput,
            EditableText::new(opened),
            Node {
                position_type: PositionType::Absolute,
                top: Val::ZERO,
                left: Val::Em(look.prompt.width() as f32),
                right: Val::ZERO,
                height: Val::Rem(1.0),
                overflow: Overflow::clip(),
                ..Node::default()
            },
        ));
}
