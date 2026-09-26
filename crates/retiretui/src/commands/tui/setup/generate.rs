//! The plan the form's answers become: written as a plan file and parsed
//! through the schema's own types, so nothing here states a default the
//! schema already states.
//!
//! Items are emitted in reference order - the people, then the events
//! that name them, then the accounts, income and expenses that name
//! both - so nothing a generated plan references dangles.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::ops::RangeInclusive;

use retiretui_engine::params::{BenefitParams, Inflation, TaxTables};
use retiretui_engine::plan::{Dollars, FilingStatus, Plan};
use retiretui_engine::tax::{FIRST_WORKING_AGE, earnings_at_wage};

use super::{Answered, LifeStage, SetupAnswers};
use crate::commands::tui::session::{CASH_ID, HORIZON_AGE, INFLATION};

/// The id a person whose name writes nothing down gets.
const FALLBACK_ID: &str = "person";

/// What a second person whose name slugifies to the first's takes, so two
/// people spelt the same are still two people.
const COLLISION_SUFFIX: char = '2';

/// The one expense every household has, which the spending step edits.
const LIVING_ID: &str = "living";

/// What a new workplace account is invested in: a balanced mix, so the
/// market tools have something to vary from the first run.
const STARTING_MIX: &str = "{ stocks = 0.6, bonds = 0.4 }";

/// Of a salary, what the household is taken to spend.
const SPENT_OF_SALARY: f64 = 0.7;

/// What a household spends where the answers give nothing to take it
/// from.
const DEFAULT_LIVING: Dollars = 60_000;

const DEFAULT_AGE: i16 = 40;
const DEFAULT_RETIREMENT_AGE: u8 = 65;
const DEFAULT_CLAIM_AGE: u8 = 67;

/// The oldest a person may be taken to be, which keeps a birth year a
/// date the schema can parse.
const OLDEST: i16 = 120;

const NOT_BUILT: &str = "the answers do not make a plan";

/// How a person's Social Security benefit is written: the figure they
/// typed, left for the engine to compute from a career at their salary,
/// or not at all.
enum Benefit {
    None,
    Stated(Dollars),
    Computed(BTreeMap<i16, Dollars>),
}

/// A person as the generator needs them: every blank answered.
struct Member {
    id: String,
    /// The name as typed, where one was.
    name: Option<String>,
    birth_year: i16,
    retirement_age: u8,
    salary: Dollars,
    claim_age: u8,
    benefit: Benefit,
}

impl Member {
    /// The event the person's income and contributions end at.
    fn retire_event(&self) -> String {
        format!("retire-{}", self.id)
    }

    fn deferred_account(&self) -> String {
        format!("{}-401k", self.id)
    }
}

/// What every person's answers are read against.
struct Household<'a> {
    start_year: i16,
    stage: LifeStage,
    params: Option<&'a BenefitParams>,
}

impl Answered<'_> {
    /// The person the answers describe, with `taken` the id the person
    /// before them has.
    fn filled(self, household: &Household<'_>, taken: Option<&str>) -> Member {
        let start_year = household.start_year;
        let born = start_year.saturating_sub(DEFAULT_AGE);
        let birth_year = self
            .birth_year
            .unwrap_or(born)
            .clamp(start_year.saturating_sub(OLDEST), start_year - 1);
        let salary = self.salary.unwrap_or_default().max(0);
        let retirement_age = self.retirement_age.unwrap_or(DEFAULT_RETIREMENT_AGE);
        let started =
            (self.working_since).unwrap_or_else(|| birth_year.saturating_add(FIRST_WORKING_AGE));
        let stopped = match household.stage {
            LifeStage::Working => household.start_year,
            LifeStage::Retired => birth_year
                .saturating_add(i16::from(retirement_age))
                .min(household.start_year),
        };
        let benefit = match self.social_security {
            Some(figure) if figure > 0 => Benefit::Stated(figure),
            Some(_) => Benefit::None,
            None => career(household, salary, started..=stopped - 1),
        };
        Member {
            id: id_of(self.name.unwrap_or_default(), taken),
            name: self
                .name
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_owned),
            birth_year,
            retirement_age,
            salary,
            claim_age: self.claim_age.unwrap_or(DEFAULT_CLAIM_AGE),
            benefit,
        }
    }
}

/// The benefit a person who typed none gets: computed from a career at
/// the salary - what they earn, or last earned - over the years they
/// worked; nothing where they name no salary or no year worked.
fn career(household: &Household<'_>, salary: Dollars, worked: RangeInclusive<i16>) -> Benefit {
    let Some(params) = household.params else {
        return Benefit::None;
    };
    if salary <= 0 || worked.is_empty() {
        return Benefit::None;
    }
    Benefit::Computed(earnings_at_wage(
        params,
        salary,
        household.start_year,
        worked,
    ))
}

/// A typed name as a plan id: its alphanumerics, lowercased.
fn id_of(name: &str, taken: Option<&str>) -> String {
    let slug: String = name
        .chars()
        .filter(|letter| letter.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    let id = if slug.is_empty() {
        FALLBACK_ID.to_owned()
    } else {
        slug
    };
    if taken == Some(id.as_str()) {
        format!("{id}{COLLISION_SUFFIX}")
    } else {
        id
    }
}

/// The people the answers describe: one, or two where the household files
/// jointly.
fn members(answers: &SetupAnswers, start_year: i16, tables: &TaxTables) -> Vec<Member> {
    let params = tables
        .params_for(start_year, &Inflation::constant(INFLATION))
        .social_security
        .benefit;
    let household = Household {
        start_year,
        stage: answers.stage(),
        params: params.as_ref(),
    };
    let first = answers.first().filled(&household, None);
    if answers.filing() == FilingStatus::Single {
        return vec![first];
    }
    let partner = answers.partner().filled(&household, Some(&first.id));
    vec![first, partner]
}

/// The plan the answers build.
///
/// # Errors
///
/// What the schema said when it would not parse what was written, which
/// is a defect in the generator rather than in the answers.
pub fn plan(answers: &SetupAnswers, start_year: i16, tables: &TaxTables) -> Result<Plan, String> {
    let document = document(answers, &members(answers, start_year, tables), start_year);
    Plan::from_toml_str(&document).map_err(|error| format!("{NOT_BUILT}: {error}"))
}

fn document(answers: &SetupAnswers, members: &[Member], start_year: i16) -> String {
    let stage = answers.stage();
    let mut toml = String::new();
    let _ = write!(
        toml,
        "schema = 1\n\n[plan]\nstart_year = {start_year}\nhorizon_age = {HORIZON_AGE}\ninflation = {INFLATION}\n"
    );
    write_household(&mut toml, answers.filing(), members);
    if stage == LifeStage::Working {
        write_events(&mut toml, members);
    }
    write_accounts(&mut toml, members);
    write_income(&mut toml, members, stage, start_year);
    let _ = write!(
        toml,
        "\n[[expenses]]\nid = \"{LIVING_ID}\"\namount = {}\n",
        living(members, stage)
    );
    toml
}

fn write_household(toml: &mut String, filing: FilingStatus, members: &[Member]) {
    let _ = write!(toml, "\n[household]\nfiling = \"{}\"\n", filing.as_str());
    for member in members {
        let _ = write!(
            toml,
            "\n[[household.people]]\nid = \"{}\"\nbirth = {:04}-01-01\n",
            member.id, member.birth_year
        );
        if let Some(name) = &member.name {
            let _ = writeln!(toml, "name = {}", toml::Value::String(name.clone()));
        }
        let Benefit::Computed(earnings) = &member.benefit else {
            continue;
        };
        let years: Vec<String> = earnings
            .iter()
            .map(|(year, amount)| format!("{year} = {amount}"))
            .collect();
        let _ = writeln!(toml, "earnings = {{ {} }}", years.join(", "));
    }
}

fn write_events(toml: &mut String, members: &[Member]) {
    for member in members {
        let _ = write!(
            toml,
            "\n[[events]]\nid = \"{}\"\ntrigger = {{ age = {}, owner = \"{}\" }}\n",
            member.retire_event(),
            member.retirement_age,
            member.id
        );
    }
}

/// A cash account for the household, and a deferred one per person,
/// balances left for the savings step to fill in.
fn write_accounts(toml: &mut String, members: &[Member]) {
    let Some(first) = members.first() else {
        return;
    };
    let _ = write!(
        toml,
        "\n[[accounts]]\nid = \"{CASH_ID}\"\nkind = \"cash\"\nowner = \"{}\"\nbalance = 0\n",
        first.id
    );
    for member in members {
        let _ = write!(
            toml,
            "\n[[accounts]]\nid = \"{}\"\nkind = \"401k\"\nowner = \"{}\"\nbalance = 0\nallocation = {STARTING_MIX}\n",
            member.deferred_account(),
            member.id
        );
    }
}

/// A salary ending the year before the person retires, and the benefit
/// they claim - the figure typed, or none for the engine to compute from
/// the career - at the age they gave, or from the plan's first year for a
/// retiree already past it. The schema takes no benefit without a claim,
/// so a retiree's is dated rather than left out.
fn write_income(toml: &mut String, members: &[Member], stage: LifeStage, start_year: i16) {
    for member in members {
        if stage == LifeStage::Working && member.salary > 0 {
            let _ = write!(
                toml,
                "\n[[income]]\nid = \"salary-{}\"\nkind = \"salary\"\nowner = \"{}\"\namount = {}\nend = {{ event = \"{}\", offset = -1 }}\n",
                member.id,
                member.id,
                member.salary,
                member.retire_event()
            );
        }
        if matches!(member.benefit, Benefit::None) {
            continue;
        }
        let _ = write!(
            toml,
            "\n[[income]]\nid = \"ss-{}\"\nkind = \"social-security\"\nowner = \"{}\"\n",
            member.id, member.id
        );
        if let Benefit::Stated(figure) = member.benefit {
            let _ = writeln!(toml, "amount = {figure}");
        }
        let has_claimed = start_year - member.birth_year >= i16::from(member.claim_age);
        let _ = if stage == LifeStage::Retired && has_claimed {
            writeln!(toml, "start = {{ date = {start_year:04}-01-01 }}")
        } else {
            writeln!(
                toml,
                "start = {{ age = {}, owner = \"{}\" }}",
                member.claim_age, member.id
            )
        };
    }
}

/// What the household is taken to spend: most of what it earns, or at
/// least what its benefits cover, and a round figure where it says
/// neither.
fn living(members: &[Member], stage: LifeStage) -> Dollars {
    let earned: Dollars = match stage {
        LifeStage::Working => members.iter().map(|member| member.salary).sum(),
        LifeStage::Retired => 0,
    };
    let benefits: Dollars = members
        .iter()
        .filter_map(|member| match member.benefit {
            Benefit::Stated(figure) => Some(figure),
            Benefit::None | Benefit::Computed(_) => None,
        })
        .sum();
    let spent = (earned as f64 * SPENT_OF_SALARY) as Dollars;
    let most = spent.max(benefits);
    if most > 0 { most } else { DEFAULT_LIVING }
}
