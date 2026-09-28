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
    let said = document.said(first, true).expect("in range");
    assert_eq!(said.ages, [("Sam".to_owned(), 30)]);
    assert!(
        said.actions
            .iter()
            .any(|action| action.contains("Sam's Roth IRA"))
    );
    let later = first + 10;
    let nominal = document.said(later, true).expect("in range").actions;
    let today = document.said(later, false).expect("in range").actions;
    assert_eq!(nominal.len(), today.len());
    assert_ne!(nominal, today, "a later year's amounts follow the basis");
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
