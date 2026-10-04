//! The line under the Roth Conversions page, saying what the keys do
//! where the keyboard is.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{IntoScheduleConfigs, Local, Query, Res, With};
use bevy_input_focus::InputFocus;
use plurimus::core::UiWidget;

use super::super::options::OptionsTable;
use super::super::{HelpLine, show_help, writes_it};
use super::panes::ConversionsTable;
use super::{PICK_DESTINATION, Swept, held};
use crate::command::{Keymap, TAKE_LADDER, WRITE_LADDER};
use crate::edit::Draft;
use crate::nav::{self, Page, ShownSurface};
use crate::theme::{Repainted, Theme};
use retiretui_client::searches::ladders::ABOUT;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        say_help
            .run_if(nav::shows(Page::RothConversions))
            .before(Repainted),
    );
}

const ON_OPTIONS: &str = "⏎ takes the highlighted ladder into the plan, after asking";
const ON_CONVERSIONS: &str = "takes this ladder into the plan, after asking";

/// What the keys do with the keyboard on the ladder's years: nothing the
/// page's own line does not say, where taking it has no key.
pub(super) fn on_conversions(keymap: &Keymap) -> String {
    match keymap.label_named(TAKE_LADDER, 0) {
        "" => ABOUT.to_owned(),
        take => format!("{take} {ON_CONVERSIONS}{}", writes_it(keymap, WRITE_LADDER)),
    }
}

fn say_help(
    (draft, focus, shown, theme): (Res<Draft>, Res<InputFocus>, ShownSurface, Res<Theme>),
    keymap: Res<Keymap>,
    places: (
        Query<(), With<OptionsTable<Swept>>>,
        Query<(), With<ConversionsTable>>,
    ),
    mut said: Local<String>,
    mut lines: Query<(&mut UiWidget, &HelpLine)>,
) {
    let is_moved = draft.is_changed() || focus.is_changed() || shown.is_changed();
    if !(is_moved || theme.is_changed()) {
        return;
    }
    let (options, conversions) = places;
    let text = match focus.get() {
        _ if held(&draft).is_err() => PICK_DESTINATION.to_owned(),
        Some(holder) if options.contains(holder) => {
            format!("{ON_OPTIONS}{}", writes_it(&keymap, WRITE_LADDER))
        }
        Some(holder) if conversions.contains(holder) => on_conversions(&keymap),
        _ => ABOUT.to_owned(),
    };
    if *said == text && !theme.is_changed() {
        return;
    }
    show_help(&mut lines, Page::RothConversions, &text, &theme);
    *said = text;
}
