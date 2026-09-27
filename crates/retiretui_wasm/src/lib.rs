//! The retirement planner's engine and client for JavaScript: a document
//! opened through the page's own reads, and the searches a worker runs over
//! a plan's text. Values cross as plain objects, typed by `bindings/`.

mod document;
mod searches;

use std::path::Path;
use std::sync::OnceLock;

use js_sys::Function;
use retiretui_engine::params::TaxTables;
use serde::Serialize;
use serde_wasm_bindgen::Serializer;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use document::Document;

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

fn refused(message: String) -> JsError {
    JsError::new(&message)
}

fn reply<T: Serialize>(answer: Result<T, String>) -> Result<JsValue, JsError> {
    to_js(&answer.map_err(refused)?)
}

#[wasm_bindgen(typescript_custom_section)]
const TYPES: &str = r#"import type {
  ActionsReply, ClaimsReply, Example, HistoricalReply, Issue, MonteCarloReply,
  Projection, Summary, SweepReply,
} from "../bindings/index";"#;

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
    /// Where a file cannot be read, or a document does not parse or resolve.
    pub fn open(
        path: &str,
        #[wasm_bindgen(unchecked_param_type = "(path: string) => string")] read: &Function,
    ) -> Result<JsDocument, JsError> {
        let mut read = |file: &Path| read_through(read, file);
        Document::open(path, &mut read).map(Self).map_err(refused)
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

    /// What the full gate found wrong.
    ///
    /// # Errors
    ///
    /// Where the issues do not convert.
    #[wasm_bindgen(unchecked_return_type = "Issue[]")]
    pub fn issues(&self) -> Result<JsValue, JsError> {
        to_js(&self.0.issues())
    }

    /// The projection, `null` while the plan has issues. Each call converts
    /// it anew, so a caller keeps what it is given.
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
    let text = read.call1(&JsValue::NULL, &path).map_err(|thrown| {
        let error = thrown
            .dyn_ref::<js_sys::Error>()
            .map(js_sys::Error::message);
        error.map_or_else(|| format!("{thrown:?}"), String::from)
    })?;
    text.as_string()
        .ok_or_else(|| "the read returned no text".to_owned())
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

/// What the full gate finds wrong with `plan`, a plan's TOML.
///
/// # Errors
///
/// Where `plan` is not a plan.
#[wasm_bindgen(unchecked_return_type = "Issue[]")]
pub fn validate(plan: &str) -> Result<JsValue, JsError> {
    reply(searches::validate(plan))
}

/// Every bracket's conversion ladder into the `destination` account, best
/// first.
///
/// # Errors
///
/// Where the plan does not pass the gate, or the search refuses it.
#[wasm_bindgen(js_name = sweepBrackets, unchecked_return_type = "SweepReply")]
pub fn sweep_brackets(plan: &str, destination: &str, deflated: bool) -> Result<JsValue, JsError> {
    reply(searches::sweep(plan, destination, deflated))
}

/// Every claim age for the household's computed benefits, best first.
///
/// # Errors
///
/// Where the plan does not pass the gate, or the search refuses it.
#[wasm_bindgen(js_name = optimizeClaims, unchecked_return_type = "ClaimsReply")]
pub fn optimize_claims(plan: &str, deflated: bool) -> Result<JsValue, JsError> {
    reply(searches::claims(plan, deflated))
}

/// The plan through the random markets its settings draw.
///
/// # Errors
///
/// Where the plan does not pass the gate, or cannot be run.
#[wasm_bindgen(js_name = monteCarlo, unchecked_return_type = "MonteCarloReply")]
pub fn monte_carlo(plan: &str) -> Result<JsValue, JsError> {
    reply(searches::monte_carlo(plan))
}

/// The plan from every historical start year.
///
/// # Errors
///
/// Where the plan does not pass the gate, or cannot be run.
#[wasm_bindgen(unchecked_return_type = "HistoricalReply")]
pub fn historical(plan: &str) -> Result<JsValue, JsError> {
    reply(searches::historical(plan))
}

#[cfg(all(test, feature = "ts"))]
mod bindings {
    use std::fmt::Write;
    use std::fs;

    use retiretui_client::replies::{
        ActionsReply, ClaimsReply, HistoricalReply, MonteCarloReply, SweepReply,
    };
    use retiretui_engine::plan::Issue;
    use retiretui_engine::project::{Projection, Summary};
    use ts_rs::{Config, TS};

    use crate::searches::Example;

    const BINDINGS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/bindings");

    #[test]
    fn export_bindings() {
        let _ = fs::remove_dir_all(BINDINGS);
        let config = Config::new()
            .with_large_int("number")
            .with_out_dir(BINDINGS);
        let exports = [
            Issue::export_all,
            Projection::export_all,
            Summary::export_all,
            ActionsReply::export_all,
            SweepReply::export_all,
            ClaimsReply::export_all,
            MonteCarloReply::export_all,
            HistoricalReply::export_all,
            Example::export_all,
        ];
        for export in exports {
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
