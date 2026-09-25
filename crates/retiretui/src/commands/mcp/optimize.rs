use std::path::Path;

use retiretui_engine::optimize::{
    OptimizeOptions, claims_overlay, ladder_overlay, optimize_claims, optimize_conversions,
    sweep_brackets,
};
use retiretui_engine::plan::PlanError;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::PlanServer;
use crate::commands::optimize::{ClaimsReply, LadderConstraints, LadderReply, SweepReply};

/// What the conversion tools take besides the ladder's constraints.
#[derive(Deserialize, JsonSchema)]
pub struct ConversionToolArgs {
    /// Plan or scenario path, relative to the served directory.
    pub path: String,
    /// Deferred source account ids, drained in the given order.
    pub from: Vec<String>,
    /// Roth destination account id; every source must share its owner.
    pub to: String,
    #[serde(flatten)]
    pub constraints: LadderConstraints,
    /// Report nominal dollars instead of today's dollars.
    #[serde(default)]
    pub nominal: bool,
}

impl ConversionToolArgs {
    fn options(&self) -> OptimizeOptions {
        self.constraints.options(&self.from, &self.to)
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct OptimizeToolArgs {
    #[serde(flatten)]
    pub conversion: ConversionToolArgs,
    /// Bracket to fill, as a percent (e.g. 22).
    pub bracket: f64,
    /// Store the ladder as a scenario overlay at this path (`.toml`,
    /// relative to the served directory); its `base` points back at `path`.
    pub write_to: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct ClaimToolArgs {
    /// Plan or scenario path, relative to the served directory.
    pub path: String,
    /// Ids of the social-security incomes to search; absent searches every
    /// income whose benefit is computed from an earnings record, and makes
    /// one up for each person with a record and none.
    #[serde(default)]
    pub incomes: Vec<String>,
    /// Report nominal dollars instead of today's dollars.
    #[serde(default)]
    pub nominal: bool,
    /// Store the best-ranked claims as a scenario overlay at this path
    /// (`.toml`, relative to the served directory); its `base` points back
    /// at `path`.
    pub write_to: Option<String>,
}

/// A search's reply with the overlay it emits.
#[derive(Serialize, JsonSchema)]
pub struct WithOverlay<T> {
    #[serde(flatten)]
    pub reply: T,
    /// The search's answer as a scenario overlay document.
    pub scenario_toml: String,
    /// Whether the overlay was stored at `write_to`.
    pub written: bool,
}

#[tool_router(router = optimize_router, vis = "pub(super)")]
impl PlanServer {
    /// Search a fill-bracket Roth conversion ladder for a plan and compare
    /// it to the baseline; use `sweep_conversion_brackets` first to pick
    /// the bracket. The ladder is returned as a scenario overlay document
    /// and, with `write_to`, stored through the validated write gate.
    #[tool]
    fn optimize_conversions(
        &self,
        Parameters(args): Parameters<OptimizeToolArgs>,
    ) -> Result<Json<WithOverlay<LadderReply>>, String> {
        let conversion = &args.conversion;
        let plan = self.load_valid_plan(&conversion.path)?;
        let options = conversion.options();
        let ladder = optimize_conversions(&plan, &self.tables, &options, args.bracket / 100.0)
            .map_err(|issues| crate::commands::issue_listing(&issues))?;
        let (scenario_toml, written) =
            self.emit_overlay(&conversion.path, args.write_to.as_deref(), |base| {
                ladder_overlay(base, &plan, &options, &ladder.ladder.steps)
            })?;
        Ok(Json(WithOverlay {
            reply: LadderReply::new(&ladder, !conversion.nominal),
            scenario_toml,
            written,
        }))
    }

    /// Run the conversion optimizer once per fillable tax bracket and
    /// report each outcome beside the baseline, so the bracket worth
    /// filling can be chosen before `optimize_conversions` emits it.
    #[tool]
    fn sweep_conversion_brackets(
        &self,
        Parameters(args): Parameters<ConversionToolArgs>,
    ) -> Result<Json<SweepReply>, String> {
        let plan = self.load_valid_plan(&args.path)?;
        let sweep = sweep_brackets(&plan, &self.tables, &args.options())
            .map_err(|issues| crate::commands::issue_listing(&issues))?;
        Ok(Json(SweepReply::new(&sweep, !args.nominal)))
    }

    /// Try every claim age for each `social-security` income whose benefit
    /// is computed from an earnings record (see `import_earnings`), and for
    /// each person with a record but no `social-security` income under a
    /// computed `ss-<person>` one made up for them, jointly for a couple,
    /// and rank the grid by what the household ends with. The best claims
    /// are returned as a scenario overlay document - a made-up income as a
    /// whole appended item - and, with `write_to`, stored through the
    /// validated write gate.
    #[tool]
    fn optimize_claims(
        &self,
        Parameters(args): Parameters<ClaimToolArgs>,
    ) -> Result<Json<WithOverlay<ClaimsReply>>, String> {
        let plan = self.load_valid_plan(&args.path)?;
        let search = optimize_claims(&plan, &self.tables, &args.incomes, &[])
            .map_err(|issues| crate::commands::issue_listing(&issues))?;
        let (scenario_toml, written) =
            self.emit_overlay(&args.path, args.write_to.as_deref(), |base| {
                claims_overlay(base, &search.added, &search.best().claims)
            })?;
        Ok(Json(WithOverlay {
            reply: ClaimsReply::new(&search, !args.nominal),
            scenario_toml,
            written,
        }))
    }
}

impl PlanServer {
    /// The overlay `serialize` gives against the right `base` - the plan's
    /// path relative to where the overlay is stored, or the path itself when
    /// it is not - and whether it was stored at `write_to` through the
    /// validated write gate.
    fn emit_overlay(
        &self,
        plan_path: &str,
        write_to: Option<&str>,
        serialize: impl FnOnce(&str) -> Result<String, PlanError>,
    ) -> Result<(String, bool), String> {
        let base = write_to.map_or_else(
            || plan_path.to_owned(),
            |target| relative_ref(target, plan_path),
        );
        let scenario_toml = serialize(&base).map_err(|err| err.to_string())?;
        let Some(target) = write_to else {
            return Ok((scenario_toml, false));
        };
        let reply = self.store_document(target, &scenario_toml)?;
        if !reply.issues.is_empty() {
            return Err(format!(
                "the overlay failed validation with {} issue(s)",
                reply.issues.len()
            ));
        }
        Ok((scenario_toml, true))
    }
}

/// The `base` reference for an overlay stored at `write_to`: the plan's
/// root-relative path re-expressed relative to `write_to`'s directory.
fn relative_ref(write_to: &str, plan_path: &str) -> String {
    let out_dir = Path::new(write_to)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    crate::commands::relative_path(out_dir, Path::new(plan_path))
}
