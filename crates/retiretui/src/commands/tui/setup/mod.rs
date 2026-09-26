//! Starting a plan: one form asking a household's shape, standing over
//! the shell while it holds no document and whenever a new plan is asked
//! for over one. Creating it builds the plan, names it, and opens it.

mod examples;
mod generate;

pub use examples::EXAMPLES;

use std::path::PathBuf;

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::prelude::{Commands, Entity, In, IntoScheduleConfigs, Res, ResMut, Resource, World};
use retiretui_engine::plan::{Dollars, FilingStatus, Plan};
use serde::Deserialize;
use toml::Table;

use super::command::Outcome;
use super::confirm::{Answer, Confirm};
use super::documents::{self, Browsing};
use super::edit::{
    self, Draft, EditSession, FieldSpec, FormButton, Ops, Row, Slot, ToolAnswers, Vocabulary,
};
use super::journal;
use super::nav::{ActivePage, Group, LastShown, PageSystems};
use super::overlay;
use super::session::{Session, Today};
use super::sidebar;

pub fn plugin(app: &mut App) {
    app.init_resource::<Composed>();
    app.add_systems(
        Update,
        (hold_the_plan_tab, land_in_the_plan).in_set(PageSystems::Turn),
    );
    app.add_systems(Update, keep_the_form_up.before(edit::EditSystems::Seed));
}

/// Where the household is in life. It decides only what the generated
/// plan is pre-filled with; every question is asked of everyone.
#[derive(Clone, Copy, PartialEq, Eq, Default, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum LifeStage {
    /// Earning, and retiring at an age still ahead.
    #[default]
    Working,
    /// Retired already: no salary, and benefits from the first year.
    Retired,
}

impl LifeStage {
    /// Every stage, in the order the form offers them.
    pub const ALL: &'static [Self] = &[Self::Working, Self::Retired];

    /// The stage as the form spells it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::Retired => "retired",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Working => "Working",
            Self::Retired => "Retired",
        }
    }
}

/// The answers as the form holds them. Every one is optional because a
/// form is answered a field at a time, and the generator fills what was
/// left blank.
#[derive(Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
struct SetupAnswers {
    example: Option<String>,
    filing: Option<FilingStatus>,
    stage: Option<LifeStage>,
    name: Option<String>,
    birth_year: Option<i16>,
    retirement_age: Option<u8>,
    working_since: Option<i16>,
    salary: Option<Dollars>,
    social_security: Option<Dollars>,
    claim_age: Option<u8>,
    partner_name: Option<String>,
    partner_birth_year: Option<i16>,
    partner_retirement_age: Option<u8>,
    partner_working_since: Option<i16>,
    partner_salary: Option<Dollars>,
    partner_social_security: Option<Dollars>,
    partner_claim_age: Option<u8>,
}

/// One person's answers, so the two are asked and read the same way.
struct Answered<'a> {
    name: Option<&'a str>,
    birth_year: Option<i16>,
    retirement_age: Option<u8>,
    working_since: Option<i16>,
    salary: Option<Dollars>,
    social_security: Option<Dollars>,
    claim_age: Option<u8>,
}

impl SetupAnswers {
    fn filing(&self) -> FilingStatus {
        self.filing.unwrap_or(FilingStatus::Single)
    }

    fn stage(&self) -> LifeStage {
        self.stage.unwrap_or_default()
    }

    fn first(&self) -> Answered<'_> {
        Answered {
            name: self.name.as_deref(),
            birth_year: self.birth_year,
            retirement_age: self.retirement_age,
            working_since: self.working_since,
            salary: self.salary,
            social_security: self.social_security,
            claim_age: self.claim_age,
        }
    }

    fn partner(&self) -> Answered<'_> {
        Answered {
            name: self.partner_name.as_deref(),
            birth_year: self.partner_birth_year,
            retirement_age: self.partner_retirement_age,
            working_since: self.partner_working_since,
            salary: self.partner_salary,
            social_security: self.partner_social_security,
            claim_age: self.partner_claim_age,
        }
    }
}

/// Whether the household is described by the answers rather than taken
/// from an example, which is when there is anything to answer.
fn is_answered(answers: &Table) -> bool {
    !answers.contains_key("example")
}

const FIELDS: &[FieldSpec] = &[
    FieldSpec::choice("example", "Start from", Vocabulary::Example)
        .blank("My own answers")
        .help("An example household's plan to start from, or blank to answer for your own."),
    FieldSpec::choice("filing", "Filing status", Vocabulary::FilingStatus)
        .help("How you file your federal return. Married filing jointly adds a partner.")
        .shown_when(is_answered),
    FieldSpec::choice("stage", "Life stage", Vocabulary::LifeStage)
        .help("Whether you are still earning or already retired.")
        .shown_when(is_answered),
    FieldSpec::text("name", "Your name")
        .help("A first name is enough; it labels what is yours.")
        .shown_when(is_answered),
    FieldSpec::whole("birth_year", "Birth year")
        .help("The year you were born, such as 1975.")
        .shown_when(is_answered),
    FieldSpec::whole("retirement_age", "Retirement age")
        .help("The age you stop working, or stopped. Your salary ends that year.")
        .shown_when(is_answered),
    FieldSpec::whole("working_since", "Working since")
        .help("The year you started working. Blank means the year you turned 22.")
        .shown_when(is_answered),
    FieldSpec::money("salary", "Salary")
        .help("What you earn per year before tax, or last earned, in today's dollars.")
        .shown_when(is_answered),
    FieldSpec::money("social_security", "Social Security")
        .help("Your yearly benefit at the age you claim, from your SSA statement. Blank computes it from your salary.")
        .shown_when(is_answered),
    FieldSpec::whole("claim_age", "Claim age")
        .help("The age you start Social Security, from 62 to 70. Blank means 67.")
        .shown_when(is_answered),
    FieldSpec::text("partner_name", "Partner's name")
        .help("Your partner's first name.")
        .shown_when(is_answered),
    FieldSpec::whole("partner_birth_year", "Partner's birth year")
        .help("The year your partner was born.")
        .shown_when(is_answered),
    FieldSpec::whole("partner_retirement_age", "Partner's retirement age")
        .help("The age your partner stops working, or stopped. Blank means the same age as you.")
        .shown_when(is_answered),
    FieldSpec::whole("partner_working_since", "Partner's working since")
        .help("The year your partner started working. Blank means the year they turned 22.")
        .shown_when(is_answered),
    FieldSpec::money("partner_salary", "Partner's salary")
        .help("What your partner earns per year before tax, or last earned, in today's dollars.")
        .shown_when(is_answered),
    FieldSpec::money("partner_social_security", "Partner's Social Security")
        .help("Your partner's yearly benefit at the age they claim. Blank computes it from their salary.")
        .shown_when(is_answered),
    FieldSpec::whole("partner_claim_age", "Partner's claim age")
        .help("The age your partner starts Social Security. Blank means 67.")
        .shown_when(is_answered),
];

impl ToolAnswers for SetupAnswers {
    const SLOT: &'static str = "new-plan";
}

const TITLE: &str = "New plan";
const CANCEL: &str = "Cancel";
const CREATE: &str = "Create";

/// The form stands over whatever the shell shows, so it is of no page.
const OPS: Ops = Ops::tool::<SetupAnswers>(None, TITLE, FIELDS).acting([CANCEL, CREATE], act);
const _: () = assert!(
    edit::help_fits(OPS),
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
    let built = edit::from_table::<SetupAnswers>(answers).and_then(|answers| {
        match answers.example.as_deref().and_then(examples::named) {
            Some((file, text)) => Plan::from_toml_str(text)
                .map(|plan| (plan, Some(file)))
                .map_err(|error| format!("{file}: {error}")),
            None => generate::plan(&answers, start_year, tables).map(|plan| (plan, None)),
        }
    });
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
    if let Err(refusal) = edit::write_draft(&draft, &path) {
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

/// The form stands where the plan's own tab would be, so that is the tab
/// a shell holding no document is on.
fn hold_the_plan_tab(session: Res<Session>, mut active: ResMut<ActivePage>) {
    if session.is_changed() && session.is_empty() {
        active.set_if_neq(ActivePage(Group::Plan.first()));
    }
}

#[cfg(test)]
mod generate_tests;
#[cfg(test)]
mod tests;
