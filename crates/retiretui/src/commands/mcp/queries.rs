use std::collections::BTreeMap;

use retiretui_engine::params::Inflation;
use retiretui_engine::plan::Dollars;
use retiretui_engine::project::{ClassTotals, Summary, YearRow, project};
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::PlanServer;
use crate::commands::actions::{self, ActionsReply};

#[derive(Deserialize, JsonSchema)]
pub struct ProjectPlanArgs {
    /// Plan file path, relative to the served directory.
    pub path: String,
    /// First projected year to include; absent means the plan's start year.
    pub from_year: Option<i16>,
    /// Last projected year to include; absent means the horizon.
    pub to_year: Option<i16>,
    /// Row shape: a compact summary (default) or the full engine row with
    /// per-account balances and per-source income.
    #[serde(default)]
    pub detail: Detail,
}

/// How much of each projected year to return.
#[derive(Deserialize, JsonSchema, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
pub enum Detail {
    /// Year totals and treatment-class balances.
    #[default]
    Summary,
    /// The complete engine row.
    Full,
}

#[derive(Serialize, JsonSchema)]
pub struct ProjectionReply {
    /// One row per projected year, in nominal dollars; divide by each row's
    /// `deflator` for today's dollars. The shape follows `detail`.
    pub years: Vec<serde_json::Value>,
}

#[derive(Serialize)]
struct SummaryRow<'a> {
    year: i16,
    ages: &'a BTreeMap<String, u8>,
    total_income: Dollars,
    expenses: Dollars,
    taxes: Dollars,
    withdrawals: Dollars,
    class_totals: ClassTotals,
    net_worth: Dollars,
    surplus: Dollars,
    unfunded: Dollars,
    deflator: f64,
}

#[derive(Deserialize, JsonSchema)]
pub struct ComparePlansArgs {
    /// Two or more plan or scenario paths, relative to the served directory.
    pub paths: Vec<String>,
    /// Report nominal dollars instead of today's dollars.
    #[serde(default)]
    pub nominal: bool,
}

#[derive(Serialize, JsonSchema)]
pub struct CompareReply {
    /// One entry per requested path, in request order.
    pub plans: Vec<ComparedPlan>,
}

#[derive(Serialize, JsonSchema)]
pub struct ComparedPlan {
    /// The requested path.
    pub path: String,
    /// The resolved plan's display name, when set.
    pub name: Option<String>,
    /// Headline figures over the whole projection.
    pub summary: Summary,
}

#[derive(Deserialize, JsonSchema)]
pub struct PlanActionsArgs {
    /// Plan file path, relative to the served directory.
    pub path: String,
    /// The year to report; absent means the current calendar year.
    pub year: Option<i16>,
}

#[derive(Deserialize, JsonSchema)]
pub struct TaxParametersArgs {
    /// The tax year to resolve.
    pub year: i16,
    /// Annual inflation rate used to extend indexed values past the last
    /// known table; required only for years beyond it.
    pub inflation: Option<f64>,
}

#[tool_router(router = query_router, vis = "pub(super)")]
impl PlanServer {
    /// Project a plan year by year against U.S. federal tax law. The plan is
    /// validated first; an invalid plan is refused with its issues.
    #[tool]
    fn project_plan(
        &self,
        Parameters(args): Parameters<ProjectPlanArgs>,
    ) -> Result<Json<ProjectionReply>, String> {
        let plan = self.load_valid_plan(&args.path)?;
        let projection = project(&plan, &self.tables);
        let years = projection
            .years
            .iter()
            .filter(|row| {
                args.from_year.is_none_or(|from| row.year >= from)
                    && args.to_year.is_none_or(|to| row.year <= to)
            })
            .map(|row| row_value(row, args.detail))
            .collect();
        Ok(Json(ProjectionReply { years }))
    }

    /// Compare two or more plans or scenarios: each is resolved, validated,
    /// and projected, and its headline summary figures returned - in today's
    /// dollars unless `nominal`. Year-by-year series come from
    /// `project_plan`.
    #[tool]
    fn compare_plans(
        &self,
        Parameters(ComparePlansArgs { paths, nominal }): Parameters<ComparePlansArgs>,
    ) -> Result<Json<CompareReply>, String> {
        if paths.len() < 2 {
            return Err("compare_plans needs at least two paths".to_owned());
        }
        let plans = paths
            .into_iter()
            .map(|path| {
                let plan = self.load_valid_plan(&path)?;
                let summary = project(&plan, &self.tables).summary(!nominal);
                Ok(ComparedPlan {
                    path,
                    name: plan.plan.name,
                    summary,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Json(CompareReply { plans }))
    }

    /// One year's concrete to-dos for a plan: executed conversions, RMDs,
    /// transfers, contributions, and funding withdrawals with nominal
    /// amounts, plus warnings. Defaults to the current calendar year.
    #[tool]
    fn plan_actions(
        &self,
        Parameters(args): Parameters<PlanActionsArgs>,
    ) -> Result<Json<ActionsReply>, String> {
        let plan = self.load_valid_plan(&args.path)?;
        let projection = project(&plan, &self.tables);
        let year = args.year.unwrap_or_else(actions::current_year);
        let row = actions::year_row(&projection, year)?;
        let warnings = actions::collect_warnings(&plan, &self.tables, row, None);
        Ok(Json(ActionsReply::new(row, warnings)))
    }

    /// The resolved tax parameters for a year: deductions, brackets, state tables,
    /// LTCG thresholds, Social Security thresholds, contribution limits, and
    /// the RMD table.
    #[tool(name = "tax_parameters")]
    fn resolve_tax_parameters(
        &self,
        Parameters(TaxParametersArgs { year, inflation }): Parameters<TaxParametersArgs>,
    ) -> Result<Json<serde_json::Value>, String> {
        let latest = self.tables.latest_known_year().unwrap_or(year);
        let inflation = match inflation {
            Some(rate) => rate,
            None if year <= latest => 0.0,
            None => {
                return Err(format!(
                    "{year} is past the last known table ({latest}); \
                     pass `inflation` to extend it"
                ));
            }
        };
        serde_json::to_value(
            self.tables
                .params_for(year, &Inflation::constant(inflation)),
        )
        .map(Json)
        .map_err(|err| err.to_string())
    }

    /// The plan and scenario file schema reference: document layout, field
    /// semantics, triggers, escalation, scenario overlays, and a worked
    /// example. Read this before composing or editing plan TOML.
    #[tool]
    fn describe_schema() -> String {
        include_str!("schema.md").to_owned()
    }
}

fn row_value(row: &YearRow, detail: Detail) -> serde_json::Value {
    let value = match detail {
        Detail::Full => serde_json::to_value(row),
        Detail::Summary => serde_json::to_value(SummaryRow {
            year: row.year,
            ages: &row.ages,
            total_income: row.total_income,
            expenses: row.expenses,
            taxes: row.taxes.total,
            withdrawals: row.total_withdrawals(),
            class_totals: row.class_totals,
            net_worth: row.net_worth,
            surplus: row.surplus,
            unfunded: row.unfunded,
            deflator: row.deflator,
        }),
    };
    value.expect("plain data serializes")
}
