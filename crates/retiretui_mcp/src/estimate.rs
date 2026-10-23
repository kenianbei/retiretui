use std::path::Path;

use retiretui_client::environment::load_patterns;
use retiretui_client::estimate::{EstimateReply, Exports, RecordsReply};
use retiretui_client::session::file_name;
use retiretui_engine::estimate::{Category, Of, Patterns};
use retiretui_engine::plan::Plan;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

use super::PlanServer;
use super::optimize::WithOverlay;

/// What the rules passed inline are named in a record's `by`.
const INLINE_PATTERNS: &str = "patterns";

/// One of a bank's exports: its text, or a file in the served directory.
#[derive(Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Export {
    /// The export's text.
    Text {
        /// The file's name, whose extension says what it is: `.csv`, `.ofx`
        /// or `.qfx`.
        name: String,
        /// The file's text.
        text: String,
    },
    /// An export file in the served directory.
    Path {
        /// Its path, relative to the served directory.
        path: String,
    },
}

/// What the estimate's tools read.
#[derive(Deserialize, JsonSchema)]
pub struct EstimateInputs {
    /// Plan or scenario path, relative to the served directory.
    pub path: String,
    /// The exports, each read once; a row a file already read answers for,
    /// by the same bank id, else the same date, amount and payee, is
    /// dropped.
    pub files: Vec<Export>,
    /// Rules as TOML: `[[patterns]]` rows testing the payee by `match`
    /// (contains; a space at either end marks the edge of a word), `is`,
    /// `one_of` or `matches` (a regex), and optionally `spent = [low,
    /// high]` in statement dollars, `export` (part of the file name) and
    /// `on` (a date), each with a `category`, `ignore = true` or `bucket =
    /// true`, the most specific match winning and every rule before the
    /// bank's own category; and `[[payees]]` rows merging payees under a
    /// `name` by the same payee keys. Read after `patterns_path`, winning a
    /// tie with it.
    pub patterns: Option<String>,
    /// A rules file in the served directory, read after the user's own.
    pub patterns_path: Option<String>,
    /// Whether the user's own patterns file, in their config directory, is
    /// read first; true when left out.
    pub user_patterns: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct EstimateToolArgs {
    #[serde(flatten)]
    pub inputs: EstimateInputs,
    /// Store the proposals applied as a scenario overlay at this path
    /// (`.toml`, relative to the served directory); its `base` points back
    /// at `path`.
    pub write_to: Option<String>,
}

/// Which records to list.
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Which {
    /// Those sorted into a category, by its slug.
    Category(Category),
    /// Those of every payee containing the words, either case, by the name
    /// the rules list it under or as the bank wrote it, wherever they were
    /// sorted.
    Payee(String),
    /// Those of the payees nothing names, a bucket rule's among them.
    Unnamed,
    /// Those the rules ignore.
    Ignored,
}

#[derive(Deserialize, JsonSchema)]
pub struct RecordsToolArgs {
    #[serde(flatten)]
    pub inputs: EstimateInputs,
    /// Which records to list.
    pub which: Which,
}

#[tool_router(router = estimate_router, vis = "pub(super)")]
impl PlanServer {
    /// Estimate the plan's expenses from a bank's exports. Each row is
    /// sorted into a category, ignored or sent to the bucket by the user's
    /// patterns file and the rules passed, else by the bank's own category
    /// column or transaction type, else by its payee's words; the latest
    /// twelve whole months are annualized in the plan's start-year dollars,
    /// each month deflated by its year; and each spending category is
    /// proposed as the amount of the expense whose id or name is for it, or
    /// as a new expense. Income and transfers are counted, never proposed;
    /// a payee nothing names is listed. The proposals applied are returned
    /// as a scenario overlay document and, with `write_to`, stored through
    /// the validated write gate.
    #[tool]
    fn estimate_spending(
        &self,
        Parameters(args): Parameters<EstimateToolArgs>,
    ) -> Result<Json<WithOverlay<EstimateReply>>, String> {
        let (plan, exports, patterns) = self.read_inputs(&args.inputs)?;
        let found = exports.estimate(&plan, &patterns);
        let (scenario_toml, written) =
            self.emit_overlay(&args.inputs.path, args.write_to.as_deref(), |base| {
                retiretui_client::estimate::overlay(base, &plan, &found)
            })?;
        Ok(Json(WithOverlay {
            reply: EstimateReply {
                estimate: found,
                written: written.then(|| args.write_to.clone()).flatten(),
            },
            scenario_toml,
            written,
        }))
    }

    /// List the records behind the spending estimate - a category's, those
    /// of payees containing some words, the unnamed or the ignored - within
    /// the estimate's twelve months, newest first, read as
    /// `estimate_spending` reads them. Each says where it was sorted and
    /// what decided it: a rule by its file and place there, the bank's
    /// category, the transaction type, or the embedded table's words.
    #[tool]
    fn estimate_records(
        &self,
        Parameters(args): Parameters<RecordsToolArgs>,
    ) -> Result<Json<RecordsReply>, String> {
        let (plan, exports, patterns) = self.read_inputs(&args.inputs)?;
        let of = match args.which {
            Which::Category(category) => Of::Category(category),
            Which::Payee(words) => Of::Containing(words),
            Which::Unnamed => Of::Unnamed,
            Which::Ignored => Of::Ignored,
        };
        Ok(Json(exports.records(&plan, &patterns, &of)))
    }
}

impl PlanServer {
    /// The plan, the exports and the patterns `inputs` name.
    fn read_inputs(&self, inputs: &EstimateInputs) -> Result<(Plan, Exports, Patterns), String> {
        let plan = self.load_valid_plan(&inputs.path)?;
        let is_user_read = inputs.user_patterns.unwrap_or(true);
        let mut patterns =
            load_patterns(None, is_user_read).map_err(|error| format!("{error:#}"))?;
        if let Some(path) = &inputs.patterns_path {
            let text = self.store.read(path)?;
            (patterns.add(path, &text)).map_err(|error| format!("{path}: {error}"))?;
        }
        if let Some(text) = &inputs.patterns {
            (patterns.add(INLINE_PATTERNS, text)).map_err(|error| format!("patterns: {error}"))?;
        }
        let mut exports = Exports::default();
        for file in &inputs.files {
            let added = match file {
                Export::Text { name, text } => exports.add(name, text),
                Export::Path { path } => {
                    exports.add(&file_name(Path::new(path)), &self.store.read(path)?)
                }
            };
            added.map_err(|error| error.to_string())?;
        }
        Ok((plan, exports, patterns))
    }
}
