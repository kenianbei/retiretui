//! The Milestones pane: the plan's key years in order, each beside the
//! item it comes from.

use retiretui_engine::plan::{Allocation, IncomeKind, Plan, TreatmentClass};
use retiretui_engine::project::{Timeline, YearRow};
use retiretui_engine::tax::{MEDICARE_AGE, rmd_start_age};

use super::rows::Entry;
use crate::commands::table::basis_amount;
use crate::commands::tui::nav::Page;
use crate::commands::tui::present::{
    account_name, compact_money, event_name, income_name, mix, residence,
};
use crate::commands::tui::session::{Projected, span};

/// The plan's milestones within its projected years, earliest first and
/// in the order their kinds are listed where two share a year.
pub(super) fn entries(projected: &Projected, nominal: bool) -> Vec<Entry> {
    let plan = &projected.plan;
    let years = &projected.projection.years;
    let timeline = Timeline::new(plan);
    let mut found: Vec<Entry> = [
        events(plan, &timeline),
        claims(plan, &timeline, years, nominal),
        incomes_starting(plan, &timeline),
        medicare(plan),
        rmds(plan),
        contributions_ending(plan, &timeline),
        moves(plan, &timeline),
        glide_steps(plan, &timeline),
    ]
    .into_iter()
    .flatten()
    .collect();
    let (first, last) = span(years);
    found.retain(|entry| {
        entry
            .year
            .is_some_and(|year| (first..=last).contains(&year))
    });
    found.sort_by_key(|entry| entry.year);
    found
}

fn events(plan: &Plan, timeline: &Timeline) -> Vec<Entry> {
    let each = plan.events.iter().enumerate();
    each.filter_map(|(at, event)| {
        let year = (*timeline.events.get(&event.id)?)?;
        let name = event_name(plan, &event.id).to_owned();
        Some(Entry::dated(year, name, (Page::Events, Some(at))))
    })
    .collect()
}

/// Each Social Security claim, with what its first full year pays: the
/// claim year is paid from the month the age is reached.
fn claims(plan: &Plan, timeline: &Timeline, years: &[YearRow], nominal: bool) -> Vec<Entry> {
    let each = plan.income.iter().enumerate();
    each.filter(|(_, income)| income.kind == IncomeKind::SocialSecurity)
        .filter_map(|(at, income)| {
            let year = timeline.income.get(&income.id)?.first()?;
            let full = years.iter().find(|row| row.year == year + 1);
            let paid = full.and_then(|row| {
                let amount = *row.income.get(&income.id)?;
                Some(basis_amount(amount, row.deflator, nominal))
            });
            let mut text = format!("Social Security · {}", plan.person_name(&income.owner));
            if let Some(paid) = paid {
                text = format!("{text}, {} a year", compact_money(paid));
            }
            Some(Entry::dated(year, text, (Page::Income, Some(at))))
        })
        .collect()
}

/// Incomes other than wages and Social Security, as they start or, once
/// only, arrive.
fn incomes_starting(plan: &Plan, timeline: &Timeline) -> Vec<Entry> {
    let each = plan.income.iter().enumerate();
    each.filter(|(_, income)| {
        !matches!(income.kind, IncomeKind::Salary | IncomeKind::SocialSecurity)
    })
    .filter_map(|(at, income)| {
        let window = timeline.income.get(&income.id)?;
        let name = income_name(plan, &income.id);
        let (year, text) = match (window.start, window.on) {
            (Some(year), _) => (year, format!("{name} starts")),
            (None, Some(year)) => (year, format!("{name} arrives")),
            (None, None) => return None,
        };
        Some(Entry::dated(year, text, (Page::Income, Some(at))))
    })
    .collect()
}

fn medicare(plan: &Plan) -> Vec<Entry> {
    let each = plan.household.people.iter().enumerate();
    each.map(|(at, person)| {
        let year = person.birth.year() + i16::from(MEDICARE_AGE);
        let text = format!("Medicare · {}", person.display_name());
        Entry::dated(year, text, (Page::People, Some(at)))
    })
    .collect()
}

/// Required distributions start for each person owning a deferred account.
fn rmds(plan: &Plan) -> Vec<Entry> {
    let each = plan.household.people.iter().enumerate();
    each.filter(|(_, person)| {
        (plan.accounts.iter()).any(|account| {
            account.owner == person.id && account.treatment() == TreatmentClass::Deferred
        })
    })
    .map(|(at, person)| {
        let birth = person.birth.year();
        let year = birth + i16::from(rmd_start_age(birth));
        let text = format!("RMDs start · {}", person.display_name());
        Entry::dated(year, text, (Page::People, Some(at)))
    })
    .collect()
}

fn contributions_ending(plan: &Plan, timeline: &Timeline) -> Vec<Entry> {
    let each = plan.contributions.iter().enumerate();
    each.filter_map(|(at, contribution)| {
        let year = timeline.contributions.get(&contribution.id)?.end?;
        let name = contribution.name.as_deref().unwrap_or(&contribution.id);
        let text = format!("{name} ends");
        Some(Entry::dated(year, text, (Page::Contributions, Some(at))))
    })
    .collect()
}

fn moves(plan: &Plan, timeline: &Timeline) -> Vec<Entry> {
    let each = plan.residency.iter().zip(&timeline.residency).enumerate();
    each.filter_map(|(at, (residency, year))| {
        let text = format!("moves to {}", residence(residency));
        Some(Entry::dated((*year)?, text, (Page::Residency, Some(at))))
    })
    .collect()
}

/// Each step of a glide path after the mix it starts on.
fn glide_steps(plan: &Plan, timeline: &Timeline) -> Vec<Entry> {
    let mut found = Vec::new();
    for (at, account) in plan.accounts.iter().enumerate() {
        let Some(years) = timeline.glide_steps.get(&account.id) else {
            continue;
        };
        let Some(Allocation::GlidePath(steps)) = &account.allocation else {
            continue;
        };
        let name = account_name(plan, &account.id);
        for (step, year) in steps.iter().zip(years).skip(1) {
            let Some(year) = *year else {
                continue;
            };
            let text = format!(
                "{name} glide step: {}",
                mix(step.stocks, step.bonds, step.cash)
            );
            found.push(Entry::dated(year, text, (Page::Accounts, Some(at))));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tui::support::{projected_from, test_projected};

    const MILESTONES_PLAN: &str = r#"
schema = 1

[plan]
name = "milestones"
start_year = 2026
horizon_age = 90
inflation = 0.025

[household]
filing = "single"

[[household.people]]
id = "me"
name = "Sam"
birth = 1970-06-15

[[residency]]
country = "us"
state = "or"

[[residency]]
country = "us"
state = "wa"
from = { event = "retire" }

[[events]]
id = "retire"
name = "Sam retires"
trigger = { age = 62, owner = "me" }

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 10000

[[accounts]]
id = "k"
name = "401(k)"
kind = "401k"
owner = "me"
balance = 500000
allocation = [
    { from = { date = 2026-01-01 }, stocks = 0.9, bonds = 0.1 },
    { from = { age = 60, owner = "me" }, stocks = 0.6, bonds = 0.4 },
]

[[income]]
id = "salary"
kind = "salary"
owner = "me"
amount = 100000
end = { event = "retire", offset = -1 }

[[income]]
id = "pension"
name = "Pension"
kind = "pension"
owner = "me"
amount = 20000
start = { event = "retire" }

[[income]]
id = "ss"
kind = "social-security"
owner = "me"
start = { age = 67, owner = "me" }

[[contributions]]
id = "deferral"
name = "Deferral"
to = "k"
amount = 20000
end = { event = "retire" }

[[expenses]]
id = "living"
amount = 60000
"#;

    fn lines(projected: &Projected, nominal: bool) -> Vec<String> {
        let entries = entries(projected, nominal);
        entries.iter().map(Entry::line).collect()
    }

    fn first_full_year(projected: &Projected, nominal: bool) -> String {
        let row = (projected.projection.years.iter())
            .find(|row| row.year == 2038)
            .unwrap();
        let paid = basis_amount(row.income["ss"], row.deflator, nominal);
        format!("2037 Social Security · Sam, {} a year", compact_money(paid))
    }

    #[test]
    fn every_kind_of_milestone_lands_in_its_year_in_order() {
        let projected = projected_from(MILESTONES_PLAN);
        assert_eq!(projected.plan.validate(), []);
        let expected = [
            "2030 401(k) glide step: 60/40".to_owned(),
            "2032 Sam retires".to_owned(),
            "2032 Pension starts".to_owned(),
            "2032 Deferral ends".to_owned(),
            "2032 moves to Washington".to_owned(),
            "2035 Medicare · Sam".to_owned(),
            first_full_year(&projected, false),
            "2045 RMDs start · Sam".to_owned(),
        ];
        assert_eq!(lines(&projected, false), expected);
        let nominal = lines(&projected, true);
        assert_eq!(nominal[6], first_full_year(&projected, true));
        assert_ne!(nominal[6], expected[6], "the benefit follows the basis");
    }

    #[test]
    fn each_milestone_opens_the_item_it_comes_from() {
        let projected = projected_from(MILESTONES_PLAN);
        let targets: Vec<_> = (entries(&projected, false).iter())
            .map(|entry| entry.opens.unwrap())
            .collect();
        let expected = [
            (Page::Accounts, Some(1)),
            (Page::Events, Some(0)),
            (Page::Income, Some(1)),
            (Page::Contributions, Some(0)),
            (Page::Residency, Some(1)),
            (Page::People, Some(0)),
            (Page::Income, Some(2)),
            (Page::People, Some(0)),
        ];
        assert_eq!(targets, expected);
    }

    #[test]
    fn a_year_past_the_projection_is_left_out() {
        let lines = lines(&test_projected(), false);
        assert_eq!(lines, ["2045 Medicare · me"], "RMDs would start in 2055");
    }
}
