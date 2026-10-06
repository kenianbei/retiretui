use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Dollars;
use retiretui_engine::project::Action;

use super::*;
use crate::actions::collect_warnings;
use crate::overview::tests::{TEST_PLAN, projected_from};
use crate::overview::{milestones, totals};
use crate::present::{money, parse_money};
use crate::session::Projected;
use crate::setup::EXAMPLES;
use crate::tax_tables::{TablesView, year_tables};

const FULL: &str = include_str!("../../../retiretui_engine/tests/fixtures/full.toml");
const SPENDER: &str = include_str!("../../../retiretui_engine/tests/fixtures/spending-plan.toml");
/// Spending the test plan's accounts cannot cover once the salary ends.
const OVERSPENT: &str = "\n[[expenses]]\nid = \"yacht\"\namount = 400000\n";
const CONVERTING: &str = "\n[[accounts]]\nid = \"r\"\nkind = \"ira\"\nroth = true\nowner = \"me\"\nbalance = 0\n\n[[conversions]]\nid = \"early\"\nfrom = \"k\"\nto = \"r\"\namount = 10000\ncola = false\nend = { date = 2028-12-31 }\n";

pub(super) fn full() -> Projected {
    projected_from(FULL)
}

/// Every plan the Ledger is checked over: the fixtures and each example.
pub(super) fn plans() -> Vec<Projected> {
    let examples = EXAMPLES.iter().map(|&(_, _, text)| text);
    let texts = [FULL, SPENDER].into_iter().chain(examples);
    let overspent = format!("{TEST_PLAN}{OVERSPENT}");
    let mut plans: Vec<Projected> = texts.map(projected_from).collect();
    plans.push(projected_from(&overspent));
    plans
}

fn year_of(projected: &Projected, year: i16, is_nominal: bool) -> Year {
    let asked = Asked {
        year,
        is_nominal,
        is_run: false,
    };
    Year::new(projected, &TaxTables::embedded(), asked).expect("a projected year")
}

fn dollars(line: &DetailLine) -> Dollars {
    parse_money(&line.amount).unwrap_or_else(|| panic!("{line:?} is not money"))
}

/// What a side's lines add up to.
fn added(funds: &Funds) -> Dollars {
    funds.lines.iter().map(dollars).sum()
}

fn labels(lines: &[DetailLine]) -> Vec<&str> {
    lines.iter().map(|line| line.label.as_str()).collect()
}

#[test]
fn every_account_holding_money_closes_on_it() {
    let projected = full();
    let years = &projected.projection.years;
    for (at, row) in years.iter().enumerate() {
        let previous = at.checked_sub(1).map(|before| &years[before]);
        let flows = account_flows(&projected.plan, previous, row, true);
        let closes = flows.iter().filter(|flow| flow.close != money(0));
        let held = row.balances.values().filter(|&&balance| balance != 0);
        assert_eq!(closes.count(), held.count(), "{}", row.year);
    }
}

#[test]
fn what_a_year_lived_on_is_what_it_spent_paid_and_put_away() {
    let mut shortfalls = 0;
    for projected in plans() {
        let name = projected.plan.plan.name.as_deref().unwrap_or("unnamed");
        for row in &projected.projection.years {
            let nominal = year_of(&projected, row.year, true);
            assert_eq!(
                nominal.money_in.total, nominal.money_out.total,
                "{name} {}",
                row.year
            );
            for side in [&nominal.money_in, &nominal.money_out] {
                assert_eq!(added(side), dollars(&side.total), "{name} {}", row.year);
            }
            let todays = year_of(&projected, row.year, false);
            assert_eq!(
                todays.money_in.total, todays.money_out.total,
                "{name} {} in today's dollars",
                row.year
            );
            let is_short = labels(&nominal.money_in.lines).contains(&"Unfunded");
            assert_eq!(is_short, row.unfunded > 0, "{name} {}", row.year);
            shortfalls += usize::from(is_short);
        }
    }
    assert!(shortfalls > 0, "no plan checked ran short");
}

#[test]
fn withdrawals_are_listed_by_account_and_a_required_one_is_named() {
    let mut required = 0;
    for projected in plans() {
        for row in &projected.projection.years {
            let year = year_of(&projected, row.year, true);
            let lines = year.money_in.lines.iter();
            let drawn: Vec<&DetailLine> = lines
                .filter(|line| line.label.starts_with("From ") || line.label.starts_with("RMD "))
                .collect();
            let listed: Dollars = drawn.iter().map(|line| dollars(line)).sum();
            assert_eq!(listed, row.total_withdrawals(), "{}", row.year);
            let named = drawn
                .iter()
                .filter(|line| line.label.starts_with("RMD from "));
            let rmds = row.actions.iter();
            let rmds = rmds.filter(|action| matches!(action, Action::Rmd { .. }));
            assert_eq!(named.count(), rmds.count(), "{}", row.year);
            required += usize::from(row.rmds > 0);
        }
    }
    assert!(required > 0, "no plan checked took a required distribution");
}

#[test]
fn every_expense_is_listed_over_what_the_spending_comes_to_by_kind() {
    let split = projected_from(SPENDER);
    let row = (split.projection.years.iter())
        .find(|row| row.expenses_essential > 0 && row.expenses_once() > 0)
        .expect("the year the roof is paid for");
    let out = year_of(&split, row.year, true).money_out;
    let listed = labels(&out.lines);
    assert_eq!(
        listed[..4],
        ["Living expenses", "mortgage", "roof", "travel"],
        "the plan's order"
    );
    let spent: Dollars = out.lines[..4].iter().map(dollars).sum();
    assert_eq!(spent, row.expenses);
    assert_eq!(labels(&out.sums), ["Essential", "Flexible", "One-time"]);
    let kinds = [
        row.expenses_essential,
        row.expenses_flexible,
        row.expenses_once(),
    ];
    assert_eq!(out.sums.iter().map(dollars).collect::<Vec<_>>(), kinds);
    let plain = projected_from(TEST_PLAN);
    let working = year_of(&plain, 2026, true).money_out;
    assert_eq!(labels(&working.lines)[..2], ["living", "Tax"]);
    assert_eq!(labels(&working.sums), ["Spending"], "of one kind");
}

#[test]
fn income_and_withdrawals_are_one_list_over_what_each_comes_to() {
    let projected = full();
    let row = projected.projection.row(2045).unwrap();
    let lived = year_of(&projected, 2045, true).money_in;
    assert_eq!(
        labels(&lived.lines),
        ["db-pension", "ss-jordan", "From brokerage", "From fid-401k"]
    );
    assert_eq!(labels(&lived.sums), ["Income", "Withdrawn"]);
    let sums: Vec<Dollars> = lived.sums.iter().map(dollars).collect();
    assert_eq!(sums, [row.total_income, row.total_withdrawals()]);
    let working = year_of(&projected, 2026, true).money_in;
    assert_eq!(labels(&working.lines), ["salary"]);
    assert_eq!(working.sums, [], "one sum would only repeat the total");
    assert_eq!(working.total.label, "Total");
}

#[test]
fn a_year_s_tax_lists_what_was_paid_and_ends_on_all_of_it() {
    let projected = full();
    for row in &projected.projection.years {
        let tax = year_of(&projected, row.year, true).tax;
        let (total, kinds) = tax.split_last().expect("a total");
        assert_eq!(
            (total.label.as_str(), dollars(total)),
            ("Total", row.taxes.total)
        );
        assert_eq!(kinds.iter().map(dollars).sum::<Dollars>(), row.taxes.total);
        assert!(kinds.iter().all(|line| dollars(line) != 0), "{tax:?}");
    }
}

#[test]
fn the_bracket_is_the_one_the_tax_tables_hold_for_the_taxable_income() {
    let projected = full();
    let tables = TaxTables::embedded();
    let mut reached = std::collections::BTreeSet::new();
    for row in &projected.projection.years {
        let view = TablesView {
            year: row.year,
            ..TablesView::default()
        };
        let sections = year_tables(&projected.plan, &tables, &view).sections;
        let brackets = &sections[0];
        assert_eq!(brackets.title, "Income tax brackets");
        let taxable = row.taxes.ordinary_taxable;
        let over = |cells: &Vec<String>| parse_money(&cells[0]).unwrap();
        let at = brackets
            .rows
            .iter()
            .rposition(|cells| over(cells) <= taxable);
        let held = &brackets.rows[at.expect("a bracket from nothing")];
        let said = year_of(&projected, row.year, true)
            .bracket
            .expect("a bracket");
        let above = brackets.rows.get(at.unwrap() + 1).map(over);
        let wanted = above.map_or_else(
            || ("Top bracket".to_owned(), held[1].clone()),
            |above| {
                let label = format!("To top of {}", held[1]);
                (label, money(above - taxable))
            },
        );
        assert_eq!((said.label.clone(), said.amount), wanted, "{}", row.year);
        reached.insert(said.label);
    }
    assert!(reached.len() > 1, "one bracket throughout: {reached:?}");
    let run = Asked {
        year: 2030,
        is_nominal: true,
        is_run: true,
    };
    let of_run = Year::new(&projected, &tables, run).unwrap();
    assert_eq!(of_run.bracket, None, "a run's tables are not the plan's");
}

#[test]
fn tax_over_magi_is_said_only_of_a_year_with_magi() {
    let projected = full();
    for row in &projected.projection.years {
        let picture = year_of(&projected, row.year, true).picture;
        let has_rate = picture.iter().any(|line| line.label == "Tax over MAGI");
        assert_eq!(has_rate, row.taxes.magi > 0, "{}", row.year);
    }
    let mut none = full();
    none.projection.years[0].taxes.magi = 0;
    let picture = year_of(&none, 2026, true).picture;
    assert!(
        picture.iter().all(|line| !line.label.contains("MAGI")),
        "{picture:?}"
    );
}

#[test]
fn every_account_as_one_opens_and_closes_on_the_net_worth() {
    let projected = full();
    let years = &projected.projection.years;
    let mut converted = 0;
    for pair in years.windows(2) {
        let (before, row) = (&pair[0], &pair[1]);
        let year = year_of(&projected, row.year, true);
        let together = year.all_accounts.expect("several accounts");
        let figure = |said: &str| parse_money(said.trim_start_matches('+')).unwrap();
        assert_eq!(figure(&together.open), before.net_worth, "{}", row.year);
        assert_eq!(figure(&together.close), row.net_worth, "{}", row.year);
        let moved: Dollars = together.moves.iter().map(|each| figure(each)).sum();
        let grown = if together.growth.is_empty() {
            0
        } else {
            figure(&together.growth)
        };
        assert_eq!(
            figure(&together.open) + moved + grown,
            row.net_worth,
            "{} adds up",
            row.year
        );
        if row.conversions > 0 {
            converted += 1;
            let both = year
                .flows
                .iter()
                .filter(|flow| flow.moves.iter().any(|each| each.contains("(conversion)")));
            assert_eq!(both.count(), 2, "{}: {:?}", row.year, year.flows);
            let said = together.moves.join(" ");
            assert!(!said.contains("conversion"), "{said}");
        }
    }
    assert!(converted > 0, "the fixture converts");
    let alone = "[[accounts]]\nid = \"k\"";
    let one_account = TEST_PLAN.split(alone).next().unwrap();
    let one_account = format!(
        "{one_account}[[income]]{}",
        TEST_PLAN.split("[[income]]").nth(1).unwrap()
    );
    let single = projected_from(&one_account);
    assert_eq!(year_of(&single, 2026, true).all_accounts, None);
}

#[test]
fn an_account_s_moves_are_what_came_in_and_then_what_went_out() {
    let projected = full();
    let year = year_of(&projected, 2037, true);
    assert!(
        year.flows.iter().any(|flow| flow.moves.len() > 1),
        "{:?}",
        year.flows
    );
    for flow in &year.flows {
        let went = flow.moves.iter().skip_while(|moved| moved.starts_with('+'));
        assert!(went.clone().all(|moved| moved.starts_with('-')), "{flow:?}");
        assert_eq!(
            flow.growth_rate.is_empty(),
            flow.growth.is_empty(),
            "{flow:?}"
        );
    }
    let grew = year.flows.iter().find(|flow| flow.account == "brokerage");
    assert_eq!(grew.map(|flow| flow.growth_rate.as_str()), Some("5.0%"));
}

#[test]
fn the_last_year_has_come_as_far_as_the_overview_s_totals() {
    for is_nominal in [true, false] {
        let projected = full();
        let last = projected.projection.years.last().unwrap().year;
        let total = |label: &str| {
            let found = totals(&projected, is_nominal);
            let found = found.into_iter().find(|each| each.label == label);
            found.unwrap_or_else(|| panic!("no {label} total")).amount
        };
        let (taxes, converted, drawn) = (total("Taxes"), total("Converted"), total("Withdrawals"));
        let wanted = format!(
            "So far: {taxes} of {taxes} taxes · {converted} of {converted} converted · {drawn} of {drawn} withdrawn"
        );
        assert_eq!(year_of(&projected, last, is_nominal).so_far, Some(wanted));
        let first = year_of(&projected, 2026, is_nominal).so_far.unwrap();
        assert!(first.contains(&format!("of {taxes} taxes")), "{first}");
        assert!(first.contains("$0 of"), "nothing converted yet: {first}");
    }
    let unconverted = projected_from(TEST_PLAN);
    let said = year_of(&unconverted, 2030, true).so_far.unwrap();
    assert!(!said.contains("converted"), "{said}");
    let converting = projected_from(&format!("{TEST_PLAN}{CONVERTING}"));
    let said = year_of(&converting, 2027, true).so_far.unwrap();
    assert!(said.contains("$20k of $30k converted"), "{said}");
}

#[test]
fn a_year_says_who_turns_what_its_milestones_and_what_to_do() {
    let projected = full();
    let tables = TaxTables::embedded();
    let dated = milestones(&projected, true);
    let mut with_milestone = 0;
    for row in &projected.projection.years {
        let year = year_of(&projected, row.year, true);
        let wanted: Vec<&str> = (dated.iter().filter(|each| each.year == Some(row.year)))
            .map(|each| each.text.as_str())
            .collect();
        assert_eq!(year.milestones, wanted, "{}", row.year);
        with_milestone += usize::from(!wanted.is_empty());
        assert!(!year.to_do.is_empty(), "{}", row.year);
        let warnings = collect_warnings(&projected.plan, &tables, row, None);
        assert_eq!(year.warnings, warnings, "{}", row.year);
    }
    assert!(with_milestone > 1);
    assert_eq!(
        year_of(&projected, 2026, true).ages,
        "jordan turns 51 · alex turns 47"
    );
    let mut quiet = full();
    quiet.projection.years[0].actions.clear();
    assert_eq!(
        year_of(&quiet, 2026, true).to_do,
        ["Nothing to do this year."]
    );
    let outside = Asked {
        year: 1999,
        is_nominal: true,
        is_run: false,
    };
    assert_eq!(Year::new(&projected, &tables, outside), None);
}

#[test]
fn a_history_charts_two_figures_of_every_year_on_the_basis_asked() {
    let projected = full();
    let projection = &projected.projection;
    let titles: Vec<&str> = History::ALL.iter().map(|each| each.title()).collect();
    assert_eq!(
        titles,
        ["Money in by year", "Money out by year", "Tax by year"]
    );
    let row = projection.row(2045).unwrap();
    let wanted = [
        (
            History::MoneyIn,
            ["Income", "Withdrawn"],
            [row.total_income, row.total_withdrawals()],
        ),
        (
            History::MoneyOut,
            ["Spending", "Tax"],
            [row.expenses, row.taxes.total],
        ),
        (
            History::Tax,
            ["MAGI", "Taxable income"],
            [row.taxes.magi, row.taxes.ordinary_taxable],
        ),
    ];
    for (history, lines, figures) in wanted {
        assert_eq!(history.lines(), lines);
        let nominal = history.points(projection, true);
        assert_eq!(nominal.len(), projection.years.len());
        let at = nominal.iter().find(|&&(year, _)| year == 2045).unwrap();
        assert_eq!(at.1, figures, "{history:?}");
        let todays = history.points(projection, false);
        let later = todays.iter().find(|&&(year, _)| year == 2045).unwrap();
        assert!(later.1[0] < figures[0], "{history:?} follows the basis");
        assert_eq!(todays[0], nominal[0], "the first year is today's");
    }
}
