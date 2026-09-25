use serde::{Deserialize, Serialize};

use super::Dollars;

/// Opt-in Medicare surcharge modeling. Base Part B/D premiums stay the
/// plan's own expense items; the engine spends only IRMAA surcharges,
/// driven by MAGI from two years before each covered year.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Medicare {
    /// MAGI for up to the two calendar years before plan start, oldest
    /// first, seeding the lookback; missing years count below every tier.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prior_magi: Vec<Dollars>,
    /// Include Part D surcharges beside Part B.
    #[serde(default = "default_part_d")]
    pub part_d: bool,
}

fn default_part_d() -> bool {
    true
}
