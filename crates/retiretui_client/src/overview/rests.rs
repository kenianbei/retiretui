//! Rests on: the assumptions the whole projection hangs from, each at the
//! field it is edited at.

use retiretui_engine::plan::AssetClass;

use crate::forms::DomainId;
use crate::present::{filing_status, residence};
use crate::searches::markets::{Assumption, counts_as_success};
use crate::session::{Projected, span};
use crate::table::rate;

/// What the Overview titles its assumptions.
pub const RESTS_ON: &str = "Rests on";

/// How far the plan runs, what it assumes of prices and returns, how the
/// household files and where it lives, and what counts as lasting.
#[must_use]
pub fn rests_on(projected: &Projected) -> Vec<Assumption> {
    let plan = &projected.plan;
    let market = plan.market();
    let (_, last) = span(&projected.projection.years);
    let returns = [
        ("stocks", AssetClass::Stocks),
        ("bonds", AssetClass::Bonds),
        ("cash", AssetClass::Cash),
    ]
    .map(|(name, class)| format!("{name} {}", rate(market.mean(class))));
    let row = |label, value, domain, field| Assumption {
        label,
        value,
        domain,
        field: Some(field),
    };
    let mut rows = vec![
        row(
            "Runs through",
            format!("{last} · to age {}", plan.plan.horizon_age),
            DomainId::Settings,
            "horizon_age",
        ),
        row(
            "Inflation",
            rate(plan.plan.inflation),
            DomainId::Settings,
            "inflation",
        ),
        row(
            "Returns",
            returns.join(", "),
            DomainId::Market,
            "stocks.mean",
        ),
        row(
            "Filing status",
            filing_status(plan.household.filing).to_owned(),
            DomainId::Household,
            "filing",
        ),
    ];
    if let Some(home) = plan.residency.first() {
        let lived_in = residence(home).to_owned();
        rows.push(row("Lives in", lived_in, DomainId::Residency, "state"));
    }
    rows.push(Assumption {
        label: "Success is",
        ..counts_as_success(plan)
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::super::tests::{projected_from, test_projected};
    use super::*;

    const FULL: &str = include_str!("../../../retiretui_engine/tests/fixtures/full.toml");

    #[test]
    fn each_assumption_is_said_at_the_field_it_is_edited_at() {
        let rows: Vec<_> = rests_on(&test_projected())
            .into_iter()
            .map(|row| (row.label, row.value, row.domain, row.field))
            .collect();
        assert_eq!(rows.len(), 5);
        assert_eq!(
            rows[0],
            (
                "Runs through",
                "2050 · to age 70".to_owned(),
                DomainId::Settings,
                Some("horizon_age")
            )
        );
        assert_eq!((rows[1].1.as_str(), rows[1].3), ("2.5%", Some("inflation")));
        assert!(rows[2].1.starts_with("stocks ") && rows[2].1.contains(", bonds "));
        assert_eq!(
            (rows[2].2, rows[2].3),
            (DomainId::Market, Some("stocks.mean"))
        );
        assert_eq!(
            (rows[3].1.as_str(), rows[3].2),
            ("Single", DomainId::Household)
        );
        assert_eq!(
            (rows[4].0, rows[4].1.as_str(), rows[4].3),
            ("Success is", "Never running short", Some("leave_at_least"))
        );
    }

    #[test]
    fn a_plan_that_says_where_it_lives_names_it() {
        let rows = rests_on(&projected_from(FULL));
        let lives = rows.iter().find(|row| row.label == "Lives in").unwrap();
        assert_eq!(lives.domain, DomainId::Residency);
        assert!(!lives.value.is_empty());
    }
}
