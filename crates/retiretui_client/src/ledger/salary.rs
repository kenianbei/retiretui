//! The years each earner's salary ends, as a chart marks them.

use std::collections::BTreeMap;

use retiretui_engine::plan::IncomeKind;

use crate::session::Projected;

/// Each earner with the first year none of their salaries pays, earliest
/// first and then by owner; an earner paid to the horizon has none.
#[must_use]
pub fn salary_ends(projected: &Projected) -> Vec<(&str, i16)> {
    let years = &projected.projection.years;
    let mut last_paid: BTreeMap<&str, i16> = BTreeMap::new();
    let salaries = projected.plan.income.iter();
    for income in salaries.filter(|income| income.kind == IncomeKind::Salary) {
        let Some(row) = years
            .iter()
            .rev()
            .find(|row| row.income.get(&income.id).is_some_and(|&amount| amount > 0))
        else {
            continue;
        };
        last_paid
            .entry(&income.owner)
            .and_modify(|last| *last = (*last).max(row.year))
            .or_insert(row.year);
    }
    let horizon = years.last().map(|row| row.year);
    let mut ends: Vec<_> = last_paid
        .into_iter()
        .filter(|&(_, year)| Some(year) != horizon)
        .map(|(owner, year)| (owner, year + 1))
        .collect();
    ends.sort_by_key(|&(_, year)| year);
    ends
}

/// [`salary_ends`] as a chart marks them: "retire" where one earner has
/// an end, each earner's name where several do.
#[must_use]
pub fn salary_marks(projected: &Projected) -> Vec<(String, i16)> {
    let ends = salary_ends(projected);
    let is_only_earner = ends.len() == 1;
    ends.into_iter()
        .map(|(owner, year)| {
            let label = if is_only_earner {
                format!("retire {year}")
            } else {
                format!("{} {year}", projected.plan.person_name(owner))
            };
            (label, year)
        })
        .collect()
}
