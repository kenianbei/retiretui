//! Over the plan: what its years add up to, each total beside what it is
//! made of and where it leads.

use retiretui_engine::plan::{Account, Dollars, IncomeKind, Plan, TreatmentClass};
use retiretui_engine::project::YearRow;

use super::Place;
use crate::forms::DomainId;
use crate::present::{compact_money, treatment_word};
use crate::session::Projected;
use crate::table::basis_amount;

/// What the Overview titles its totals.
pub const OVER_THE_PLAN: &str = "Over the plan";

/// A tool a total leads to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    /// The Roth Conversions tool.
    RothConversions,
    /// The Withdrawal Order tool.
    WithdrawalOrder,
    /// The Tax Tables.
    TaxTables,
}

impl Tool {
    /// The tool's page as an address names it.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::RothConversions => "roth-conversions",
            Self::WithdrawalOrder => "withdrawal-order",
            Self::TaxTables => "tax-tables",
        }
    }
}

/// Where a total leads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Leads {
    /// The domain it is edited in.
    Place(Place),
    /// The tool that searches or shows it.
    Tool(Tool),
    /// The Ledger at a year.
    Year(i16),
}

/// A lifetime total.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Total {
    /// What it totals.
    pub label: &'static str,
    /// The total, compact.
    pub amount: String,
    /// What it is made of; empty where it is nothing.
    pub made_of: String,
    /// Where it leads, where it leads anywhere.
    pub leads: Option<Leads>,
}

const INCOME_KINDS: [&str; 4] = ["salary", "Social Security", "pension", "other"];
const SPENDING_KINDS: [&str; 3] = ["essential", "flexible", "one-time"];

/// An amount over the years it was more than nothing.
#[derive(Default)]
struct Spanned {
    amount: Dollars,
    years: Option<(i16, i16)>,
}

impl Spanned {
    fn add(&mut self, year: i16, amount: Dollars) {
        if amount <= 0 {
            return;
        }
        self.amount += amount;
        self.years = Some(self.years.map_or((year, year), |(first, _)| (first, year)));
    }

    /// `what` and its years: "to Roth, 2037–2044", "to Roth in 2037".
    fn said(&self, what: &str) -> String {
        match self.years {
            None => String::new(),
            Some((first, last)) if first == last => format!("{what} in {first}"),
            Some((first, last)) => format!("{what}, {first}–{last}"),
        }
    }
}

#[derive(Default)]
struct Sums {
    income: [Dollars; 4],
    /// What each class gave up, in the order of [`TreatmentClass::ALL`].
    withdrawn: [Dollars; 4],
    spent: [Dollars; 3],
    taxes: Dollars,
    state: Dollars,
    /// The year that paid the most tax, the first of them on a tie.
    heaviest: Option<(Dollars, i16)>,
    converted: Spanned,
    required: Spanned,
    medicare: Spanned,
}

impl Sums {
    fn add(&mut self, plan: &Plan, row: &YearRow, nominal: bool) {
        let dollars = |amount: Dollars| basis_amount(amount, row.deflator, nominal);
        for (id, &amount) in &row.income {
            self.income[income_kind(plan, id)] += dollars(amount);
        }
        for (id, &amount) in &row.withdrawals {
            let class = plan.account(id).map(Account::treatment);
            let mut classes = TreatmentClass::ALL.iter();
            if let Some(at) = classes.position(|&each| Some(each) == class) {
                self.withdrawn[at] += dollars(amount);
            }
        }
        let spent = [
            row.expenses_essential,
            row.expenses_flexible,
            row.expenses_once(),
        ];
        for (sum, amount) in self.spent.iter_mut().zip(spent) {
            *sum += dollars(amount);
        }
        let taxed = dollars(row.taxes.total);
        self.taxes += taxed;
        self.state += dollars(row.taxes.state);
        if taxed > 0 && self.heaviest.is_none_or(|(most, _)| taxed > most) {
            self.heaviest = Some((taxed, row.year));
        }
        self.converted.add(row.year, dollars(row.conversions));
        self.required.add(row.year, dollars(row.rmds));
        self.medicare.add(row.year, dollars(row.medicare));
    }
}

/// Where an income's kind sits in [`INCOME_KINDS`]; an income the plan does
/// not state is other.
fn income_kind(plan: &Plan, id: &str) -> usize {
    match plan.income_source(id).map(|income| income.kind) {
        Some(IncomeKind::Salary) => 0,
        Some(IncomeKind::SocialSecurity) => 1,
        Some(IncomeKind::Pension) => 2,
        _ => 3,
    }
}

/// Each part that is more than nothing as its whole percent of them all:
/// "salary 61% · pension 39%".
fn shares<'a>(parts: impl IntoIterator<Item = (&'a str, Dollars)> + Clone) -> String {
    let whole: Dollars = parts.clone().into_iter().map(|(_, amount)| amount).sum();
    if whole <= 0 {
        return String::new();
    }
    let said: Vec<String> = parts
        .into_iter()
        .filter(|&(_, amount)| amount > 0)
        .map(|(name, amount)| format!("{name} {}%", (amount * 100 + whole / 2) / whole))
        .collect();
    said.join(" · ")
}

/// Federal and state tax, and the year that paid the most.
fn taxes_said(sums: &Sums) -> String {
    let mut said = Vec::new();
    let federal = sums.taxes - sums.state;
    for (name, amount) in [("federal", federal), ("state", sums.state)] {
        if amount > 0 {
            said.push(format!("{name} {}", compact_money(amount)));
        }
    }
    if let Some((_, year)) = sums.heaviest {
        said.push(format!("most in {year}"));
    }
    said.join(" · ")
}

impl Total {
    fn new(label: &'static str, amount: Dollars, made_of: String, leads: Option<Leads>) -> Self {
        Self {
            label,
            amount: compact_money(amount),
            made_of,
            leads,
        }
    }
}

impl Sums {
    /// What came in, was drawn, was spent and was paid in tax.
    fn flows(&self) -> [Total; 4] {
        let domain = |domain| Some(Leads::Place((domain, None)));
        let classes = TreatmentClass::ALL.iter().copied().map(treatment_word);
        [
            Total::new(
                "Income",
                self.income.iter().sum(),
                shares(INCOME_KINDS.into_iter().zip(self.income)),
                domain(DomainId::Income),
            ),
            Total::new(
                "Withdrawals",
                self.withdrawn.iter().sum(),
                shares(classes.zip(self.withdrawn)),
                Some(Leads::Tool(Tool::WithdrawalOrder)),
            ),
            Total::new(
                "Spending",
                self.spent.iter().sum(),
                shares(SPENDING_KINDS.into_iter().zip(self.spent)),
                domain(DomainId::Expenses),
            ),
            Total::new(
                "Taxes",
                self.taxes,
                taxes_said(self),
                Some(Leads::Tool(Tool::TaxTables)),
            ),
        ]
    }

    /// What was converted, what the law required drawn, and what Medicare
    /// charged on top.
    fn moved(&self) -> [Total; 3] {
        let required = self.required.years.map(|(first, _)| first);
        [
            Total::new(
                "Converted",
                self.converted.amount,
                self.converted.said("to Roth"),
                Some(Leads::Tool(Tool::RothConversions)),
            ),
            Total::new(
                "Required",
                self.required.amount,
                required.map_or_else(String::new, |year| format!("distributions from {year}")),
                required.map(Leads::Year),
            ),
            Total::new(
                "Medicare",
                self.medicare.amount,
                self.medicare.said("surcharges"),
                Some(Leads::Place((DomainId::Household, None))),
            ),
        ]
    }
}

/// What the plan takes in, draws, spends and pays over all its years, in
/// nominal dollars or today's. Withdrawals are everything drawn from the
/// accounts, required distributions among them, and no conversion.
#[must_use]
pub fn totals(projected: &Projected, nominal: bool) -> Vec<Total> {
    let mut sums = Sums::default();
    for row in &projected.projection.years {
        sums.add(&projected.plan, row, nominal);
    }
    sums.flows().into_iter().chain(sums.moved()).collect()
}

#[cfg(test)]
mod tests {
    use retiretui_engine::project::Projection;

    use super::super::tests::{TEST_PLAN, projected_from, test_projected};
    use super::*;

    const FULL: &str = include_str!("../../../retiretui_engine/tests/fixtures/full.toml");
    const SPENDER: &str =
        include_str!("../../../retiretui_engine/tests/fixtures/spending-plan.toml");
    const CONVERTING: &str = "\n[[accounts]]\nid = \"r\"\nkind = \"ira\"\nroth = true\nowner = \"me\"\nbalance = 0\n\n[[conversions]]\nid = \"early\"\nfrom = \"k\"\nto = \"r\"\namount = 10000\ncola = false\nend = { date = 2028-12-31 }\n";

    fn amounts(projected: &Projected, nominal: bool) -> Vec<(&'static str, String)> {
        let found = totals(projected, nominal);
        found.into_iter().map(|it| (it.label, it.amount)).collect()
    }

    fn summed(projection: &Projection, of: impl Fn(&YearRow) -> Dollars) -> String {
        compact_money(projection.years.iter().map(of).sum())
    }

    #[test]
    fn each_total_is_the_sum_of_its_years() {
        let projected = projected_from(FULL);
        let projection = &projected.projection;
        assert_eq!(
            amounts(&projected, true),
            [
                ("Income", summed(projection, |row| row.total_income)),
                (
                    "Withdrawals",
                    summed(projection, YearRow::total_withdrawals)
                ),
                ("Spending", summed(projection, |row| row.expenses)),
                ("Taxes", summed(projection, |row| row.taxes.total)),
                ("Converted", summed(projection, |row| row.conversions)),
                ("Required", summed(projection, |row| row.rmds)),
                ("Medicare", summed(projection, |row| row.medicare)),
            ]
        );
    }

    #[test]
    fn todays_dollars_deflate_each_year_before_summing() {
        let projected = projected_from(FULL);
        let today = |of: fn(&YearRow) -> Dollars| {
            let years = projected.projection.years.iter();
            compact_money(
                years
                    .map(|row| basis_amount(of(row), row.deflator, false))
                    .sum(),
            )
        };
        let found = amounts(&projected, false);
        assert_eq!(
            found,
            [
                ("Income", today(|row| row.total_income)),
                ("Withdrawals", today(YearRow::total_withdrawals)),
                ("Spending", today(|row| row.expenses)),
                ("Taxes", today(|row| row.taxes.total)),
                ("Converted", today(|row| row.conversions)),
                ("Required", today(|row| row.rmds)),
                ("Medicare", today(|row| row.medicare)),
            ]
        );
        let nominal = amounts(&projected, true);
        let differing = found
            .iter()
            .zip(&nominal)
            .filter(|(today, nominal)| today != nominal);
        assert!(differing.count() >= 4, "{found:?} against {nominal:?}");
    }

    #[test]
    fn withdrawals_hold_what_the_law_required_and_no_conversion() {
        let projected = projected_from(FULL);
        let found = totals(&projected, true);
        let years = &projected.projection.years;
        let required: Dollars = years.iter().map(|row| row.rmds).sum();
        let withdrawn: Dollars = years.iter().map(YearRow::total_withdrawals).sum();
        assert!(
            required > 0 && withdrawn > required,
            "{required} of {withdrawn}"
        );
        let first = years.iter().find(|row| row.rmds > 0).unwrap().year;
        assert_eq!(found[5].made_of, format!("distributions from {first}"));
        assert_eq!(found[5].leads, Some(Leads::Year(first)));

        let converting = projected_from(&format!("{TEST_PLAN}{CONVERTING}"));
        let years = &converting.projection.years;
        let converted: Vec<&YearRow> = years.iter().filter(|row| row.conversions > 0).collect();
        assert_eq!(converted.len(), 3);
        assert!(converted.iter().all(|row| row.total_withdrawals() == 0));
        let found = totals(&converting, true);
        assert_eq!(found[4].amount, "$30k");
        assert_eq!(found[4].made_of, "to Roth, 2026–2028");
    }

    #[test]
    fn a_total_says_what_it_is_made_of_and_leaves_out_what_is_nothing() {
        let found = totals(&test_projected(), true);
        assert_eq!(found[0].made_of, "salary 100%");
        assert_eq!(found[2].made_of, "flexible 100%");
        assert_eq!(found[5].amount, "$0");
        assert_eq!((found[5].made_of.as_str(), found[5].leads), ("", None));
        assert_eq!(found[6].made_of, "");

        let spender = totals(&projected_from(SPENDER), true);
        let kinds: Vec<&str> = spender[2].made_of.split(" · ").collect();
        assert_eq!(kinds.len(), 3, "{}", spender[2].made_of);
        assert!(kinds[0].starts_with("essential ") && kinds[2].starts_with("one-time "));
    }

    #[test]
    fn taxes_name_who_was_paid_and_the_heaviest_year() {
        let projected = projected_from(FULL);
        let years = &projected.projection.years;
        let heaviest = years
            .iter()
            .rev()
            .max_by_key(|row| row.taxes.total)
            .unwrap();
        let said = &totals(&projected, true)[3].made_of;
        assert!(said.starts_with("federal $"), "{said}");
        assert!(said.contains(" · state $"), "{said}");
        assert!(
            said.ends_with(&format!("most in {}", heaviest.year)),
            "{said}"
        );
    }

    #[test]
    fn each_total_leads_to_where_it_is_edited_searched_or_shown() {
        let leads: Vec<Option<Leads>> = totals(&projected_from(FULL), true)
            .into_iter()
            .map(|it| it.leads)
            .collect();
        assert_eq!(leads[0], Some(Leads::Place((DomainId::Income, None))));
        assert_eq!(leads[1], Some(Leads::Tool(Tool::WithdrawalOrder)));
        assert_eq!(leads[2], Some(Leads::Place((DomainId::Expenses, None))));
        assert_eq!(leads[3], Some(Leads::Tool(Tool::TaxTables)));
        assert_eq!(leads[4], Some(Leads::Tool(Tool::RothConversions)));
        assert_eq!(leads[6], Some(Leads::Place((DomainId::Household, None))));
    }

    #[test]
    fn shares_round_to_whole_percents_of_what_is_there() {
        assert_eq!(shares([("a", 610), ("b", 0), ("c", 390)]), "a 61% · c 39%");
        assert_eq!(shares([("a", 0), ("b", 0)]), "");
    }

    #[test]
    fn a_span_of_one_year_names_it() {
        let mut once = Spanned::default();
        once.add(2030, 0);
        assert_eq!(once.said("to Roth"), "");
        once.add(2030, 5);
        assert_eq!(once.said("to Roth"), "to Roth in 2030");
    }
}
