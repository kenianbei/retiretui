//! The Monte Carlo page: the plan run through random markets, drawn from
//! its assumptions or from historical years, and the runs at the 90th,
//! 75th, 50th, 25th and 10th percentile and the worst.

use bevy_app::App;
use retiretui_client::searches::markets;
use retiretui_engine::market::{History, MonteCarlo, Progress, RunError, monte_carlo};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;

use super::MarketTool;
use crate::nav::Page;
use crate::tools::Found;
use crate::tools::options::Laid;

pub fn plugin(app: &mut App) {
    super::install::<MonteCarlo>(app);
}

impl Found for MonteCarlo {
    const NOTHING_SEARCHED: &'static str = markets::NOTHING_SEARCHED;
    const IS_COUNTED: bool = true;
    const IS_PLAN_ROW_CHOSEN: bool = true;

    fn laid(&self, plan: &Plan, _: bool) -> Laid {
        super::laid(self, plan)
    }
}

impl MarketTool for MonteCarlo {
    const PAGE: Page = Page::MonteCarlo;

    const OPEN: &'static str = "open-monte-carlo-run";
    const EDIT: &'static str = "monte-carlo-assumption";
    const VIEW: &'static str = "monte-carlo-view";

    fn search(
        plan: &Plan,
        tables: &TaxTables,
        history: &History,
        progress: &Progress,
    ) -> Result<Self, RunError> {
        monte_carlo(plan, tables, history, progress)
    }

    fn total(plan: &Plan) -> usize {
        usize::try_from(plan.market().trials()).unwrap_or_default()
    }

    fn ledger_label(first: &str) -> String {
        format!("the {} market", first.to_lowercase())
    }
}
