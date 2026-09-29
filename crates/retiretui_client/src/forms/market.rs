//! The Market domain: what the market tools assume and how they run, the
//! plan's `[market]` edited as one form. Every field may be left blank for
//! the built-in default its blank names.

use retiretui_engine::plan::{Draw, Market, Plan};
use toml::{Table, Value};

use super::offers::Vocabulary;
use super::{DomainId, FieldSpec, Single};
use crate::codec::{get_path, set_path};

/// What the market tools assume, edited as one form.
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

/// The headings the Market's fields are gathered under.
const SUCCESS: &str = "Success";
const STOCKS: &str = "Stocks";
const BONDS: &str = "Bonds";
const CASH: &str = "Cash";
const INFLATION: &str = "Inflation";
const CORRELATIONS: &str = "Correlations";
const MONTE_CARLO: &str = "Monte Carlo";
const HISTORICAL: &str = "Historical";

const fn pair(key: &'static str, label: &'static str, default: &'static str) -> FieldSpec {
    FieldSpec::text(key, label)
        .in_group(CORRELATIONS)
        .defaults_to(default)
        .help(PAIR_HELP)
}

const fn mean(key: &'static str, class: &'static str, default: &'static str) -> FieldSpec {
    FieldSpec::rate(key, "Mean return")
        .in_group(class)
        .defaults_to(default)
}

const fn spread(key: &'static str, class: &'static str, default: &'static str) -> FieldSpec {
    FieldSpec::rate(key, "Spread")
        .in_group(class)
        .defaults_to(default)
}

impl Single for MarketSettings {
    type Item = Market;
    const ID: DomainId = DomainId::Market;
    const PATHS: &'static [&'static str] = &["market"];
    const FIELDS: &'static [FieldSpec] = &[
        FieldSpec::money("leave_at_least", "Leave at least").in_group(SUCCESS).blank("Nothing").help(
            "Besides never falling short, what a run must end with, in today's dollars, to count as a success. Optional.",
        ),
        mean("stocks.mean", STOCKS, "6%").help("Stocks' compound yearly return, the median year's. Blank is 6%."),
        spread("stocks.volatility", STOCKS, "16%")
            .help("How far a year\'s return strays, as a standard deviation. Blank is 16%."),
        mean("bonds.mean", BONDS, "4%").help("Bonds' compound yearly return, the median year's. Blank is 4%."),
        spread("bonds.volatility", BONDS, "6%")
            .help("How far a year\'s return strays, as a standard deviation. Blank is 6%."),
        mean("cash.mean", CASH, "2.5%").help("Cash's compound yearly return, the median year's. Blank is 2.5%."),
        spread("cash.volatility", CASH, "1%")
            .help("How far a year\'s return strays, as a standard deviation. Blank is 1%."),
        FieldSpec::rate("inflation.volatility", "Spread").in_group(INFLATION).defaults_to("1.5%").help(
            "How far a year's inflation strays from the plan's rate, as a yearly shock. Blank is 1.5%.",
        ),
        FieldSpec::share("inflation.persistence", "Persistence").in_group(INFLATION).defaults_to("60%").help(
            "How much of a year's stray carries into the next, so high inflation comes in runs. Blank is 60%.",
        ),
        pair("correlation.stocks_bonds", "Stocks and bonds", "0.1"),
        pair("correlation.stocks_cash", "Stocks and cash", "0"),
        pair("correlation.stocks_inflation", "Stocks and inflation", "-0.1"),
        pair("correlation.bonds_cash", "Bonds and cash", "0"),
        pair("correlation.bonds_inflation", "Bonds and inflation", "-0.2"),
        pair("correlation.cash_inflation", "Cash and inflation", "0.5"),
        FieldSpec::choice("monte_carlo.draw", "Draws from", Vocabulary::Draw)
            .in_group(MONTE_CARLO)
            .blank(crate::present::draw(Draw::Assumptions))
            .help("Random years from the assumptions above, or historical years drawn at random."),
        FieldSpec::whole("monte_carlo.trials", "Trials").in_group(MONTE_CARLO).defaults_to("1,000")
            .help("How many markets are run. Blank is 1,000."),
        FieldSpec::whole("monte_carlo.seed", "Seed").in_group(MONTE_CARLO).defaults_to("42")
            .help("The same seed draws the same markets, so an edit is measured against them. Blank is 42."),
        FieldSpec::whole("monte_carlo.block_years", "Years together").in_group(MONTE_CARLO).defaults_to("1")
            .shown_when(draws_history)
            .help("How many consecutive historical years each draw takes. Blank is 1."),
        FieldSpec::whole("historical.from", "From").in_group(HISTORICAL).defaults_to("1871")
            .help("The first year tried as the plan's first. Blank is the record's first, 1871."),
        FieldSpec::whole("historical.to", "To").in_group(HISTORICAL).defaults_to("2025").help("The last year tried. Blank is the record's last, 2025."),
        FieldSpec::flag(WRAPS, "Wrap").in_group(HISTORICAL).derived(seed_wraps, write_wraps).help(
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
    use crate::forms::{Blank, heading};
    use crate::present;

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
        assert_eq!(default_of(key), Some(default), "{key}");
    }

    fn default_of(key: &str) -> Option<&'static str> {
        let spec = MarketSettings::FIELDS.iter().find(|spec| spec.key == key);
        match spec?.blank? {
            Blank::Default(value) => Some(value),
            Blank::Means(_) => None,
        }
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
        let from = defaults.from().to_string();
        assert_eq!(default_of("historical.from"), Some(from.as_str()));
        let to = defaults.to().to_string();
        assert_eq!(default_of("historical.to"), Some(to.as_str()));
        let matrix = defaults.correlation();
        let (stocks, bonds, cash, inflation) = (0, 1, 2, 3);
        let pairs = [
            ("stocks_bonds", matrix[stocks][bonds]),
            ("stocks_cash", matrix[stocks][cash]),
            ("stocks_inflation", matrix[stocks][inflation]),
            ("bonds_cash", matrix[bonds][cash]),
            ("bonds_inflation", matrix[bonds][inflation]),
            ("cash_inflation", matrix[cash][inflation]),
        ];
        for (pair, default) in pairs {
            let key = format!("correlation.{pair}");
            let said = default.to_string();
            assert_eq!(default_of(&key), Some(said.as_str()), "{key}");
        }
    }

    #[test]
    fn the_fields_are_gathered_under_headings_and_named_with_them_elsewhere() {
        let fields = MarketSettings::FIELDS;
        let headings: Vec<&str> = (0..fields.len())
            .filter_map(|at| heading(at.checked_sub(1).map(|before| &fields[before]), &fields[at]))
            .collect();
        let expected = [
            "Success",
            "Stocks",
            "Bonds",
            "Cash",
            "Inflation",
            "Correlations",
            "Monte Carlo",
            "Historical",
        ];
        assert_eq!(headings, expected);
        let spread = fields.iter().find(|spec| spec.key == "bonds.volatility");
        assert_eq!(
            spread.map(FieldSpec::named).as_deref(),
            Some("Bonds: Spread")
        );
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
