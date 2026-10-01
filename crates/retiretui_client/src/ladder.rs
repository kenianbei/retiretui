//! What a Roth conversion ladder is held to.

use retiretui_engine::optimize::{GainsRate, OptimizeOptions};
use retiretui_engine::plan::Dollars;
use schemars::JsonSchema;
use serde::Deserialize;

/// What a ladder is held to, as `optimize conversions` and the MCP
/// optimizer tools take it.
#[derive(Deserialize, JsonSchema, Debug, Default)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct LadderConstraints {
    /// First conversion year; defaults to plan start.
    #[cfg_attr(feature = "clap", arg(long))]
    pub start_year: Option<i16>,
    /// Last conversion year; defaults to the year before the owner's RMDs.
    #[cfg_attr(feature = "clap", arg(long))]
    pub end_year: Option<i16>,
    /// Cap on any single year's conversion.
    #[cfg_attr(feature = "clap", arg(long))]
    pub annual_max: Option<Dollars>,
    /// Cap on total conversions across the ladder.
    #[cfg_attr(feature = "clap", arg(long))]
    pub total_max: Option<Dollars>,
    /// Dollars left unfilled below the bracket top.
    #[cfg_attr(feature = "clap", arg(long, default_value_t = 0))]
    #[serde(default)]
    pub headroom: Dollars,
    /// Highest IRMAA tier the ladder may buy (0 = under every surcharge);
    /// requires a `[medicare]` section in the plan.
    #[cfg_attr(feature = "clap", arg(long))]
    pub irmaa_tier: Option<u8>,
    /// Explicit MAGI ceiling in today's dollars.
    #[cfg_attr(feature = "clap", arg(long))]
    pub max_magi: Option<Dollars>,
    /// The rate realized long-term gains may not be pushed past: 0 keeps
    /// them untaxed, 15 keeps them out of 20%.
    #[cfg_attr(feature = "clap", arg(long))]
    pub gains_rate: Option<GainsRate>,
}

impl LadderConstraints {
    /// The options a ladder from `sources` into `destination` is searched
    /// under.
    #[must_use]
    pub fn options(&self, sources: &[String], destination: &str) -> OptimizeOptions {
        OptimizeOptions {
            sources: sources.to_vec(),
            destination: destination.to_owned(),
            start_year: self.start_year,
            end_year: self.end_year,
            annual_max: self.annual_max,
            total_max: self.total_max,
            headroom: self.headroom,
            irmaa_tier: self.irmaa_tier,
            max_magi: self.max_magi,
            gains_rate: self.gains_rate,
        }
    }
}
