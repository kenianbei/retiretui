//! A plan file opened through whatever reads the page's files: its resolved
//! plan, the files it came from, its issues, and, where it has none, its
//! projection.

use std::cell::{OnceCell, RefCell};
use std::path::{Path, PathBuf};

use retiretui_client::actions::{collect_warnings, sentence};
use retiretui_client::draft::Draft;
use retiretui_client::files::resolve_with_files;
use retiretui_client::forms::{DomainId, Form, ListOps, ToolAnswers};
use retiretui_client::issues::{issue_field, issue_listing, issue_place, issue_words};
use retiretui_client::replies::{ActionsReply, year_row};
use retiretui_client::searches::ladders::aim_at;
use retiretui_client::session::{Projected, Today, YearCursor, span};
use retiretui_client::statement::{self, recorded};
use retiretui_client::store::normal;
use retiretui_engine::market::RunName;
use retiretui_engine::optimize::benefit_estimates;
use retiretui_engine::plan::{Dollars, Item, Plan};
use retiretui_engine::project::{Projection, Summary, YearRow};
use serde::Serialize;

use crate::domain::{list_of, name_at};
use crate::editor::Editor;
use crate::markets::{market_named, replayed};
use crate::tables;
use crate::vocabulary::{form_at, slug_of};

/// A scenario holds only its changes to a base; the plan resolved from it
/// written in its place would lose which were its own.
const OVER_SCENARIO: &str = "a scenario cannot be saved over; save it under a name of its own";

/// A resolved plan and what the gate made of it.
#[derive(Debug)]
pub(crate) struct Document {
    draft: Draft,
    files: Vec<PathBuf>,
    /// The last valid draft's plan and projection, held while it has
    /// issues.
    projected: Option<Projected>,
    /// Each person's benefit estimated at 62, full retirement age and 70
    /// from the projected plan, by id, once asked for.
    estimates: OnceCell<Vec<(String, Estimate)>>,
    /// The last market a view was shown in, replayed from the projected
    /// plan.
    replayed: RefCell<Option<(RunName, Projected)>>,
}

/// A person's monthly benefit at 62, full retirement age and 70.
pub(crate) type Estimate = [Option<Dollars>; 3];

/// An issue beside where it is in the plan, in the forms' words.
#[derive(Serialize, PartialEq, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PlacedIssue {
    /// The plan path it is about, such as `accounts[2].locked_until`.
    pub path: String,
    /// What is wrong, as the engine says it.
    pub message: String,
    /// The domain, the item by its display name and the field's label,
    /// then the message; the engine's own words where no domain holds it.
    pub words: String,
    /// Where a form edits what it is about; `null` where none does.
    pub place: Option<Place>,
}

/// Where in the plan's pages an issue is.
#[derive(Serialize, PartialEq, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Place {
    /// The domain's page address.
    pub domain: String,
    /// The item, where the domain holds many.
    pub index: Option<usize>,
    /// The key of the field, where a form has one for it.
    pub field: Option<&'static str>,
}

fn place_of(path: &str) -> Option<Place> {
    let (domain, index) = issue_place(path)?;
    Some(Place {
        domain: slug_of(domain),
        index,
        field: issue_field(path),
    })
}

/// A year as every surface says it: what to do, what to watch, and how old
/// everyone is.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SaidYear {
    /// The year.
    pub year: i16,
    /// Each person, by display name, and the age they reach in it.
    pub ages: Vec<(String, u8)>,
    /// Each action as a sentence, its amount nominal.
    pub actions: Vec<String>,
    /// What to watch for in the year.
    pub warnings: Vec<String>,
}

impl Document {
    /// Opens `path`, following a scenario's base chain through `read`; a
    /// base is found beside the file that names it.
    ///
    /// # Errors
    ///
    /// Where a file cannot be read, or a document does not parse or resolve.
    pub fn open(
        path: &str,
        read: &mut dyn FnMut(&Path) -> Result<String, String>,
    ) -> Result<Self, String> {
        let mut files = Vec::new();
        let canonical = |path: &Path| Ok(normal(path));
        let plan = resolve_with_files(normal(Path::new(path)), read, &canonical, &mut files)?;
        let draft = Draft::validated(plan, tables(), files.len() > 1);
        let is_valid = draft.issues().is_empty();
        let projected = is_valid.then(|| Projected::new(draft.plan.clone(), tables()));
        Ok(Self {
            draft,
            files,
            projected,
            estimates: OnceCell::new(),
            replayed: RefCell::default(),
        })
    }

    /// Whether it is a scenario, which cannot be written back as it stands.
    #[must_use]
    pub const fn is_read_only(&self) -> bool {
        self.draft.is_read_only()
    }

    /// The draft every edit lands in.
    #[must_use]
    pub const fn draft(&self) -> &Draft {
        &self.draft
    }

    /// Every file the resolution read, the document's own first.
    #[must_use]
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// What the full gate found wrong, each where it is.
    #[must_use]
    pub fn issues(&self) -> Vec<PlacedIssue> {
        let issues = self.draft.issues().iter();
        issues
            .map(|issue| PlacedIssue {
                path: issue.path.clone(),
                message: issue.message.clone(),
                words: issue_words(issue, &self.draft),
                place: place_of(&issue.path),
            })
            .collect()
    }

    /// Stores the item `editor` holds into the draft as one step of its
    /// history, answering where it now sits, or `None` where it held no
    /// edits to store.
    ///
    /// # Errors
    ///
    /// Why nothing was stored, in the form's words.
    pub fn apply(&mut self, editor: &mut Editor) -> Result<Option<usize>, String> {
        let stored = editor.edit.apply(&mut self.draft, None)?;
        if stored.is_some() {
            self.commit();
        }
        Ok(stored)
    }

    /// Removes item `index` of `form`'s domain as one step of history,
    /// where it is still the item called `name`.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only, the domain is a single item, or the
    /// item at `index` is no longer the one named.
    pub fn remove(&mut self, form: &Form, index: usize, name: &str) -> Result<(), String> {
        let list = self.still_at(form, index, name)?;
        self.step(|plan| {
            (list.remove)(plan, index);
            Ok(())
        })
    }

    /// The list `form` edits, where item `index` of it is still the one
    /// called `name`.
    fn still_at(&self, form: &Form, index: usize, name: &str) -> Result<ListOps, String> {
        let list = list_of(form)?;
        if name_at(&self.draft, form, list, index).as_deref() != Some(name) {
            return Err(format!("{name} is no longer where it was in the plan"));
        }
        Ok(list)
    }

    /// Records the statement `xml` on the person at `index`, where they are
    /// still the one called `name`, as one step of history, answering what
    /// was recorded.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only, the person there is no longer them, or
    /// the statement does not parse or is not theirs.
    pub fn import_earnings(
        &mut self,
        index: usize,
        name: &str,
        xml: &str,
    ) -> Result<String, String> {
        self.person_step(index, name, |plan, person| {
            let statement = statement::record(plan, person, xml)?;
            Ok(recorded(plan.person_name(person), &statement))
        })
    }

    /// Makes `change` to the person at `index`, where they are still the
    /// one called `name`, as one step of history, answering what it
    /// answers.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only, the person there is no longer them, or
    /// `change` fails.
    pub(crate) fn person_step(
        &mut self,
        index: usize,
        name: &str,
        change: impl FnOnce(&mut Plan, &str) -> Result<String, String>,
    ) -> Result<String, String> {
        self.still_at(form_at(&slug_of(DomainId::People))?, index, name)?;
        let person = self.draft.plan.household.people[index].id.clone();
        self.step(|plan| change(plan, &person))
    }

    /// Changes the plan by `change` as one step of history, answering what
    /// it answers; a change that fails is no step.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only, or `change` fails.
    pub(crate) fn step<T>(
        &mut self,
        change: impl FnOnce(&mut Plan) -> Result<T, String>,
    ) -> Result<T, String> {
        if let Some(reason) = self.draft.refuse_if_read_only() {
            return Err(reason);
        }
        let answer = change(&mut self.draft.plan)?;
        self.commit();
        Ok(answer)
    }

    /// Holds `answers` as tool `T`'s, beside the plan and outside its
    /// history.
    pub(crate) fn hold_answers<T: ToolAnswers>(&mut self, answers: toml::Table) {
        self.draft.set_answers::<T>(answers);
    }

    /// Stores what the tool form `editor` holds as that tool's answers,
    /// outside the plan's history.
    ///
    /// # Errors
    ///
    /// Why they were not stored, in the form's words.
    pub(crate) fn hold(&mut self, editor: &mut Editor) -> Result<(), String> {
        editor.edit.apply(&mut self.draft, None).map(drop)
    }

    fn commit(&mut self) {
        self.draft.record();
        self.revalidate();
    }

    /// A valid draft is projected at once; an invalid one keeps the last
    /// good projection.
    fn revalidate(&mut self) {
        if self.draft.revalidate(tables()) {
            self.projected = Some(Projected::new(self.draft.plan.clone(), tables()));
            self.estimates = OnceCell::new();
            self.replayed = RefCell::default();
        }
    }

    /// The person `id`'s estimates from the last plan without issues, made
    /// once for every person the first time any is asked for.
    pub(crate) fn estimate(&self, id: &str) -> Estimate {
        let Some(projected) = &self.projected else {
            return [None; 3];
        };
        let estimates = self.estimates.get_or_init(|| {
            let people = projected.plan.household.people.iter();
            people
                .map(|person| {
                    let estimate = benefit_estimates(&projected.plan, tables(), &person.id);
                    (person.id.clone(), estimate)
                })
                .collect()
        });
        let found = estimates.iter().find(|(each, _)| each == id);
        found.map_or([None; 3], |(_, estimate)| *estimate)
    }

    /// Aims the conversion constraints at `destination`, beside the plan
    /// and outside its history.
    pub(crate) fn aim_at(&mut self, destination: &str) {
        aim_at(&mut self.draft, destination);
    }

    /// Steps back over the last edit, answering whether there was one.
    pub fn undo(&mut self) -> bool {
        let is_undone = self.draft.undo();
        if is_undone {
            self.revalidate();
        }
        is_undone
    }

    /// Steps forward over the last undone edit, answering whether there
    /// was one.
    pub fn redo(&mut self) -> bool {
        let is_redone = self.draft.redo();
        if is_redone {
            self.revalidate();
        }
        is_redone
    }

    /// Writes the draft back where it came from through `write`.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only, has issues, or `write` fails.
    pub fn save(
        &mut self,
        write: &mut dyn FnMut(&str) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(reason) = self.draft.refuse_if_read_only() {
            return Err(reason);
        }
        self.write_through(write)?;
        self.draft.saved();
        Ok(())
    }

    /// Writes the draft as a plan of its own at `path` through `write`,
    /// which it then is.
    ///
    /// # Errors
    ///
    /// Where the draft has issues, `path` is the scenario it was resolved
    /// from, which the plan would replace, or `write` fails.
    pub fn save_as(
        &mut self,
        path: &str,
        write: &mut dyn FnMut(&str) -> Result<(), String>,
    ) -> Result<(), String> {
        let path = normal(Path::new(path));
        if self.is_read_only() && self.files.first() == Some(&path) {
            return Err(OVER_SCENARIO.to_owned());
        }
        self.write_through(write)?;
        self.draft.saved_as();
        self.files = vec![path];
        Ok(())
    }

    fn write_through(
        &self,
        write: &mut dyn FnMut(&str) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(reason) = self.draft.refuse_if_invalid() {
            return Err(reason);
        }
        write(&self.plan_text()?)
    }

    /// The projection, where the plan passed the gate.
    #[must_use]
    pub fn projection(&self) -> Option<&Projection> {
        Some(&self.projected.as_ref()?.projection)
    }

    /// The plan of the last valid draft beside its projection.
    pub(crate) fn projected(&self) -> Result<&Projected, String> {
        self.projected
            .as_ref()
            .ok_or_else(|| issue_listing(self.draft.issues()))
    }

    /// What `read` makes of the projected plan in `market`, the one it
    /// states where that is `None`; a market is replayed once, then kept
    /// until another is asked for or the draft changes.
    pub(crate) fn in_market<T>(
        &self,
        market: Option<&str>,
        read: impl FnOnce(&Projected) -> Result<T, String>,
    ) -> Result<T, String> {
        let own = self.projected()?;
        let Some(market) = market else {
            return read(own);
        };
        let name = market_named(market)?;
        let mut kept = self.replayed.borrow_mut();
        if kept.as_ref().is_none_or(|(held, _)| *held != name) {
            *kept = Some((name, replayed(own, name)?));
        }
        let (_, projected) = kept.as_ref().expect("replayed above");
        read(projected)
    }

    /// The headline figures, in today's dollars where `deflated`.
    #[must_use]
    pub fn summary(&self, deflated: bool) -> Option<Summary> {
        Some(self.projection()?.summary(deflated))
    }

    /// The year a view shows: `requested`, or `today` where none, held
    /// within the plan's years; `None` while the plan has issues.
    #[must_use]
    pub fn year_at(&self, requested: Option<i16>, today: i16) -> Option<i16> {
        let years = span(&self.projection()?.years);
        Some(YearCursor(requested).resolve(Today(today), years, years))
    }

    /// `year`'s recorded actions and warnings.
    ///
    /// # Errors
    ///
    /// Where the plan has issues, or `year` is outside its projection.
    pub fn actions(&self, year: i16) -> Result<ActionsReply, String> {
        let row = self.row(year)?;
        let plan = &self.projected()?.plan;
        let warnings = collect_warnings(plan, tables(), row, None);
        Ok(ActionsReply::new(row, warnings))
    }

    /// `year` in words.
    ///
    /// # Errors
    ///
    /// Where the plan has issues, or `year` is outside its projection.
    pub fn said(&self, year: i16) -> Result<SaidYear, String> {
        let row = self.row(year)?;
        let plan = &self.projected()?.plan;
        let people = plan.household.people.iter();
        let ages = people.filter_map(|person| {
            let age = *row.ages.get(&person.id)?;
            Some((person.display_name().to_owned(), age))
        });
        let actions = row.actions.iter();
        Ok(SaidYear {
            year,
            ages: ages.collect(),
            actions: actions.map(|action| sentence(plan, action)).collect(),
            warnings: collect_warnings(plan, tables(), row, None),
        })
    }

    fn row(&self, year: i16) -> Result<&YearRow, String> {
        year_row(&self.projected()?.projection, year)
    }

    /// The resolved plan as canonical TOML.
    ///
    /// # Errors
    ///
    /// Where the plan does not serialize.
    pub fn plan_text(&self) -> Result<String, String> {
        let plan = &self.draft.plan;
        plan.to_toml_string().map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use retiretui_client::setup::EXAMPLES;
    use retiretui_engine::plan::Plan;

    use super::*;

    fn reader(files: &[(&str, &str)]) -> impl FnMut(&Path) -> Result<String, String> + use<> {
        let files: BTreeMap<PathBuf, String> = files
            .iter()
            .map(|(path, text)| (PathBuf::from(path), (*text).to_owned()))
            .collect();
        move |path| {
            files
                .get(path)
                .cloned()
                .ok_or_else(|| "no such file".to_owned())
        }
    }

    fn starter() -> &'static str {
        EXAMPLES[0].2
    }

    #[test]
    fn a_plan_opens_projected() {
        let document = Document::open(
            "plans/../plan.toml",
            &mut reader(&[("/plan.toml", starter())]),
        )
        .expect("opens");
        assert!(document.issues().is_empty());
        assert!(!document.is_read_only());
        assert_eq!(document.files(), [PathBuf::from("/plan.toml")]);
        let projection = document.projection().expect("projected");
        let first = projection.years[0].year;
        assert!(document.summary(true).is_some());
        assert_eq!(document.actions(first).expect("in range").year, first);
        assert_eq!(document.year_at(None, first - 5), Some(first));
        assert_eq!(document.year_at(None, first + 1), Some(first + 1));
        let last = projection.years.last().expect("a year").year;
        assert_eq!(document.year_at(Some(first + 2), first), Some(first + 2));
        assert_eq!(document.year_at(Some(first - 9), first + 3), Some(first));
        assert_eq!(document.year_at(Some(last + 9), first), Some(last));
        let said = document.said(first).expect("in range");
        assert_eq!(said.ages, [("Sam".to_owned(), 30)]);
        assert!(
            said.actions
                .iter()
                .any(|action| action.contains("Sam's Roth IRA"))
        );
        let reopened = Plan::from_toml_str(&document.plan_text().expect("serializes"));
        assert_eq!(reopened.expect("parses"), document.draft.plan);
    }

    #[test]
    fn a_scenario_resolves_beside_its_file_and_is_read_only() {
        let scenario = "schema = 1\nbase = \"../base.toml\"\n";
        let files = [("/base.toml", starter()), ("/what-if/early.toml", scenario)];
        let document = Document::open("/what-if/early.toml", &mut reader(&files)).expect("opens");
        assert!(document.is_read_only());
        let read = [
            PathBuf::from("/what-if/early.toml"),
            PathBuf::from("/base.toml"),
        ];
        assert_eq!(document.files(), read);
    }

    #[test]
    fn a_missing_file_is_refused_by_name() {
        let error = Document::open("/gone.toml", &mut reader(&[])).expect_err("refused");
        assert_eq!(error, "failed to read /gone.toml: no such file");
    }

    #[test]
    fn an_invalid_plan_opens_with_issues_and_no_projection() {
        let mut plan = Plan::from_toml_str(starter()).expect("parses");
        plan.plan.inflation = 5.0;
        let text = plan.to_toml_string().expect("serializes");
        let document =
            Document::open("/plan.toml", &mut reader(&[("/plan.toml", &text)])).expect("opens");
        let issues = document.issues();
        let issue = issues.first().expect("an issue");
        assert!(issue.words.starts_with("Settings"), "{}", issue.words);
        assert!(issue.words.ends_with(&issue.message));
        let place = issue.place.as_ref().expect("a place");
        assert_eq!((place.domain.as_str(), place.index), ("settings", None));
        assert_eq!(place.field, Some("inflation"));
        assert!(document.projection().is_none());
        assert_eq!(document.year_at(None, 2030), None);
        assert!(document.summary(false).is_none());
        assert!(document.actions(plan.plan.start_year).is_err());
    }
}
