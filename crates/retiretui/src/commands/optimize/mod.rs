//! The `optimize` subcommands: the Roth conversion ladder and the Social
//! Security claim search.

mod claims;
mod conversions;

use clap::Subcommand;

pub use claims::{ClaimArgs, ClaimsReply};
pub use conversions::{LadderConstraints, LadderReply, OptimizeArgs, SweepReply};

/// What `optimize` searches.
#[derive(Subcommand)]
pub enum OptimizeCommand {
    /// Search a fill-bracket Roth conversion ladder and compare it to the
    /// baseline.
    Conversions(OptimizeArgs),
    /// Try every claim age for each computed Social Security benefit and
    /// rank them by what the household ends with.
    Claims(ClaimArgs),
}

pub fn run(command: &OptimizeCommand) -> anyhow::Result<()> {
    match command {
        OptimizeCommand::Conversions(args) => conversions::run(args),
        OptimizeCommand::Claims(args) => claims::run(args),
    }
}
