use std::collections::BTreeMap;

use crate::params::TaxTables;
use crate::plan::{Account, ColaSpec, Dollars, Plan, Span};

use super::collect::{default_cliff_end, seed_magi_lookback};
use super::invest::Holding;
use super::resolve::Resolver;
use super::{Action, ClassTotals, MarketPath, Projection, Taxes, YearRow, horizon_year};

pub(crate) struct Simulation<'a> {
    pub(super) plan: &'a Plan,
    pub(super) tables: &'a TaxTables,
    pub(super) path: &'a MarketPath,
    pub(super) resolver: Resolver,
    /// How each account's return is found, by account index.
    pub(super) holdings: Vec<Holding>,
    pub(super) index_of: BTreeMap<&'a str, usize>,
    pub(super) start_year: i16,
    pub(super) end_year: i16,
    pub(super) balances: Vec<Dollars>,
    pub(super) bases: Vec<Dollars>,
    pub(super) transfer_done: Vec<bool>,
    pub(super) conversion_done: Vec<bool>,
    pub(super) surplus_index: Option<usize>,
    /// `[MAGI(year - 2), MAGI(year - 1)]` for the year being stepped.
    pub(super) magi_lookback: [Dollars; 2],
    pub(super) default_cliff_end: i16,
    /// Salary paid so far by owner id and year, nominal: the covered
    /// earnings a computed Social Security benefit extends the owner's
    /// record with.
    pub(super) covered: BTreeMap<&'a str, BTreeMap<i16, Dollars>>,
    /// The computed Social Security benefit per income, in start-year
    /// dollars of the income's own escalation, once its claim year has
    /// been walked.
    pub(super) benefits: Vec<Option<Dollars>>,
}

#[derive(Default)]
pub(super) struct YearAcc {
    pub(super) income: BTreeMap<String, Dollars>,
    pub(super) cash: Dollars,
    pub(super) ordinary: Dollars,
    pub(super) ss_gross: Dollars,
    pub(super) gains: Dollars,
    pub(super) penalty_base: Dollars,
    pub(super) expenses: Dollars,
    pub(super) medicare: Dollars,
    pub(super) employee: Dollars,
    pub(super) employer: Dollars,
    pub(super) rmds: Dollars,
    pub(super) conversions: Dollars,
    pub(super) withdrawals: BTreeMap<String, Dollars>,
    pub(super) growth: BTreeMap<String, Dollars>,
    pub(super) drained_cash: Dollars,
    pub(super) pending: Vec<Dollars>,
    pub(super) surplus: Dollars,
    pub(super) unfunded: Dollars,
    pub(super) cliffs: Dollars,
    /// Traditional IRA contributions by account index whose deduction the
    /// year's MAGI decides.
    pub(super) ira_to_settle: Vec<(usize, Dollars)>,
    pub(super) actions: Vec<Action>,
    /// Settle-loop drains per account index, in first-drain order.
    pub(super) funding: Vec<(usize, Dollars)>,
}

impl<'a> Simulation<'a> {
    pub(crate) fn new(plan: &'a Plan, tables: &'a TaxTables, path: &'a MarketPath) -> Self {
        let index_of: BTreeMap<&str, usize> = plan
            .accounts
            .iter()
            .enumerate()
            .map(|(i, account)| (account.id.as_str(), i))
            .collect();
        let surplus_index = plan
            .surplus_account()
            .and_then(|id| index_of.get(id).copied());
        let resolver = Resolver::new(plan);
        let holdings = plan
            .accounts
            .iter()
            .map(|account| Holding::of(plan, &resolver, account))
            .collect();
        Self {
            plan,
            tables,
            path,
            resolver,
            holdings,
            index_of,
            start_year: plan.plan.start_year,
            end_year: horizon_year(plan),
            balances: plan
                .accounts
                .iter()
                .map(|account| account.balance)
                .collect(),
            bases: plan.accounts.iter().map(Account::starting_basis).collect(),
            transfer_done: vec![false; plan.transfers.len()],
            conversion_done: vec![false; plan.conversions.len()],
            surplus_index,
            magi_lookback: seed_magi_lookback(plan),
            default_cliff_end: default_cliff_end(plan),
            covered: BTreeMap::new(),
            benefits: vec![None; plan.income.len()],
        }
    }

    pub(crate) fn run(mut self) -> Projection {
        let years = (self.start_year..=self.end_year)
            .map(|year| self.step(year))
            .collect();
        Projection { years }
    }

    fn step(&mut self, year: i16) -> YearRow {
        let factor = self.path.deflator(year);
        let params = self.tables.params_for(year, self.path.inflation());
        let mut acc = YearAcc {
            pending: vec![0; self.plan.accounts.len()],
            ..YearAcc::default()
        };
        let snapshot = self.balances.clone();
        self.grow(year, &snapshot, &mut acc);
        self.execute_transfers(year, factor, &mut acc);
        self.take_rmds(year, &params, &snapshot, &mut acc);
        self.collect_income(year, &mut acc);
        self.collect_expenses(year, &mut acc);
        self.collect_medicare(year, &params, &mut acc);
        self.collect_contributions(year, &params, &mut acc);
        self.execute_conversions(year, &mut acc);
        let taxes = self.settle_cash(year, &params, &mut acc);
        self.settle_ira_bands(&params, &taxes, &mut acc);
        self.magi_lookback = [self.magi_lookback[1], taxes.magi];
        self.sweep_surplus(&mut acc);
        self.land(&acc);
        self.build_row(year, factor, taxes, acc)
    }

    pub(super) fn cola_factor(&self, cola: ColaSpec, year: i16) -> f64 {
        cola.factor(self.path.deflator(year), i32::from(year - self.start_year))
    }

    fn sweep_surplus(&mut self, acc: &mut YearAcc) {
        if acc.surplus <= 0 {
            return;
        }
        let Some(index) = self.surplus_index else {
            return;
        };
        acc.pending[index] += acc.surplus;
        if self.plan.accounts[index].kind.tracks_basis() {
            self.bases[index] += acc.surplus;
        }
    }

    /// What the year paid in - contributions, the surplus - lands at its
    /// close and earns nothing until next year.
    fn land(&mut self, acc: &YearAcc) {
        for (balance, pending) in self.balances.iter_mut().zip(&acc.pending) {
            *balance += pending;
        }
    }

    /// The year's actions in the order they ran: what the walk recorded,
    /// then the drains that funded it, then the surplus swept.
    fn executed(&self, acc: &mut YearAcc) -> Vec<Action> {
        let mut actions = std::mem::take(&mut acc.actions);
        for &(index, amount) in &acc.funding {
            actions.push(Action::Withdrawal {
                account: self.plan.accounts[index].id.clone(),
                amount,
            });
        }
        if let Some(index) = self.surplus_index
            && acc.surplus > 0
        {
            actions.push(Action::Surplus {
                account: self.plan.accounts[index].id.clone(),
                amount: acc.surplus,
            });
        }
        actions
    }

    fn build_row(&self, year: i16, factor: f64, taxes: Taxes, mut acc: YearAcc) -> YearRow {
        let ages = self
            .plan
            .household
            .people
            .iter()
            .map(|person| {
                (
                    person.id.clone(),
                    person.age_in_year(year).clamp(0, 255) as u8,
                )
            })
            .collect();
        let mut class_totals = ClassTotals::default();
        let mut balances = BTreeMap::new();
        for (index, account) in self.plan.accounts.iter().enumerate() {
            class_totals.add(account.treatment(), self.balances[index]);
            balances.insert(account.id.clone(), self.balances[index]);
        }
        let net_worth = self.balances.iter().sum();
        let actions = self.executed(&mut acc);
        YearRow {
            year,
            ages,
            deflator: factor,
            total_income: acc.income.values().sum(),
            income: acc.income,
            expenses: acc.expenses,
            contributions_employee: acc.employee,
            contributions_employer: acc.employer,
            rmds: acc.rmds,
            conversions: acc.conversions,
            withdrawals: acc.withdrawals,
            growth: acc.growth,
            actions,
            medicare: acc.medicare + acc.cliffs,
            taxes,
            surplus: acc.surplus,
            unfunded: acc.unfunded,
            balances,
            class_totals,
            net_worth,
        }
    }

    pub(super) fn item_active(&self, year: i16, span: Span<'_>) -> bool {
        self.resolver.is_active(self.plan, year, span)
    }
}
