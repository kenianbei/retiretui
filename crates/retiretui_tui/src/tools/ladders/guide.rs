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
use super::{PICK_DESTINATION, Swept, held};
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

const ON_OPTIONS: &str =
    "⏎ takes the highlighted ladder into the plan, after asking; w writes it as a scenario.";
const ON_CONVERSIONS: &str =
    "t takes this ladder into the plan, after asking; w writes it as a scenario.";

fn say_help(
    (draft, focus, shown, theme): (Res<Draft>, Res<InputFocus>, ShownSurface, Res<Theme>),
    places: (
        Query<(), With<OptionsTable<Swept>>>,
        Query<(), With<ConversionsTable>>,
    ),
    mut said: Local<&'static str>,
    mut lines: Query<(&mut UiWidget, &HelpLine)>,
) {
    let is_moved = draft.is_changed() || focus.is_changed() || shown.is_changed();
    if !(is_moved || theme.is_changed()) {
        return;
    }
    let (options, conversions) = places;
    let text = match focus.get() {
        _ if held(&draft).is_err() => PICK_DESTINATION,
        Some(holder) if options.contains(holder) => ON_OPTIONS,
        Some(holder) if conversions.contains(holder) => ON_CONVERSIONS,
        _ => ABOUT,
    };
    if *said == text && !theme.is_changed() {
        return;
    }
    *said = text;
    show_help(&mut lines, Page::RothConversions, text, &theme);
}
