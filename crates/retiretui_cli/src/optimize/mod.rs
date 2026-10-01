//! The `optimize` subcommands: the Roth conversion ladder, the Social
//! Security claim search and the withdrawal-order search.

mod claims;
mod conversions;
mod order;

use clap::Subcommand;

pub use claims::ClaimArgs;
pub use conversions::OptimizeArgs;
pub use order::OrderArgs;

/// What `optimize` searches.
#[derive(Subcommand, Debug)]
pub enum OptimizeCommand {
    /// Search a fill-bracket Roth conversion ladder and compare it to the
    /// baseline.
    Conversions(OptimizeArgs),
    /// Try every claim age for each computed Social Security benefit and
    /// rank them by what the household ends with.
    Claims(ClaimArgs),
    /// Try every order the classes the plan withdraws from can be drained
    /// in and rank them by what the household ends with.
    Order(OrderArgs),
}

pub fn run(command: &OptimizeCommand) -> anyhow::Result<()> {
    match command {
        OptimizeCommand::Conversions(args) => conversions::run(args),
        OptimizeCommand::Claims(args) => claims::run(args),
        OptimizeCommand::Order(args) => order::run(args),
    }
}
