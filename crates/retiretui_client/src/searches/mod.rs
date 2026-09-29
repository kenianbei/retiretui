//! The searches every interface runs over a plan, beyond the engine's own.

pub mod claims;
pub mod ladders;
pub mod markets;
pub mod overview;

use retiretui_engine::market::RunError;
use retiretui_engine::project::Summary;

use crate::issues::issue_listing;
use crate::present::{self, MoneyForm};

/// What the plan's own row says first, above a search's options.
pub const CURRENT_PLAN: &str = "Current";
/// The figures an option is chosen by, in the order the options are ranked
/// by and then what they cost; the rest of a summary is the Compare tab's.
pub const FIGURES: [&str; 4] = ["Unfunded", present::ENDS_WITH, "Taxes", "Medicare"];

/// The column saying how much more an option ends with than the plan.
pub const AGAINST_PLAN: &str = "Vs. the plan";

/// An option's cells from [`AGAINST_PLAN`] on, in `form`: what `own`
/// ends with against `plan`, blank on the plan's own row, where there is
/// none to set it against, then its [`FIGURES`].
#[must_use]
pub fn option_cells(own: &Summary, plan: Option<&Summary>, form: MoneyForm) -> Vec<String> {
    let against = plan.map_or_else(String::new, |plan| {
        form.signed(own.final_net_worth - plan.final_net_worth)
    });
    let figures = [
        own.lifetime_unfunded,
        own.final_net_worth,
        own.lifetime_taxes,
        own.lifetime_medicare,
    ];
    std::iter::once(against)
        .chain(figures.map(|amount| form.money(amount)))
        .collect()
}

/// Why a search answered nothing, as the CLI and MCP say it.
#[must_use]
pub fn run_refusal(error: RunError) -> String {
    match error {
        RunError::Cancelled => "the search was cancelled".to_owned(),
        RunError::Refused(issues) => issue_listing(&issues),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ending(final_net_worth: i64) -> Summary {
        Summary {
            final_net_worth,
            peak_net_worth: final_net_worth,
            peak_year: 2060,
            lifetime_taxes: 1_370_211,
            lifetime_conversions: 635_410,
            lifetime_unfunded: 0,
            lifetime_medicare: 0,
            first_unfunded_year: None,
            final_deferred: 0,
        }
    }

    #[test]
    fn an_option_says_what_it_ends_with_against_the_plan() {
        let (plan, option) = (ending(4_437_120), ending(5_829_124));
        let full = option_cells(&option, Some(&plan), MoneyForm::Full);
        assert_eq!(
            full,
            ["+$1,392,004", "$0", "$5,829,124", "$1,370,211", "$0"]
        );
        let compact = option_cells(&plan, Some(&option), MoneyForm::Compact);
        assert_eq!(compact[..3], ["-$1.39M", "$0", "$4.44M"]);
        assert_eq!(
            option_cells(&plan, None, MoneyForm::Full)[0],
            "",
            "the plan's own row"
        );
        let ladder = ladders::option_cells(&option, Some(&plan), MoneyForm::Full);
        assert_eq!(ladder[..2], ["+$1,392,004", "$635,410"]);
    }
}
