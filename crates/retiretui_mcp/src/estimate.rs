use retiretui_client::estimate::{EstimateReply, Exports};
use retiretui_engine::estimate::Patterns;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

use super::PlanServer;
use super::optimize::WithOverlay;

/// One of a bank's exports, as its text.
#[derive(Deserialize, JsonSchema)]
pub struct Export {
    /// The file's name, whose extension says what it is: `.csv`, `.ofx` or
    /// `.qfx`.
    pub name: String,
    /// The file's text.
    pub text: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct EstimateToolArgs {
    /// Plan or scenario path, relative to the served directory.
    pub path: String,
    /// The exports, each read once; a row a file already read answers for,
    /// by the same bank id, else the same date, amount and payee, is
    /// dropped.
    pub files: Vec<Export>,
    /// The user's rules as TOML: `[[patterns]]` rows testing the payee by
    /// `match` (contains), `is`, `one_of` or `matches` (a regex), and
    /// optionally `spent = [low, high]` in statement dollars, `export`
    /// (part of the file name) and `on` (a date), each with a `category`,
    /// `ignore = true` or `bucket = true`, the most specific match winning
    /// and every rule before the bank's own category; and `[[payees]]` rows
    /// merging payees under a `name` by the same payee keys.
    pub patterns: Option<String>,
    /// Store the proposals applied as a scenario overlay at this path
    /// (`.toml`, relative to the served directory); its `base` points back
    /// at `path`.
    pub write_to: Option<String>,
}

#[tool_router(router = estimate_router, vis = "pub(super)")]
impl PlanServer {
    /// Estimate the plan's expenses from a bank's exports. Each row is
    /// sorted into a category, ignored or sent to the bucket by the rules
    /// in `patterns`, else by the bank's own category column or transaction
    /// type, else by its payee's words; the latest twelve whole months are
    /// annualized in the plan's start-year dollars, each month deflated by
    /// its year; and each spending category is proposed as the amount of
    /// the expense whose id or name is for it, or as a new expense. Income
    /// and transfers are counted, never proposed; a payee nothing names is
    /// listed. The user's own patterns file is not read: pass its text as
    /// `patterns`. The proposals applied are returned as a scenario overlay
    /// document and, with `write_to`, stored through the validated write
    /// gate.
    #[tool]
    fn estimate_spending(
        &self,
        Parameters(args): Parameters<EstimateToolArgs>,
    ) -> Result<Json<WithOverlay<EstimateReply>>, String> {
        let plan = self.load_valid_plan(&args.path)?;
        let mut patterns = Patterns::embedded();
        if let Some(text) = &args.patterns {
            patterns
                .add(text)
                .map_err(|error| format!("patterns: {error}"))?;
        }
        let mut exports = Exports::default();
        for file in &args.files {
            exports
                .add(&file.name, &file.text)
                .map_err(|error| error.to_string())?;
        }
        let found = exports.estimate(&plan, &patterns);
        let (scenario_toml, written) =
            self.emit_overlay(&args.path, args.write_to.as_deref(), |base| {
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
}
