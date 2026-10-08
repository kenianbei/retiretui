//! The retirement planner's engine and client for JavaScript: a document
//! opened through the page's own reads, and the searches a worker runs over
//! a plan's text. Values cross as plain objects, typed by `bindings/`.

mod claims;
mod compare;
mod document;
mod domain;
mod edits;
mod ladders;
mod ledger;
mod markets;
mod orders;
mod overview;
mod searches;
mod setup;
mod spending;
mod tax_tables;
mod unopened;
mod view;
mod vocabulary;

use std::path::Path;
use std::sync::OnceLock;

use js_sys::Function;
use retiretui_client::present::MoneyForm;
use retiretui_engine::params::TaxTables;
use retiretui_engine::project::{Projection, Summary};
use serde::Serialize;
use serde_wasm_bindgen::Serializer;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use document::Document;
use unopened::OpenFailure;

/// The embedded tax tables; the page has no directories of its own.
fn tables() -> &'static TaxTables {
    static TABLES: OnceLock<TaxTables> = OnceLock::new();
    TABLES.get_or_init(TaxTables::embedded)
}

const SERIALIZER: Serializer = Serializer::json_compatible();

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    value
        .serialize(&SERIALIZER)
        .map_err(|error| JsError::new(&error.to_string()))
}

fn from_js<T: serde::de::DeserializeOwned>(value: JsValue) -> Result<T, JsError> {
    serde_wasm_bindgen::from_value(value).map_err(|error| JsError::new(&error.to_string()))
}

/// A value in either dollar basis.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Bases<T> {
    /// In today's dollars.
    pub today: T,
    /// In the dollars of each year.
    pub nominal: T,
}

impl<T> Bases<T> {
    /// What `of` makes in each basis, given whether it is nominal.
    fn of(of: impl Fn(bool) -> T) -> Self {
        Self {
            today: of(false),
            nominal: of(true),
        }
    }
}

/// A search option's cells in each basis, in full: `cells` of
/// `projection`, against the plan's `summaries` where it is an option.
fn option_figures(
    cells: fn(&Summary, Option<&Summary>, MoneyForm) -> Vec<String>,
    projection: &Projection,
    summaries: Option<&Bases<Summary>>,
) -> Bases<Vec<String>> {
    Bases::of(|nominal| {
        let plan = summaries.map(|plan| if nominal { &plan.nominal } else { &plan.today });
        cells(&projection.summary(!nominal), plan, MoneyForm::Full)
    })
}

fn refused(message: String) -> JsError {
    JsError::new(&message)
}

fn reply<T: Serialize>(answer: Result<T, String>) -> Result<JsValue, JsError> {
    to_js(&answer.map_err(refused)?)
}

#[wasm_bindgen(typescript_custom_section)]
const TYPES: &str = r#"import type {
  ActionsReply, ChartSeries, Claim, ClaimWords, ClaimsOptions, CompareView, CompareWords,
  Domain, DomainTable,
  Example, FieldView, Issue, LadderWords, LadderYear, LaddersReply,
  Ledger, MarketRuns, MarketWords, Metric, OrderOptions, OrderWords, OverviewView, PersonAction,
  PersonRow, PlacedIssue,
  NewPlanMade, OpenFailure,
  Projection, ReadRow, RothOwner, ScaledExpense, Searched, Sort, SpendingOptions,
  SpendingWords, Step, Summary, TablesView,
  TreatmentClass, ViewWords, Year, YearFigure, YearTables,
} from "../bindings/index";
export type * from "../bindings/index";"#;

/// A plan file opened through the page's reads.
#[wasm_bindgen(js_name = Document)]
#[derive(Debug)]
pub struct JsDocument(Document);

#[wasm_bindgen(js_class = Document)]
impl JsDocument {
    /// Opens `path`, reading it and any scenario base through `read`.
    ///
    /// # Errors
    ///
    /// An `OpenFailure` where a file cannot be read, or a document does
    /// not parse or resolve.
    pub fn open(
        path: &str,
        #[wasm_bindgen(unchecked_param_type = "(path: string) => string")] read: &Function,
    ) -> Result<JsDocument, JsValue> {
        let mut read = |file: &Path| read_through(read, file);
        Document::open(path, &mut read)
            .map(Self)
            .map_err(|error| to_js(&OpenFailure::from(&error)).unwrap_or_else(JsValue::from))
    }

    /// Takes `to` wherever `from` is among the files the document was
    /// resolved from, nothing read again or written: the file was renamed,
    /// and holds what it held.
    pub fn relocate(&mut self, from: &str, to: &str) {
        self.0.relocate(from, to);
    }

    /// Whether it is a scenario, which cannot be written back as it stands.
    #[wasm_bindgen(getter, js_name = isReadOnly)]
    #[must_use]
    pub fn is_read_only(&self) -> bool {
        self.0.is_read_only()
    }

    /// Every file the resolution read, the document's own first.
    #[must_use]
    pub fn files(&self) -> Vec<String> {
        let files = self.0.files().iter();
        files
            .map(|file| file.to_string_lossy().into_owned())
            .collect()
    }

    /// What the full gate found wrong, each where it is.
    ///
    /// # Errors
    ///
    /// Where the issues do not convert.
    #[wasm_bindgen(unchecked_return_type = "PlacedIssue[]")]
    pub fn issues(&self) -> Result<JsValue, JsError> {
        to_js(&self.0.issues())
    }

    /// The projection of the draft, or while it has issues of the last
    /// draft that had none; `null` where there never was one. Each call
    /// converts it anew, so a caller keeps what it is given.
    ///
    /// # Errors
    ///
    /// Where the projection does not convert.
    #[wasm_bindgen(unchecked_return_type = "Projection | null")]
    pub fn projection(&self) -> Result<JsValue, JsError> {
        to_js(&self.0.projection())
    }

    /// The headline figures, in today's dollars where `deflated`; `null`
    /// while the plan has issues.
    ///
    /// # Errors
    ///
    /// Where the summary does not convert.
    #[wasm_bindgen(unchecked_return_type = "Summary | null")]
    pub fn summary(&self, deflated: bool) -> Result<JsValue, JsError> {
        to_js(&self.0.summary(deflated))
    }

    /// The year a view shows: `requested`, or `today` where it is `null`,
    /// held within the plan's years; `undefined` while the plan has issues.
    #[wasm_bindgen(js_name = yearAt)]
    #[must_use]
    pub fn year_at(&self, requested: Option<i16>, today: i16) -> Option<i16> {
        self.0.year_at(requested, today)
    }

    /// `year`'s recorded actions and warnings.
    ///
    /// # Errors
    ///
    /// Where the plan has issues, or `year` is outside its projection.
    #[wasm_bindgen(unchecked_return_type = "ActionsReply")]
    pub fn actions(&self, year: i16) -> Result<JsValue, JsError> {
        reply(self.0.actions(year))
    }

    /// The resolved plan as canonical TOML: what a worker is handed, and
    /// what a save writes.
    ///
    /// # Errors
    ///
    /// Where the plan does not serialize.
    #[wasm_bindgen(js_name = planText)]
    pub fn plan_text(&self) -> Result<String, JsError> {
        self.0.plan_text().map_err(refused)
    }
}

fn read_through(read: &Function, file: &Path) -> Result<String, String> {
    let path = JsValue::from_str(&file.to_string_lossy());
    let text = read
        .call1(&JsValue::NULL, &path)
        .map_err(|thrown| thrown_message(&thrown))?;
    text.as_string()
        .ok_or_else(|| "the read returned no text".to_owned())
}

/// What a JavaScript function threw, as its message says it.
fn thrown_message(thrown: &JsValue) -> String {
    let error = thrown
        .dyn_ref::<js_sys::Error>()
        .map(js_sys::Error::message);
    error.map_or_else(|| format!("{thrown:?}"), String::from)
}

/// Every example plan.
///
/// # Errors
///
/// Where the examples do not convert.
#[wasm_bindgen(unchecked_return_type = "Example[]")]
pub fn examples() -> Result<JsValue, JsError> {
    to_js(&searches::examples())
}

/// Every editing domain, in the order the plan lists them.
///
/// # Errors
///
/// Where the domains do not convert.
#[wasm_bindgen(unchecked_return_type = "Domain[]")]
pub fn domains() -> Result<JsValue, JsError> {
    to_js(&vocabulary::domains())
}

/// The file the scenario `text`, kept at `path`, is resolved over: its
/// `base`, beside it; `undefined` for a plan, or text that reads as neither.
#[wasm_bindgen(js_name = baseOf)]
#[must_use]
pub fn base_of(path: &str, text: &str) -> Option<String> {
    retiretui_client::files::base_of(path, text)
}

/// `text`, a scenario, naming `base` in place of the base it names.
///
/// # Errors
///
/// Where `text` is not a scenario, or does not serialize again.
#[wasm_bindgen]
pub fn rebased(text: &str, base: &str) -> Result<String, JsError> {
    retiretui_client::files::rebased(text, base).map_err(refused)
}

/// How many issues there are, in words: `1 issue`, `3 issues`.
#[wasm_bindgen(js_name = issueCount)]
#[must_use]
pub fn issue_count(count: usize) -> String {
    retiretui_client::present::issue_count(count)
}

/// The version of the planner the bindings were built from.
#[wasm_bindgen]
#[must_use]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// What the full gate finds wrong with `plan`, a plan's TOML.
///
/// # Errors
///
/// Where `plan` is not a plan.
#[wasm_bindgen(unchecked_return_type = "Issue[]")]
pub fn validate(plan: &str) -> Result<JsValue, JsError> {
    reply(searches::validate(plan))
}

#[cfg(all(test, feature = "ts"))]
mod bindings {
    use std::fmt::Write;
    use std::fs;

    use retiretui_client::replies::ActionsReply;
    use retiretui_engine::plan::Issue;
    use retiretui_engine::project::{Projection, Summary};
    use ts_rs::{Config, ExportError, TS};

    use crate::claims::{ClaimWords, ClaimsOptions, PersonRow, RothOwner};
    use crate::document::PlacedIssue;
    use crate::unopened::{OpenFailure, Written};
    use retiretui_client::forms::details::ReadRow;
    use retiretui_client::forms::sort::Sort;

    use crate::compare::{CompareView, CompareWords, Searched, YearFigure};
    use crate::domain::DomainTable;
    use crate::ladders::{LadderWords, LaddersReply};
    use crate::ledger::{ChartSeries, Ledger, ViewWords};
    use crate::markets::{MarketRuns, MarketWords};
    use crate::orders::{OrderOptions, OrderWords};
    use crate::overview::OverviewView;
    use crate::searches::Example;
    use crate::setup::NewPlanMade;
    use crate::spending::{SpendingOptions, SpendingWords};
    use crate::view::FieldView;
    use crate::vocabulary::Domain;
    use retiretui_client::ledger::Year;
    use retiretui_client::setup::Step;
    use retiretui_client::tax_tables::{TablesView, YearTables};

    const BINDINGS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/bindings");

    /// Every type the page is typed by, with the types they name.
    const EXPORTS: &[fn(&Config) -> Result<(), ExportError>] = &[
        Issue::export_all,
        PlacedIssue::export_all,
        ReadRow::export_all,
        Projection::export_all,
        Summary::export_all,
        ActionsReply::export_all,
        LaddersReply::export_all,
        LadderWords::export_all,
        ClaimsOptions::export_all,
        ClaimWords::export_all,
        PersonRow::export_all,
        RothOwner::export_all,
        OrderOptions::export_all,
        OrderWords::export_all,
        SpendingOptions::export_all,
        SpendingWords::export_all,
        MarketRuns::export_all,
        MarketWords::export_all,
        CompareView::export_all,
        Searched::export_all,
        YearFigure::export_all,
        CompareWords::export_all,
        Example::export_all,
        Domain::export_all,
        DomainTable::export_all,
        Sort::export_all,
        FieldView::export_all,
        Ledger::export_all,
        Year::export_all,
        ChartSeries::export_all,
        ViewWords::export_all,
        OverviewView::export_all,
        TablesView::export_all,
        YearTables::export_all,
        Step::export_all,
        NewPlanMade::export_all,
        OpenFailure::export_all,
        Written::export_all,
    ];

    #[test]
    fn export_bindings() {
        let _ = fs::remove_dir_all(BINDINGS);
        let config = Config::new()
            .with_large_int("number")
            .with_out_dir(BINDINGS);
        for export in EXPORTS {
            export(&config).expect("exports");
        }
        let mut names: Vec<String> = fs::read_dir(BINDINGS)
            .expect("exported")
            .map(|entry| {
                entry
                    .expect("listed")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .filter_map(|file| Some(file.strip_suffix(".ts")?.to_owned()))
            .collect();
        names.sort();
        let index = names.iter().fold(String::new(), |mut index, name| {
            let _ = writeln!(index, "export type {{ {name} }} from \"./{name}\";");
            index
        });
        fs::write(format!("{BINDINGS}/index.ts"), index).expect("written");
    }
}
