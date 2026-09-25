//! The optimizers: each searches a plan by re-projecting candidate plans,
//! so every interaction the engine models is priced in rather than
//! approximated, and emits its answer as a scenario overlay.

mod claims;
mod conversions;
mod estimate;

pub use claims::{
    Claim, ClaimCandidate, ClaimSearch, apply_claims, claims_overlay, optimize_claims,
};
pub use conversions::{
    BracketSweep, LADDER_ID_PREFIX, LadderStep, OptimizeOptions, OptimizedLadder, SweptBracket,
    apply_ladder, is_ladder, ladder_overlay, optimize_conversions, sweep_brackets,
};
pub use estimate::{benefit_estimates, career_at_salary};

use std::cmp::Reverse;

use crate::plan::Dollars;
use crate::project::Projection;

/// What a search ranks an outcome by, best first: the least left
/// unfunded, then the most left at the end in today's dollars.
#[must_use]
pub fn rank_key(projection: &Projection) -> (Dollars, Reverse<Dollars>) {
    let summary = projection.summary(true);
    (summary.lifetime_unfunded, Reverse(summary.final_net_worth))
}
