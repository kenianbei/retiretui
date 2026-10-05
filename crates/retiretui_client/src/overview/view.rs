//! The Overview as one value: its strip, the sentence for a plan that runs
//! short, its lists, its totals and what it rests on. What a search
//! answers, the share of markets survived and what could do better, is
//! drawn beside it.

use retiretui_engine::plan::IncomeKind;
use retiretui_engine::project::YearRow;

use serde::Serialize;

use super::{Place, Row, Total, attention, milestones, rests_on, totals};
use crate::forms::DomainId;
use crate::present::{
    BALANCES_CHART, ENDS_WITH, INCOME_CHART, MONEY_LASTS, NET_WORTH_CHART, SUCCESS, compact_money,
    lasts_through, runs_short,
};
use crate::searches::markets::Assumption;
use crate::session::Projected;
use crate::table::basis_amount;

/// What the strip calls the least the household holds once it stops earning.
const LOW_POINT: &str = "Lowest after retiring";
/// The strip's labels in its order; a search answers the second.
pub const STRIP: [&str; 4] = [MONEY_LASTS, SUCCESS, LOW_POINT, ENDS_WITH];
/// What the low point says of a household still paid a salary in its last
/// year.
const STILL_EARNING: &str = "Earning to the end";
/// What a surface says of its figures while the draft has issues.
pub const STALE: &str = "The figures are the last the plan had without its issues.";

/// A chart of the Overview's.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum Chart {
    /// What each tax treatment holds.
    #[default]
    Balances,
    /// Everything the household holds.
    NetWorth,
    /// What it earns against what it pays in tax.
    IncomeTaxes,
    /// Its net worth through random markets.
    Markets,
}

impl Chart {
    /// Every chart, in the order they are turned through.
    pub const ALL: [Self; 4] = [
        Self::Balances,
        Self::NetWorth,
        Self::IncomeTaxes,
        Self::Markets,
    ];

    /// What the chart is titled.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Balances => BALANCES_CHART,
            Self::NetWorth => NET_WORTH_CHART,
            Self::IncomeTaxes => INCOME_CHART,
            Self::Markets => "Net worth through random markets",
        }
    }
}

/// Where a plan runs short.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Shortfall {
    /// The first year it does.
    pub year: i16,
    /// That year and what the money cannot cover, as a sentence.
    pub said: String,
    /// Where what the plan spends is edited.
    pub place: Place,
}

/// What the Overview says of a projection, in one dollar basis.
#[derive(Clone, PartialEq, Debug)]
pub struct View {
    /// How long the money lasts: "Never short", "Through 2041".
    pub money_lasts: String,
    /// Where it runs short; none where it never does.
    pub shortfall: Option<Shortfall>,
    /// The least the household holds once the last salary ends, and when.
    pub low_point: String,
    /// Net worth at the end.
    pub ends_with: String,
    /// The plan's milestones, earliest first.
    pub milestones: Vec<Row>,
    /// What needs attention in the projection, what has no year first.
    pub attention: Vec<Row>,
    /// What the years add up to.
    pub totals: Vec<Total>,
    /// What the projection hangs from.
    pub rests_on: Vec<Assumption>,
}

impl View {
    /// `projected` as the Overview says it, nominal or in today's dollars.
    #[must_use]
    pub fn new(projected: &Projected, nominal: bool) -> Self {
        let summary = projected.projection.summary(!nominal);
        let shortfall =
            summary
                .first_unfunded_year
                .zip(runs_short(&summary))
                .map(|(year, said)| Shortfall {
                    year,
                    said,
                    place: (DomainId::Expenses, None),
                });
        Self {
            money_lasts: lasts_through(&summary, projected.plan.plan.start_year),
            shortfall,
            low_point: low_point(projected, nominal),
            ends_with: compact_money(summary.final_net_worth),
            milestones: milestones(projected, nominal),
            attention: attention(projected, nominal),
            totals: totals(projected, nominal),
            rests_on: rests_on(projected),
        }
    }
}

/// The lowest year-end net worth after the last year any salary pays, and
/// its year, the earliest on a tie. A plan with no salary is measured from
/// its first year.
fn low_point(projected: &Projected, nominal: bool) -> String {
    let years = &projected.projection.years;
    let salaries = projected.plan.income.iter();
    let salaries: Vec<&str> = salaries
        .filter(|income| income.kind == IncomeKind::Salary)
        .map(|income| income.id.as_str())
        .collect();
    let last_paid = salaries
        .iter()
        .filter_map(|id| last_paid_at(years, id))
        .max();
    let after = last_paid.map_or(0, |at| at + 1);
    years[after..]
        .iter()
        .map(|row| (basis_amount(row.net_worth, row.deflator, nominal), row.year))
        .min_by_key(|&(worth, _)| worth)
        .map_or_else(
            || STILL_EARNING.to_owned(),
            |(worth, year)| format!("{} in {year}", compact_money(worth)),
        )
}

/// The place of the last year the income `id` pays anything.
fn last_paid_at(years: &[YearRow], id: &str) -> Option<usize> {
    years
        .iter()
        .rposition(|row| row.income.get(id).is_some_and(|&amount| amount > 0))
}

#[cfg(test)]
mod tests {
    use retiretui_engine::plan::Dollars;

    use super::super::tests::{TEST_PLAN, projected_from, test_projected};
    use super::*;

    fn lowest_from(projected: &Projected, first: i16) -> (Dollars, i16) {
        let years = projected.projection.years.iter();
        years
            .filter(|row| row.year >= first)
            .map(|row| (row.net_worth, row.year))
            .min()
            .unwrap()
    }

    #[test]
    fn the_low_point_is_measured_once_the_last_salary_has_ended() {
        let thrifty = TEST_PLAN.replace("amount = 60000", "amount = 20000");
        let projected = projected_from(&thrifty);
        let years = &projected.projection.years;
        let paid = |row: &&YearRow| row.income.contains_key("salary");
        let last_paid = years.iter().rfind(paid).unwrap().year;
        let (worth, year) = lowest_from(&projected, last_paid + 1);
        let (least, _) = lowest_from(&projected, i16::MIN);
        assert!(least < worth, "it holds less while it is still saving");
        let view = View::new(&projected, true);
        assert_eq!(
            view.low_point,
            format!("{} in {year}", compact_money(worth))
        );
        assert_ne!(View::new(&projected, false).low_point, view.low_point);
    }

    #[test]
    fn a_plan_with_no_salary_is_measured_from_its_first_year() {
        let unpaid = TEST_PLAN.replace("kind = \"salary\"", "kind = \"pension\"");
        let projected = projected_from(&unpaid);
        let (worth, year) = lowest_from(&projected, i16::MIN);
        let view = View::new(&projected, true);
        assert_eq!(
            view.low_point,
            format!("{} in {year}", compact_money(worth))
        );
    }

    #[test]
    fn a_household_paid_to_the_end_has_no_low_point_to_say() {
        let paid = TEST_PLAN.replace("end = { age = 60, owner = \"me\" }\n", "");
        let view = View::new(&projected_from(&paid), true);
        assert_eq!(view.low_point, STILL_EARNING);
    }

    #[test]
    fn a_plan_that_runs_short_says_through_when_and_from_when() {
        let lasting = View::new(&test_projected(), true);
        assert_eq!(
            (lasting.money_lasts.as_str(), lasting.shortfall),
            ("Never short", None)
        );

        let short = TEST_PLAN.replace("amount = 60000", "amount = 95000");
        let view = View::new(&projected_from(&short), true);
        let shortfall = view.shortfall.unwrap();
        assert_eq!(view.money_lasts, format!("Through {}", shortfall.year - 1));
        assert!(
            shortfall
                .said
                .starts_with(&format!("Runs short from {}", shortfall.year))
        );
    }

    #[test]
    fn a_chart_is_named_in_an_address_as_it_is_turned_to() {
        let keys = Chart::ALL.map(|chart| toml::Value::try_from(chart).unwrap());
        let named = ["balances", "net-worth", "income-taxes", "markets"];
        assert_eq!(keys, named.map(toml::Value::from));
        assert_eq!(Chart::default(), Chart::ALL[0]);
    }
}
