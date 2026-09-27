//! The searches every interface runs over a plan, beyond the engine's own.

pub mod ladders;
pub mod overview;

use retiretui_engine::market::RunError;

use crate::issues::issue_listing;

/// Why a search answered nothing, as the CLI and MCP say it.
#[must_use]
pub fn run_refusal(error: RunError) -> String {
    match error {
        RunError::Cancelled => "the search was cancelled".to_owned(),
        RunError::Refused(issues) => issue_listing(&issues),
    }
}
