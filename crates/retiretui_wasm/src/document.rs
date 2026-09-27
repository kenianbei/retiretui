//! A plan file opened through whatever reads the page's files: its resolved
//! plan, the files it came from, its issues, and, where it has none, its
//! projection.

use std::path::{Path, PathBuf};

use retiretui_client::actions::{collect_warnings, sentence};
use retiretui_client::draft::Draft;
use retiretui_client::files::resolve_with_files;
use retiretui_client::issues::{issue_listing, issue_words};
use retiretui_client::replies::{ActionsReply, year_row};
use retiretui_client::session::{Today, YearCursor, span};
use retiretui_client::store::normal;
use retiretui_engine::plan::Item;
use retiretui_engine::project::{Projection, Summary, YearRow, project};
use serde::Serialize;

use crate::tables;

/// A resolved plan and what the gate made of it.
#[derive(Debug)]
pub(crate) struct Document {
    draft: Draft,
    files: Vec<PathBuf>,
    projection: Option<Projection>,
}

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
        let draft = Draft::validated(plan, tables());
        let is_valid = draft.issues().is_empty();
        let projection = is_valid.then(|| project(&draft.plan, tables()));
        Ok(Self {
            draft,
            files,
            projection,
        })
    }

    /// Whether it is a scenario, which cannot be written back as it stands.
    #[must_use]
    pub fn is_read_only(&self) -> bool {
        self.files.len() > 1
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
            })
            .collect()
    }

    /// The projection, where the plan passed the gate.
    #[must_use]
    pub fn projection(&self) -> Option<&Projection> {
        self.projection.as_ref()
    }

    /// The headline figures, in today's dollars where `deflated`.
    #[must_use]
    pub fn summary(&self, deflated: bool) -> Option<Summary> {
        Some(self.projection()?.summary(deflated))
    }

    /// The year every view starts on: `today`, held within the plan's
    /// years; `None` while the plan has issues.
    #[must_use]
    pub fn this_year(&self, today: i16) -> Option<i16> {
        let years = span(&self.projection()?.years);
        Some(YearCursor::default().resolve(Today(today), years, years))
    }

    /// `year`'s recorded actions and warnings.
    ///
    /// # Errors
    ///
    /// Where the plan has issues, or `year` is outside its projection.
    pub fn actions(&self, year: i16) -> Result<ActionsReply, String> {
        let row = self.row(year)?;
        let warnings = collect_warnings(&self.draft.plan, tables(), row, None);
        Ok(ActionsReply::new(row, warnings))
    }

    /// `year` in words.
    ///
    /// # Errors
    ///
    /// Where the plan has issues, or `year` is outside its projection.
    pub fn said(&self, year: i16) -> Result<SaidYear, String> {
        let row = self.row(year)?;
        let plan = &self.draft.plan;
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
        let projection = self
            .projection()
            .ok_or_else(|| issue_listing(self.draft.issues()))?;
        year_row(projection, year)
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
        assert_eq!(document.this_year(first - 5), Some(first));
        assert_eq!(document.this_year(first + 1), Some(first + 1));
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
        assert!(document.projection().is_none());
        assert_eq!(document.this_year(2030), None);
        assert!(document.summary(false).is_none());
        assert!(document.actions(plan.plan.start_year).is_err());
    }
}
