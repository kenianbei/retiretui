//! The Historical page: the plan run from every start year of the record,
//! each year's returns and inflation as they came, worst first.

use bevy_app::App;
use retiretui_engine::market::{History, Progress, Run, RunError, RunName, Runs, historical};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;

use super::assumptions::Assumption;
use super::{MarketTool, count_text};
use crate::commands::tui::nav::Page;
use crate::commands::tui::tools::Found;
use crate::commands::tui::tools::options::Laid;

pub fn plugin(app: &mut App) {
    super::install::<Runs>(app);
}

impl Found for Runs {
    const NOTHING_SEARCHED: &'static str = super::NOTHING_SEARCHED;
    const IS_COUNTED: bool = true;

    fn laid(&self, plan: &Plan, _: bool) -> Laid {
        super::laid(self, plan)
    }
}

impl MarketTool for Runs {
    const PAGE: Page = Page::Historical;
    const RUN_HEADING: &'static str = "Start Years";
    const HELP: &'static str = "Each year of the record as the plan's first, its markets and inflation as they came. Today's dollars.";

    const OPEN: &'static str = "open-historical-run";
    const EDIT: &'static str = "historical-assumption";
    const VIEW: &'static str = "historical-view";
    const HEADLINE: &'static str = "survived";
    const HAS_BY_YEAR: bool = false;
    const VERDICT: &'static str = "Survived";

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

    fn runs(&self) -> &Runs {
        self
    }

    fn listed(&self) -> Vec<(String, &Run)> {
        self.worst_first()
            .into_iter()
            .map(|at| {
                let run = &self.runs[at];
                let started = match run.name {
                    RunName::Start(year) => year.to_string(),
                    RunName::Planned | RunName::Trial(_) => String::new(),
                };
                (started, run)
            })
            .collect()
    }

    fn ledger_label(first: &str) -> String {
        format!("retiring in {first}")
    }

    fn verdict(&self) -> String {
        format!(
            "{} of {}",
            count_text(self.successes),
            count_text(self.runs.len())
        )
    }

    fn settings(plan: &Plan) -> Vec<Assumption> {
        let market = plan.market();
        let wrapped = if market.wrap() { ", wrapped" } else { "" };
        let years = format!("{}–{}{wrapped}", market.from(), market.to());
        vec![("Years", years, Page::Market)]
    }
}
