//! The Historical page: the plan run from every start year of the record,
//! each year's returns and inflation as they came, worst first.

use bevy_app::App;
use retiretui_client::searches::markets;
use retiretui_engine::market::{History, Progress, RunError, Runs, historical};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;

use super::MarketTool;
use crate::nav::Page;
use crate::overview::Better;
use crate::tools::Found;
use crate::tools::options::Laid;

pub fn plugin(app: &mut App) {
    super::install::<Runs>(app);
}

impl Found for Runs {
    const NOTHING_SEARCHED: &'static str = markets::NOTHING_SEARCHED;
    const IS_COUNTED: bool = true;
    const IS_PLAN_ROW_CHOSEN: bool = true;

    fn laid(&self, plan: &Plan, _: bool) -> Laid {
        super::laid(self, plan)
    }
}

impl MarketTool for Runs {
    const PAGE: Page = Page::Historical;
    const HELP: &'static str = "Each year of the record as the plan's first, its markets and inflation as they came. Today's dollars.";

    const OPEN: &'static str = "open-historical-run";
    const EDIT: &'static str = "historical-assumption";
    const VIEW: &'static str = "historical-view";

    fn search(
        plan: &Plan,
        tables: &TaxTables,
        history: &History,
        progress: &Progress,
    ) -> Result<Self, RunError> {
        historical(plan, tables, history, progress)
    }

    fn total(plan: &Plan) -> usize {
        let market = plan.market();
        usize::try_from(market.to() - market.from() + 1).unwrap_or_default()
    }

    fn found_by(better: &Better, plan: &Plan) -> Option<Self> {
        better.historical(plan).cloned()
    }

    fn ledger_label(first: &str) -> String {
        format!("retiring in {first}")
    }
}
