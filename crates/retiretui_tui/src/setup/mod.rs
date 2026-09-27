//! Starting a plan: one form asking a household's shape, standing over
//! the shell while it holds no document and whenever a new plan is asked
//! for over one. Creating it builds the plan, names it, and opens it.

use retiretui_client::setup::{self, FIELDS, SetupAnswers};

use std::path::PathBuf;

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{Commands, Entity, In, IntoScheduleConfigs, Res, ResMut, Resource, World};
use retiretui_engine::plan::Plan;

use super::command::Outcome;
use super::confirm::{Answer, Confirm};
use super::documents::{self, Browsing};
use super::edit::{self, Draft, EditSession, FormButton, Ops, Row, Slot};
use super::journal;
use super::nav::{Group, LastShown, PageSystems};
use super::overlay;
use super::session::{Session, Today};
use super::sidebar;

pub fn plugin(app: &mut App) {
    app.init_resource::<Composed>();
    app.add_systems(Update, land_in_the_plan.in_set(PageSystems::Turn));
    app.add_systems(Update, keep_the_form_up.before(edit::EditSystems::Seed));
}

const TITLE: &str = "New plan";
const CANCEL: &str = "Cancel";
const CREATE: &str = "Create";

/// The form stands over whatever the shell shows, so it is of no page.
const OPS: Ops = Ops::tool::<SetupAnswers>(None, TITLE, FIELDS).acting([CANCEL, CREATE], act);
const _: () = assert!(
    edit::help_fits(OPS.form.fields),
    "a field's help is missing or too long"
);

/// The form's one item: the answers as they stand.
const SETUP_ITEM: (Ops, Option<Entity>, Slot) = (OPS, None, Slot::At(Row(0)));

/// The plan the answers made, and the file it was written to, held from
/// the moment it is built until the shell has opened it.
#[derive(Resource, Default, Debug)]
struct Composed {
    plan: Option<Plan>,
    /// The name the plan is offered under, where it has one of its own.
    named: Option<&'static str>,
    written: Option<PathBuf>,
}

/// The `new` command: the form stands over the shell, the document
/// staying open beneath it until a new one takes its place.
pub fn compose(mut commands: Commands) -> Outcome {
    open(&mut commands);
    Outcome::Done
}

fn open(commands: &mut Commands) {
    commands.run_system_cached_with(edit::open_item, SETUP_ITEM);
}

/// What the form's buttons come to once they have done a form's own work.
fn act(which: FormButton, commands: &mut Commands) {
    if which == FormButton::Apply {
        commands.run_system_cached(create);
    }
}

/// Without a document the form is all there is, so it stands whenever
/// nothing does - the picker a launch opens has first say, asked of the
/// picker itself since it stands a frame after it is opened; a document
/// opening takes it down.
fn keep_the_form_up(
    (session, overlays, browsing): (Res<Session>, Res<overlay::Focus>, Res<Browsing>),
    mut editing: ResMut<EditSession>,
    mut commands: Commands,
) {
    if session.is_empty() {
        if !editing.is_open() && overlays.is_clear() && !browsing.is_open() {
            open(&mut commands);
        }
    } else if session.is_changed() && editing.is_over(None) {
        editing.close();
    }
}

/// Builds the plan the answers describe and stands the form again, for
/// the question of what to call it to be asked over - once what is to
/// become of a draft the open document does not hold is known. Nothing
/// reaches disk, and no document is left, before then.
fn create(world: &mut World) {
    let answers = world.resource::<Draft>().answers::<SetupAnswers>();
    let start_year = world.resource::<Today>().0;
    let tables = &world.resource::<Session>().tables;
    let built = setup::compose(answers, start_year, tables);
    match built {
        Ok((plan, named)) => {
            let mut composed = world.resource_mut::<Composed>();
            composed.plan = Some(plan);
            composed.named = named;
            let _ = world.run_system_cached_with(edit::open_item, SETUP_ITEM);
            let _ = world.run_system_cached(name_the_plan);
        }
        Err(message) => journal::warn(message),
    }
}

fn name_the_plan(
    (draft, session, composed): (Res<Draft>, Res<Session>, Res<Composed>),
    mut confirm: ResMut<Confirm>,
    mut commands: Commands,
) {
    let named = composed.named;
    if !draft.is_dirty() {
        commands.run_system_cached_with(documents::name_new_plan, named);
        return;
    }
    confirm.ask_among(
        format!("Save the changes to {} first?", session.file_name()),
        vec![
            Answer::closing("Cancel"),
            Answer::running("Discard", move |commands| {
                commands.run_system_cached_with(documents::name_new_plan, named);
            })
            .destructive(),
            Answer::running("Save", move |commands| {
                commands.run_system_cached_with(save_then_name, named);
            })
            .primary(),
        ],
    );
}

fn save_then_name(In(named): In<Option<&'static str>>, world: &mut World) {
    match world.run_system_cached(edit::save) {
        Ok(Outcome::Refused(reason)) => journal::warn(reason),
        _ => {
            let _ = world.run_system_cached_with(documents::name_new_plan, named);
        }
    }
}

/// Writes the composed plan under `path` and makes it the document.
pub fn write_new(In(path): In<PathBuf>, world: &mut World) {
    let Some(plan) = world.resource_mut::<Composed>().plan.take() else {
        return;
    };
    let draft = Draft::validated(plan, &world.resource::<Session>().tables);
    let store = world.resource::<Session>().store.as_ref();
    if let Err(refusal) = edit::write_draft(store, &draft, &path) {
        journal::warn(refusal);
        return;
    }
    if documents::land(path.clone().into(), world) {
        world.resource_mut::<Composed>().written = Some(path);
    }
}

/// The file the plan was written to is the document: the shell is in the
/// plan's tab, the keyboard on its sidebar.
fn land_in_the_plan(
    session: Res<Session>,
    mut composed: ResMut<Composed>,
    mut last: ResMut<LastShown>,
    mut commands: Commands,
) {
    if !session.is_changed() || composed.written.is_none() {
        return;
    }
    if session.plan_path != composed.written {
        return;
    }
    *composed = Composed::default();
    *last = LastShown::default();
    commands.queue(|world: &mut World| {
        let _ = sidebar::enter(world, Group::Plan);
    });
}

#[cfg(test)]
mod tests;
