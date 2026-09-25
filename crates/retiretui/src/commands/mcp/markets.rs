use retiretui_engine::market::{self, Progress};
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

use super::PlanServer;
use crate::commands::markets::{HistoricalReply, MonteCarloReply, run_refusal};

#[derive(Deserialize, JsonSchema)]
pub struct MarketToolArgs {
    /// Plan or scenario path, relative to the served directory.
    pub path: String,
}

#[tool_router(router = markets_router, vis = "pub(super)")]
impl PlanServer {
    /// Run the plan through many random markets as its `[market]` states -
    /// drawn from the asset assumptions, or historical years drawn at random
    /// - and report how many kept spending covered (and left
    /// `leave_at_least`), the plan as planned, the markets at the 90th, 75th,
    /// 50th, 25th and 10th percentile and the worst, and net worth by year
    /// at those percentiles. Every figure is in today's dollars by each
    /// market's own inflation. A scenario changes the settings or the
    /// assumptions.
    #[tool]
    fn plan_monte_carlo(
        &self,
        Parameters(args): Parameters<MarketToolArgs>,
    ) -> Result<Json<MonteCarloReply>, String> {
        let plan = self.load_valid_plan(&args.path)?;
        let found = market::monte_carlo(&plan, &self.tables, &self.history, &Progress::default())
            .map_err(run_refusal)?;
        Ok(Json(MonteCarloReply::new(&plan, &found)))
    }

    /// Run the plan through history as if retired in each start year its
    /// `[market.historical]` names (1871 to 2025 by default), each year's
    /// returns and inflation as they came, and report how many start years
    /// kept spending covered, the plan as planned, then every start year
    /// worst first, in today's dollars.
    #[tool]
    fn plan_historical(
        &self,
        Parameters(args): Parameters<MarketToolArgs>,
    ) -> Result<Json<HistoricalReply>, String> {
        let plan = self.load_valid_plan(&args.path)?;
        let found = market::historical(&plan, &self.tables, &self.history, &Progress::default())
            .map_err(run_refusal)?;
        Ok(Json(HistoricalReply::new(&plan, &found)))
    }
}
