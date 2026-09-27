use std::collections::BTreeMap;

use retiretui_engine::params::{Inflation, TaxParams};
use retiretui_engine::plan::Dollars;
use retiretui_engine::project::{ClassTotals, Summary, YearRow, project};
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::PlanServer;
use retiretui_client::replies::ActionsReply;

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
    pub years: Years,
}

/// The projected rows, in the shape `detail` asks for.
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Years {
    /// Each year's headline figures.
    Summary(Vec<SummaryRow>),
    /// Each year's every figure: flows, balances per account, and actions.
    Full(Vec<YearRow>),
}

/// A year's headline figures.
#[derive(Serialize, JsonSchema)]
pub struct SummaryRow {
    /// The calendar year.
    pub year: i16,
    /// Age each person reaches during the year, by person id.
    pub ages: BTreeMap<String, u8>,
    /// Gross income, Social Security included.
    pub total_income: Dollars,
    /// Spending for the year.
    pub expenses: Dollars,
    /// Ordinary, gains and state tax plus penalties.
    pub taxes: Dollars,
    /// Total withdrawn across all accounts, RMDs included.
    pub withdrawals: Dollars,
    /// End-of-year balances by treatment class.
    pub class_totals: ClassTotals,
    /// Sum of all end-of-year balances.
    pub net_worth: Dollars,
    /// Unspent income swept to the surplus account.
    pub surplus: Dollars,
    /// Spending the accounts could not cover.
    pub unfunded: Dollars,
    /// Cumulative inflation factor since plan start; nominal ÷ this = today's
    /// dollars.
    pub deflator: f64,
}

impl SummaryRow {
    fn of(row: YearRow) -> Self {
        Self {
            year: row.year,
            withdrawals: row.total_withdrawals(),
            ages: row.ages,
            total_income: row.total_income,
            expenses: row.expenses,
            taxes: row.taxes.total,
            class_totals: row.class_totals,
            net_worth: row.net_worth,
            surplus: row.surplus,
            unfunded: row.unfunded,
            deflator: row.deflator,
        }
    }
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
        let rows = projection.years.into_iter().filter(|row| {
            args.from_year.is_none_or(|from| row.year >= from)
                && args.to_year.is_none_or(|to| row.year <= to)
        });
        let years = match args.detail {
            Detail::Summary => Years::Summary(rows.map(SummaryRow::of).collect()),
            Detail::Full => Years::Full(rows.collect()),
        };
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
        let year = args
            .year
            .unwrap_or_else(|| retiretui_client::session::Today::now().0);
        let row = retiretui_client::session::year_row(&projection, year)?;
        let warnings = retiretui_client::actions::collect_warnings(&plan, &self.tables, row, None);
        Ok(Json(ActionsReply::new(row, warnings)))
    }

    /// The resolved tax parameters for a year: deductions, brackets, state tables,
    /// LTCG thresholds, Social Security thresholds, contribution limits, and
    /// the RMD table.
    #[tool(name = "tax_parameters")]
    fn resolve_tax_parameters(
        &self,
        Parameters(TaxParametersArgs { year, inflation }): Parameters<TaxParametersArgs>,
    ) -> Result<Json<TaxParams>, String> {
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
        Ok(Json(
            self.tables
                .params_for(year, &Inflation::constant(inflation)),
        ))
    }

    /// The plan and scenario file schema reference: document layout, field
    /// semantics, triggers, escalation, scenario overlays, and a worked
    /// example. Read this before composing or editing plan TOML.
    #[tool]
    fn describe_schema() -> String {
        include_str!("schema.md").to_owned()
    }
}
