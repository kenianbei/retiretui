//! The Monte Carlo page: the plan run through random markets, drawn from
//! its assumptions or from historical years, and the runs at the 90th,
//! 75th, 50th, 25th and 10th percentile and the worst.

use bevy_app::App;
use retiretui_engine::market::{
    BAND_PERCENTILES, History, MonteCarlo, Progress, Run, RunError, Runs, monte_carlo,
};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Draw, Plan};

use super::assumptions::{Assumption, assumed};
use super::{MarketTool, count_text, share_text};
use crate::commands::markets::percentile_label;
use crate::commands::tui::nav::Page;
use crate::commands::tui::present;
use crate::commands::tui::tools::Found;
use crate::commands::tui::tools::options::Laid;

pub fn plugin(app: &mut App) {
    super::install::<MonteCarlo>(app);
}

impl Found for MonteCarlo {
    const NOTHING_SEARCHED: &'static str = super::NOTHING_SEARCHED;
    const IS_COUNTED: bool = true;

    fn laid(&self, plan: &Plan, _: bool) -> Laid {
        super::laid(self, plan)
    }
}

impl MarketTool for MonteCarlo {
    const PAGE: Page = Page::MonteCarlo;
    const RUN_HEADING: &'static str = "Markets";
    const HELP: &'static str = "Random markets from the assumptions or from history; the same seed draws the same markets. Today's dollars.";

    const OPEN: &'static str = "open-monte-carlo-run";
    const EDIT: &'static str = "monte-carlo-assumption";
    const VIEW: &'static str = "monte-carlo-view";
    const HEADLINE: &'static str = "money lasts in";
    const HAS_BY_YEAR: bool = true;
    const VERDICT: &'static str = "Money lasts";

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

    fn runs(&self) -> &Runs {
        &self.runs
    }

    fn listed(&self) -> Vec<(String, &Run)> {
        let labels = BAND_PERCENTILES
            .iter()
            .rev()
            .map(|&percentile| percentile_label(percentile));
        let mut listed: Vec<(String, &Run)> = labels
            .zip(&self.singled_out)
            .map(|(label, &at)| (label, &self.runs.runs[at]))
            .collect();
        if let Some(&worst) = self.singled_out.get(BAND_PERCENTILES.len()) {
            listed.push(("Worst".to_owned(), &self.runs.runs[worst]));
        }
        listed
    }

    fn ledger_label(first: &str) -> String {
        format!("the {} market", first.to_lowercase())
    }

    fn verdict(&self) -> String {
        format!(
            "{} of {}",
            share_text(&self.runs),
            count_text(self.runs.runs.len())
        )
    }

    fn settings(plan: &Plan) -> Vec<Assumption> {
        let market = plan.market();
        let drawn = match market.draw() {
            Draw::History if market.block_years() > 1 => {
                format!(
                    "{}, {} together",
                    present::draw(Draw::History),
                    market.block_years()
                )
            }
            draw => present::draw(draw).to_owned(),
        };
        let trials = format!(
            "{} · seed {}",
            count_text(usize::try_from(market.trials()).unwrap_or_default()),
            market.seed()
        );
        let mut rows = vec![
            ("Draw from", drawn, Page::Market),
            ("Trials", trials, Page::Market),
        ];
        rows.extend(assumed(plan));
        rows
    }
}
