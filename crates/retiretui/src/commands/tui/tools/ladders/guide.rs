//! The line under the Roth Conversions page, saying what the keys do
//! where the keyboard is.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{IntoScheduleConfigs, Local, Query, Res, With};
use bevy_input_focus::InputFocus;
use plurimus::core::UiWidget;

use super::super::options::OptionsTable;
use super::super::{HelpLine, show_help};
use super::panes::ConversionsTable;
use super::{Swept, held};
use crate::commands::tui::edit::Draft;
use crate::commands::tui::nav::{ActivePage, Page};
use crate::commands::tui::theme::{Repainted, Theme};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, say_help.before(Repainted));
}

const PICK_DESTINATION: &str = "Pick the Roth account to convert to under Constraints, and every bracket's ladder is searched.";
const ON_FORM: &str =
    "⏎ edits a constraint; the ladders are searched again once they are applied. ⇧⇥ back to them.";
const ON_OPTIONS: &str =
    "⏎ takes the highlighted ladder into the plan, after asking; w writes it as a scenario.";
const ON_CONVERSIONS: &str =
    "t takes this ladder into the plan, after asking; w writes it as a scenario.";

fn say_help(
    (draft, focus, active, theme): (Res<Draft>, Res<InputFocus>, Res<ActivePage>, Res<Theme>),
    places: (
        Query<(), With<OptionsTable<Swept>>>,
        Query<(), With<ConversionsTable>>,
    ),
    mut said: Local<&'static str>,
    mut lines: Query<(&mut UiWidget, &HelpLine)>,
) {
    let is_moved = draft.is_changed() || focus.is_changed() || active.is_changed();
    if active.0 != Page::RothConversions || !(is_moved || theme.is_changed()) {
        return;
    }
    let (options, conversions) = places;
    let text = match focus.get() {
        _ if held(&draft).is_err() => PICK_DESTINATION,
        Some(holder) if options.contains(holder) => ON_OPTIONS,
        Some(holder) if conversions.contains(holder) => ON_CONVERSIONS,
        _ => ON_FORM,
    };
    if *said == text && !theme.is_changed() {
        return;
    }
    *said = text;
    show_help(&mut lines, Page::RothConversions, text, &theme);
}
