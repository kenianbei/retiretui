//! The Market domain: what the market tools assume and how they run, the
//! plan's `[market]` edited as one form. Every field may be left blank for
//! the built-in default its blank names.

use retiretui_engine::plan::{Draw, Market, Plan};
use toml::{Table, Value};

use super::codec::{get_path, set_path};
use super::domain::{FieldSpec, Single};
use super::offers::Vocabulary;
use crate::commands::tui::nav::Page;

pub struct MarketSettings;

const PAIR_HELP: &str = "How the two move together, from -1 to 1. Blank takes the default.";

/// The key of the tick for `historical.wrap`, which is on by default: read
/// from the settled value, and written only where it is not the default.
const WRAPS: &str = "wraps";
const WRAP_KEY: &str = "historical.wrap";

fn draws_history(item: &Table) -> bool {
    get_path(item, "monte_carlo.draw").and_then(Value::as_str) == Some(Draw::History.as_str())
}

fn seed_wraps(item: &Table) -> Value {
    let stated = get_path(item, WRAP_KEY).and_then(Value::as_bool);
    Value::Boolean(stated.unwrap_or_else(|| Market::NONE.wrap()))
}

fn write_wraps(item: &mut Table, wraps: &Value) {
    let differs = wraps
        .as_bool()
        .filter(|&wraps| wraps != Market::NONE.wrap());
    set_path(item, WRAP_KEY, differs.map(Value::Boolean));
}

const fn pair(key: &'static str, label: &'static str) -> FieldSpec {
    FieldSpec::text(key, label).help(PAIR_HELP)
}

impl Single for MarketSettings {
    type Item = Market;
    const PAGE: Page = Page::Market;
    const PATHS: &'static [&'static str] = &["market"];
    const FIELDS: &'static [FieldSpec] = &[
        FieldSpec::money("leave_at_least", "Leave at least").help(
            "Besides never falling short, what a run must end with, in today's dollars, to count as a success. Optional.",
        ),
        FieldSpec::rate("stocks.mean", "Stocks").help("Stocks' compound yearly return, the median year's. Blank is 6%."),
        FieldSpec::rate("stocks.volatility", "  spread").help("How far a year's return strays, as a standard deviation. Blank is 16%."),
        FieldSpec::rate("bonds.mean", "Bonds").help("Bonds' compound yearly return, the median year's. Blank is 4%."),
        FieldSpec::rate("bonds.volatility", "  spread").help("How far a year's return strays, as a standard deviation. Blank is 6%."),
        FieldSpec::rate("cash.mean", "Cash").help("Cash's compound yearly return, the median year's. Blank is 2.5%."),
        FieldSpec::rate("cash.volatility", "  spread").help("How far a year's return strays, as a standard deviation. Blank is 1%."),
        FieldSpec::rate("inflation.volatility", "Inflation spread").help(
            "How far a year's inflation strays from the plan's rate, as a yearly shock. Blank is 1.5%.",
        ),
        FieldSpec::share("inflation.persistence", "  persistence").help(
            "How much of a year's stray carries into the next, so high inflation comes in runs. Blank is 60%.",
        ),
        pair("correlation.stocks_bonds", "Stocks, bonds"),
        pair("correlation.stocks_cash", "Stocks, cash"),
        pair("correlation.stocks_inflation", "Stocks, inflation"),
        pair("correlation.bonds_cash", "Bonds, cash"),
        pair("correlation.bonds_inflation", "Bonds, inflation"),
        pair("correlation.cash_inflation", "Cash, inflation"),
        FieldSpec::choice("monte_carlo.draw", "Monte Carlo draws", Vocabulary::Draw)
            .blank("The assumptions")
            .help("Random years from the assumptions above, or historical years drawn at random."),
        FieldSpec::whole("monte_carlo.trials", "  trials")
            .help("How many markets are run. Blank is 1,000."),
        FieldSpec::whole("monte_carlo.seed", "  seed")
            .help("The same seed draws the same markets, so an edit is measured against them. Blank is 42."),
        FieldSpec::whole("monte_carlo.block_years", "  years together")
            .shown_when(draws_history)
            .help("How many consecutive historical years each draw takes. Blank is 1."),
        FieldSpec::whole("historical.from", "Historical from")
            .help("The first year tried as the plan's first. Blank is the record's first, 1871."),
        FieldSpec::whole("historical.to", "  to").help("The last year tried. Blank is the record's last, 2025."),
        FieldSpec::flag(WRAPS, "  wrap").derived(seed_wraps, write_wraps).help(
            "Whether a history reaching past the record's last year goes on from its first.",
        ),
    ];

    fn get(plan: &Plan) -> Market {
        plan.market.clone().unwrap_or_default()
    }

    fn set(plan: &mut Plan, market: Market) {
        plan.market = (market != Market::default()).then_some(market);
    }
}

#[cfg(test)]
mod tests {
    use retiretui_engine::plan::AssetClass;

    use super::*;
    use crate::commands::tui::present;

    fn help_of(key: &str) -> &'static str {
        let spec = MarketSettings::FIELDS.iter().find(|spec| spec.key == key);
        spec.map(|spec| spec.help).unwrap_or_default()
    }

    #[track_caller]
    fn names_its_default(key: &str, default: &str) {
        let help = help_of(key);
        assert!(
            help.contains(&format!("Blank is {default}")),
            "{key}: {help}"
        );
    }

    #[test]
    fn each_help_names_the_default_the_engine_takes() {
        let defaults = Market::NONE;
        for &class in AssetClass::ALL {
            let name = class.as_str();
            names_its_default(
                &format!("{name}.mean"),
                &present::rate(defaults.mean(class)),
            );
            let volatility = present::rate(defaults.volatility(class));
            names_its_default(&format!("{name}.volatility"), &volatility);
        }
        names_its_default(
            "inflation.volatility",
            &present::rate(defaults.inflation_volatility()),
        );
        names_its_default(
            "inflation.persistence",
            &present::rate(defaults.inflation_persistence()),
        );
        let trials = present::money(i64::from(defaults.trials()));
        names_its_default("monte_carlo.trials", trials.trim_start_matches('$'));
        names_its_default("monte_carlo.seed", &defaults.seed().to_string());
        names_its_default(
            "monte_carlo.block_years",
            &defaults.block_years().to_string(),
        );
        assert!(help_of("historical.from").contains(&defaults.from().to_string()));
        assert!(help_of("historical.to").contains(&defaults.to().to_string()));
    }

    #[test]
    fn wrapping_reads_as_on_and_is_written_only_when_turned_off() {
        let empty = Table::new();
        assert_eq!(seed_wraps(&empty), Value::Boolean(true));
        let mut item = Table::new();
        write_wraps(&mut item, &Value::Boolean(true));
        assert!(item.is_empty(), "the default is left unsaid: {item}");
        write_wraps(&mut item, &Value::Boolean(false));
        assert_eq!(get_path(&item, WRAP_KEY), Some(&Value::Boolean(false)));
        assert_eq!(seed_wraps(&item), Value::Boolean(false));
    }

    #[test]
    fn years_drawn_together_are_asked_only_of_a_draw_from_history() {
        assert!(!draws_history(&Table::new()));
        let history: Table = "monte_carlo = { draw = \"history\" }".parse().unwrap();
        assert!(draws_history(&history));
    }
}
