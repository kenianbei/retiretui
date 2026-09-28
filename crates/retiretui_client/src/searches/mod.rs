//! The searches every interface runs over a plan, beyond the engine's own.

pub mod claims;
pub mod ladders;
pub mod markets;
pub mod overview;

use retiretui_engine::market::RunError;
use retiretui_engine::plan::Dollars;
use retiretui_engine::project::Summary;

use crate::issues::issue_listing;

/// What the plan's own row says first, above a search's options.
pub const CURRENT_PLAN: &str = "Current";
/// The figures an option is chosen by, in the order the options are ranked
/// by and then what they cost; the rest of a summary is the Compare tab's.
pub const FIGURES: [&str; 4] = ["unfunded", "final net", "taxes", "medicare"];

/// A summary's [`FIGURES`], in their order.
#[must_use]
pub const fn figure_amounts(summary: &Summary) -> [Dollars; 4] {
    [
        summary.lifetime_unfunded,
        summary.final_net_worth,
        summary.lifetime_taxes,
        summary.lifetime_medicare,
    ]
}

/// Why a search answered nothing, as the CLI and MCP say it.
#[must_use]
pub fn run_refusal(error: RunError) -> String {
    match error {
        RunError::Cancelled => "the search was cancelled".to_owned(),
        RunError::Refused(issues) => issue_listing(&issues),
    }
}
