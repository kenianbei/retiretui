//! The market tools as every surface says them: how the plan fared and in
//! which zone, the runs singled out, what they were made under and where
//! that is edited, net worth by year, how the runs end, and a run named by
//! the market it went through.

use retiretui_engine::market::{BAND_PERCENTILES, MonteCarlo, Run, RunName, Runs};
use retiretui_engine::plan::{Account, AssetClass, Dollars, Draw, Item, Plan};
use serde::Serialize;

use crate::forms::DomainId;
use crate::present::{self, MoneyForm};
use crate::table::{count, money, percentile_label, rate};

/// What a market tool's runs table says before its first search answers.
pub const NOTHING_SEARCHED: &str = "Runs by itself while this page is shown.";
/// What the plan's own row says first.
pub const PLANNED: &str = "As planned";
/// What a run never short says in its "Short in" cell.
const NEVER: &str = "never";
/// A share this high or above reads as comfortable.
pub(crate) const GOOD_ZONE: f64 = 0.9;
/// A share this high or above, and under [`GOOD_ZONE`], reads as close.
const CAUTION_ZONE: f64 = 0.75;
/// Where the ending buckets break, in today's dollars: under the first,
/// between each two, and over the last.
const MILLION: Dollars = 1_000_000;
const ENDING_BREAKS: [Dollars; 4] = [MILLION, 2 * MILLION, 4 * MILLION, 8 * MILLION];
const SHORT: &str = "short";
const WORST: &str = "Worst";
const TRIAL: &str = "trial-";

/// A share of runs to a tenth of a percent: `85.2%`.
pub(crate) fn share(share: f64) -> String {
    const TENTHS_OF_A_PERCENT: f64 = 1000.0;
    rate((share * TENTHS_OF_A_PERCENT).round() / TENTHS_OF_A_PERCENT)
}

/// How a plan fared, `share_of_runs` of its `runs`, named by the noun's
/// two forms: `87% of 1,000 markets`.
#[must_use]
pub fn verdict_of(share_of_runs: f64, runs: usize, (one, many): (&str, &str)) -> String {
    let counted = present::counted(runs, one, many);
    format!("{} of {counted}", share(share_of_runs))
}

/// What a market tool searches, and how it names what it found.
pub trait Markets {
    /// The runs table's first column.
    const RUN_HEADING: &'static str;
    /// What one run is, and many: `market`, `markets`.
    const RUN_NOUN: (&'static str, &'static str);
    /// What the tool is for, in a line.
    const ABOUT: &'static str;
    /// Whether net worth year by year at each percentile is shown:
    /// percentiles over overlapping histories claim a precision they do not
    /// have.
    const HAS_BY_YEAR: bool;

    /// Every run, and the plan in its own market.
    fn runs(&self) -> &Runs;

    /// The runs singled out, in the table's order.
    fn listed(&self) -> Vec<Listed<'_>>;

    /// How the plan fared, under a heading that says of what: `87% of
    /// 1,000 markets`.
    fn verdict(&self) -> String {
        let runs = self.runs();
        verdict_of(runs.success_rate(), runs.runs.len(), Self::RUN_NOUN)
    }

    /// How the plan fared, as a sentence of its own: `Money lasts in 87%
    /// of 1,000 markets`.
    fn headline(&self) -> String {
        format!("{} in {}", present::MONEY_LASTS, self.verdict())
    }

    /// What the runs are made under, besides what both tools share.
    fn settings(plan: &Plan) -> Vec<Assumption>;
}

/// A run singled out: the key a highlight is kept by, which the same place
/// keeps after a search again, its first cell, and the run.
#[derive(Debug)]
pub struct Listed<'a> {
    /// `p90`, `worst`, or a start year.
    pub key: String,
    /// What the table's first column says of it.
    pub first: String,
    /// The run.
    pub run: &'a Run,
}

/// A row of what the runs are made under, and where it is edited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assumption {
    /// What it is.
    pub label: &'static str,
    /// What the plan says of it.
    pub value: String,
    /// The domain it is edited in.
    pub domain: DomainId,
    /// The field it is edited at, where it is one.
    pub field: Option<&'static str>,
}

impl Assumption {
    fn market(label: &'static str, value: String, field: &'static str) -> Self {
        Self {
            label,
            value,
            domain: DomainId::Market,
            field: Some(field),
        }
    }
}

/// How a share of runs reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum Zone {
    /// Comfortable.
    Good,
    /// Close.
    Caution,
    /// Too often short.
    Short,
}

/// The zone `share` falls in.
#[must_use]
pub fn zone_of(share: f64) -> Zone {
    if share >= GOOD_ZONE {
        Zone::Good
    } else if share >= CAUTION_ZONE {
        Zone::Caution
    } else {
        Zone::Short
    }
}

/// How many runs ended in a bucket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Ending {
    /// The bucket: "short", or a span of what the runs end with.
    pub label: String,
    /// How many runs ended in it.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub count: u64,
    /// Whether it holds the runs that fell short.
    pub is_short: bool,
}

/// The runs table's columns.
#[must_use]
pub fn run_columns<M: Markets>() -> [&'static str; 3] {
    [M::RUN_HEADING, "Ends with", "Short in"]
}

/// A run's row: its first cell, what it ends with in `form`, and the year
/// it first falls short, with the eldest's age then.
#[must_use]
pub fn run_cells(plan: &Plan, first: String, run: &Run, form: MoneyForm) -> Vec<String> {
    let short = run.first_short.map_or_else(
        || NEVER.to_owned(),
        |year| match plan.household.people.first() {
            Some(person) => format!("{year} ({})", person.age_in_year(year)),
            None => year.to_string(),
        },
    );
    vec![first, form.money(run.ending), short]
}

/// What success means under the plan's `[market]`.
fn success(plan: &Plan) -> Assumption {
    let value = plan.market().leave_at_least().map_or_else(
        || "Never running short".to_owned(),
        |floor| format!("Ending with at least {}", money(floor)),
    );
    Assumption::market("Counts as a success", value, "leave_at_least")
}

/// Each asset class's return and inflation's, as the plan assumes them.
fn assumed(plan: &Plan) -> Vec<Assumption> {
    let market = plan.market();
    let mut rows: Vec<Assumption> = [
        ("Stocks", AssetClass::Stocks, "stocks.mean"),
        ("Bonds", AssetClass::Bonds, "bonds.mean"),
        ("Cash", AssetClass::Cash, "cash.mean"),
    ]
    .into_iter()
    .map(|(label, class, field)| {
        let spread = format!(
            "{} ± {}",
            rate(market.mean(class)),
            rate(market.volatility(class))
        );
        Assumption::market(label, spread, field)
    })
    .collect();
    let inflation = format!(
        "{} ± {}",
        rate(plan.plan.inflation),
        rate(market.inflation_volatility())
    );
    rows.push(Assumption::market(
        "Inflation",
        inflation,
        "inflation.volatility",
    ));
    rows
}

/// The accounts no mix is held in, which earn their fixed return in every
/// market.
fn unmixed(plan: &Plan) -> Option<Assumption> {
    let names: Vec<&str> = plan
        .accounts
        .iter()
        .filter(|account| account.allocation.is_none())
        .map(Account::display_name)
        .collect();
    (!names.is_empty()).then(|| Assumption {
        label: "Fixed return",
        value: names.join(", "),
        domain: DomainId::Accounts,
        field: None,
    })
}

/// What `M`'s runs are made under: what success means, its settings, and
/// the accounts every market leaves alone.
#[must_use]
pub fn assumptions<M: Markets>(plan: &Plan) -> Vec<Assumption> {
    let mut rows = vec![success(plan)];
    rows.extend(M::settings(plan));
    rows.extend(unmixed(plan));
    rows
}

/// Net worth at each percentile, and the share still funded, year by
/// year, the net worth in `form`: the header, then a row a year.
#[must_use]
pub fn by_year(runs: &Runs, form: MoneyForm) -> (Vec<String>, Vec<Vec<String>>) {
    let mut header = vec!["Year".to_owned()];
    header.extend(
        BAND_PERCENTILES
            .iter()
            .map(|percentile| format!("{percentile}th")),
    );
    header.push("Funded".to_owned());
    let rows = runs
        .bands
        .iter()
        .map(|band| {
            let mut cells = vec![band.year.to_string()];
            cells.extend(band.net_worth.iter().map(|&worth| form.money(worth)));
            cells.push(rate(band.funded));
            cells
        })
        .collect();
    (header, rows)
}

/// How many runs end in each bucket, those that fell short first.
#[must_use]
pub fn endings(runs: &Runs) -> Vec<Ending> {
    let mut counts = vec![0_u64; ENDING_BREAKS.len() + 2];
    for run in &runs.runs {
        let at = if run.first_short.is_some() {
            0
        } else {
            1 + ENDING_BREAKS
                .iter()
                .filter(|&&edge| run.ending >= edge)
                .count()
        };
        counts[at] += 1;
    }
    counts
        .into_iter()
        .enumerate()
        .map(|(at, count)| Ending {
            label: bucket_label(at),
            count,
            is_short: at == 0,
        })
        .collect()
}

/// The words for bucket `at`: short, then each span between the breaks,
/// which are whole millions.
fn bucket_label(at: usize) -> String {
    let millions = |amount: Dollars| format!("${}M", amount / MILLION);
    match at {
        0 => SHORT.to_owned(),
        1 => format!("<{}", millions(ENDING_BREAKS[0])),
        _ if at > ENDING_BREAKS.len() => {
            format!("{}+", millions(ENDING_BREAKS[ENDING_BREAKS.len() - 1]))
        }
        _ => format!(
            "{}–{}",
            millions(ENDING_BREAKS[at - 2]),
            millions(ENDING_BREAKS[at - 1])
        ),
    }
}

/// The market a run went through, as an address keeps it: `trial-423`,
/// `1929`; none for the plan's own.
#[must_use]
pub fn market_key(name: RunName) -> Option<String> {
    match name {
        RunName::Planned => None,
        RunName::Trial(trial) => Some(format!("{TRIAL}{trial}")),
        RunName::Start(year) => Some(year.to_string()),
    }
}

/// The market [`market_key`] wrote; none for what it cannot have.
#[must_use]
pub fn market_of(key: &str) -> Option<RunName> {
    match key.strip_prefix(TRIAL) {
        Some(trial) => trial.parse().ok().map(RunName::Trial),
        None => key.parse().ok().map(RunName::Start),
    }
}

/// A run's market named by what it is, which an edit leaves true:
/// `random market 423`, `retiring in 1929`.
#[must_use]
pub fn market_said(name: RunName) -> String {
    match name {
        RunName::Planned => "the plan's own market".to_owned(),
        RunName::Trial(trial) => format!("random market {trial}"),
        RunName::Start(year) => format!("retiring in {year}"),
    }
}

impl Markets for MonteCarlo {
    const RUN_HEADING: &'static str = "Markets";
    const RUN_NOUN: (&'static str, &'static str) = ("market", "markets");
    const ABOUT: &'static str = "The plan through many random markets; the share in which its money lasts says how surely it does.";
    const HAS_BY_YEAR: bool = true;

    fn runs(&self) -> &Runs {
        &self.runs
    }

    fn listed(&self) -> Vec<Listed<'_>> {
        let places = BAND_PERCENTILES
            .iter()
            .rev()
            .map(|&percentile| (format!("p{percentile}"), percentile_label(percentile)));
        let mut listed: Vec<Listed<'_>> = places
            .zip(&self.singled_out)
            .map(|((key, first), &at)| Listed {
                key,
                first,
                run: &self.runs.runs[at],
            })
            .collect();
        if let Some(&worst) = self.singled_out.get(BAND_PERCENTILES.len()) {
            listed.push(Listed {
                key: WORST.to_lowercase(),
                first: WORST.to_owned(),
                run: &self.runs.runs[worst],
            });
        }
        listed
    }

    fn settings(plan: &Plan) -> Vec<Assumption> {
        let market = plan.market();
        let drawn = match market.draw() {
            Draw::History if market.block_years() > 1 => format!(
                "{}, {} together",
                present::draw(Draw::History),
                market.block_years()
            ),
            draw => present::draw(draw).to_owned(),
        };
        let trials = format!(
            "{} · seed {}",
            count(usize::try_from(market.trials()).unwrap_or_default()),
            market.seed()
        );
        let mut rows = vec![
            Assumption::market("Markets drawn from", drawn, "monte_carlo.draw"),
            Assumption::market("Trials", trials, "monte_carlo.trials"),
        ];
        rows.extend(assumed(plan));
        rows
    }
}

impl Markets for Runs {
    const RUN_HEADING: &'static str = "Start years";
    const RUN_NOUN: (&'static str, &'static str) = ("start year", "start years");
    const ABOUT: &'static str = "The plan from each year of the U.S. record as its first, through the markets that followed.";
    const HAS_BY_YEAR: bool = false;

    fn runs(&self) -> &Runs {
        self
    }

    fn listed(&self) -> Vec<Listed<'_>> {
        self.worst_first()
            .into_iter()
            .map(|at| {
                let run = &self.runs[at];
                let started = match run.name {
                    RunName::Start(year) => year.to_string(),
                    RunName::Planned | RunName::Trial(_) => String::new(),
                };
                Listed {
                    key: started.clone(),
                    first: started,
                    run,
                }
            })
            .collect()
    }

    fn settings(plan: &Plan) -> Vec<Assumption> {
        let market = plan.market();
        let wrapped = if market.wrap() { ", wrapped" } else { "" };
        let years = format!("{}–{}{wrapped}", market.from(), market.to());
        vec![Assumption::market("Years", years, "historical.from")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_share_of_runs_is_said_to_a_tenth_of_a_percent() {
        assert_eq!(share(132.0 / 155.0), "85.2%");
        assert_eq!(share(0.997), "99.7%");
        assert_eq!(share(1.0), "100%");
    }

    #[test]
    fn a_share_reads_by_its_zone_at_the_edges() {
        assert_eq!(zone_of(0.9), Zone::Good);
        assert_eq!(zone_of(0.899), Zone::Caution);
        assert_eq!(zone_of(0.75), Zone::Caution);
        assert_eq!(zone_of(0.749), Zone::Short);
    }

    #[test]
    fn a_market_key_reads_back_as_the_market_it_names() {
        for name in [RunName::Trial(423), RunName::Start(1929)] {
            let key = market_key(name).expect("a market of its own");
            assert_eq!(market_of(&key), Some(name));
        }
        assert_eq!(market_key(RunName::Planned), None);
        assert_eq!(market_of("trial-x"), None);
        assert_eq!(market_of("p10"), None);
        assert_eq!(market_said(RunName::Trial(423)), "random market 423");
        assert_eq!(market_said(RunName::Start(1929)), "retiring in 1929");
    }

    #[test]
    fn the_buckets_run_from_short_past_the_last_break() {
        let labels: Vec<String> = (0..ENDING_BREAKS.len() + 2).map(bucket_label).collect();
        assert_eq!(
            labels,
            ["short", "<$1M", "$1M–$2M", "$2M–$4M", "$4M–$8M", "$8M+"]
        );
    }
}
