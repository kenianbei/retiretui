//! The spending ceiling as every surface says it: the target share a
//! person holds it to, the search in the plan's own market and at that
//! target, the options' rows, and what is asked and said when one is taken
//! into the plan.

use retiretui_engine::market::{History, Progress, RunError, monte_carlo};
use retiretui_engine::optimize::{
    CEILING_STEPS, Measure, SpendingCeiling, apply_spending, spending_ceiling,
};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Dollars, Item, Plan};
use serde::Deserialize;

use super::markets::{GOOD_ZONE, share};
use super::{capitalised, figures};
use crate::codec::from_table;
use crate::forms::{FieldSpec, ToolAnswers};
use crate::present::{self, MoneyForm};
use crate::table::rate;

/// What the options say before anything is searched.
pub const NOTHING_SEARCHED: &str =
    "The most you could spend is searched here as soon as the plan is valid.";

/// What the tool is for, in a line.
pub const ABOUT: &str = "The most your flexible spending can be and still last, in the plan's own market and in most random ones.";

/// What is said where the plan asks to leave nothing: the ceiling in its
/// own market spends it all.
pub const SPENDS_IT_ALL: &str = "In its own market the plan spends it all by its end. Leave at least, on the Market page, holds some back.";

/// What names an option's columns before its [`FIGURES`](super::FIGURES).
pub const OPTION_COLUMNS: [&str; 4] = ["Held to", "Flexible", "Change", present::SUCCESS];
/// The columns a ceiling's expenses are tabled under.
pub const ITEM_COLUMNS: [&str; 3] = ["Expense", "Now", "At the ceiling"];

const OWN_MARKET: &str = "In its own market";
const AT_LEAST: &str = "at least";
const PERCENT: f64 = 100.0;

/// The key the ceiling in the plan's own market is highlighted by.
pub const PLANNED: &str = "planned";
/// The key the ceiling at the target share is highlighted by.
pub const AT_TARGET: &str = "target";
/// The place among the [listed](Found::listed) ceilings of the one a
/// person is after, which a highlight starts on: the one at the target.
pub const LEADING: usize = 1;

/// The target share where none is stated: the one that reads as
/// comfortable.
pub const DEFAULT_TARGET: f64 = GOOD_ZONE;

/// The tool's settings as its form holds them: the target share, blank
/// taking the share that reads as comfortable.
#[derive(Deserialize)]
pub struct Answers {
    success: Option<f64>,
}

/// The Spending Ceiling form's fields.
pub const FIELDS: &[FieldSpec] = &[FieldSpec::share("success", "Target success")
    .defaults_to("90%")
    .help("The share of random markets the higher spending must last in.")];

impl ToolAnswers for Answers {
    const SLOT: &'static str = "spending";
}

/// The target share `answers`, the form's table, holds.
#[must_use]
pub fn target_in(answers: toml::Table) -> f64 {
    let held = from_table::<Answers>(answers).ok();
    held.and_then(|answers| answers.success)
        .unwrap_or(DEFAULT_TARGET)
}

/// How many steps a search of `plan` counts at most: a Monte Carlo search
/// for each plan the target is judged by, and one more for the ceiling in
/// the plan's own market.
#[must_use]
pub fn total(plan: &Plan) -> usize {
    let trials = usize::try_from(plan.market().trials()).unwrap_or_default();
    (CEILING_STEPS + 1) * trials
}

/// What a search found.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    /// The ceiling in the plan's own market.
    pub planned: SpendingCeiling,
    /// The share of random markets the plan lasts in at that ceiling.
    pub planned_success: f64,
    /// The ceiling in the target share of random markets.
    pub at_target: SpendingCeiling,
    /// That target.
    pub target: f64,
}

/// Searches `plan`'s spending ceiling in its own market and in `target` of
/// its random markets, and runs the first through those markets too.
///
/// # Errors
///
/// As [`spending_ceiling`] refuses or is cancelled.
pub fn search(
    plan: &Plan,
    tables: &TaxTables,
    history: &History,
    target: f64,
    progress: &Progress,
) -> Result<Found, RunError> {
    let planned = spending_ceiling(plan, tables, history, Measure::Planned, progress)?;
    let at_target = spending_ceiling(plan, tables, history, Measure::Success(target), progress)?;
    let mut spent = plan.clone();
    apply_spending(&mut spent, &planned.expenses);
    let runs = monte_carlo(&spent, tables, history, progress)?.runs;
    Ok(Found {
        planned,
        planned_success: runs.success_rate(),
        at_target,
        target,
    })
}

/// One ceiling as an option.
#[derive(Debug)]
pub struct Listed<'a> {
    /// What a highlight is kept by: [`PLANNED`] or [`AT_TARGET`].
    pub key: &'static str,
    /// What the ceiling was held to: `In 90% of markets`.
    pub held_to: String,
    /// The ceiling.
    pub ceiling: &'a SpendingCeiling,
    /// The share of random markets the plan lasts in at it.
    pub success: f64,
}

impl Found {
    /// The share of random markets the plan lasts in as it stands.
    #[must_use]
    pub fn plan_success(&self) -> f64 {
        self.at_target.baseline.success_rate.unwrap_or_default()
    }

    /// The ceilings, in the table's order.
    #[must_use]
    pub fn listed(&self) -> [Listed<'_>; 2] {
        let at_target = &self.at_target;
        [
            Listed {
                key: PLANNED,
                held_to: OWN_MARKET.to_owned(),
                ceiling: &self.planned,
                success: self.planned_success,
            },
            Listed {
                key: AT_TARGET,
                held_to: format!("In {} of markets", rate(self.target)),
                ceiling: at_target,
                success: at_target.judged.success_rate.unwrap_or_default(),
            },
        ]
    }

    /// The plan's own row past its name: what `plan` spends flexibly, no
    /// change, its success, and its [`FIGURES`](super::FIGURES).
    #[must_use]
    pub fn plan_cells(&self, plan: &Plan, deflated: bool, form: MoneyForm) -> Vec<String> {
        let summary = self.planned.baseline.projection.summary(deflated);
        let stated = form.money(plan.flexible_spending());
        let leading = [stated, String::new(), share(self.plan_success())];
        leading.into_iter().chain(figures(&summary, form)).collect()
    }
}

impl Listed<'_> {
    /// The flexible spending a year at the ceiling, in today's dollars.
    #[must_use]
    pub fn flexible(&self) -> Dollars {
        let scaled = self.ceiling.expenses.iter();
        scaled.map(|expense| expense.amount).sum()
    }

    /// The flexible spending as a cell says it, `at least` where the
    /// search was capped.
    #[must_use]
    pub fn flexible_said(&self, form: MoneyForm) -> String {
        let money = form.money(self.flexible());
        if self.ceiling.is_capped {
            format!("{AT_LEAST} {money}")
        } else {
            money
        }
    }

    /// The ceiling's flexible spending against what `plan` states: `+18%`.
    #[must_use]
    pub fn change(&self, plan: &Plan) -> String {
        let stated = plan.flexible_spending().max(1);
        let percent = ((self.flexible() - stated) as f64 / stated as f64 * PERCENT).round();
        if percent == 0.0 {
            present::SAME.to_owned()
        } else {
            format!("{percent:+}%")
        }
    }

    /// The row past what it was held to: its flexible spending, the change
    /// against `plan`, its success, and its [`FIGURES`](super::FIGURES).
    #[must_use]
    pub fn cells(&self, plan: &Plan, deflated: bool, form: MoneyForm) -> Vec<String> {
        let summary = self.ceiling.judged.projection.summary(deflated);
        let leading = [
            self.flexible_said(form),
            self.change(plan),
            share(self.success),
        ];
        leading.into_iter().chain(figures(&summary, form)).collect()
    }

    /// Each expense the ceiling scales under [`ITEM_COLUMNS`]: its name,
    /// what `plan` states, and what the ceiling makes it.
    #[must_use]
    pub fn items(&self, plan: &Plan, form: MoneyForm) -> Vec<[String; 3]> {
        let scaled = self.ceiling.expenses.iter();
        scaled
            .filter_map(|scaled| {
                let stated = plan.expenses.iter().find(|it| it.id == scaled.id)?;
                Some([
                    stated.display_name().to_owned(),
                    form.money(stated.amount),
                    form.money(scaled.amount),
                ])
            })
            .collect()
    }

    /// What is asked before the ceiling is taken into `plan`.
    #[must_use]
    pub fn take_question(&self, plan: &Plan) -> String {
        format!(
            "Set flexible spending to {} a year? {} against the plan.",
            MoneyForm::Full.money(self.flexible()),
            capitalised(&self.change(plan))
        )
    }
}

/// What is said once a ceiling of `flexible` spending a year is taken into
/// the plan.
#[must_use]
pub fn taken(flexible: Dollars) -> String {
    format!(
        "flexible spending is now {} a year",
        MoneyForm::Full.money(flexible)
    )
}

/// The note under the options, where `plan` asks to leave nothing.
#[must_use]
pub fn note(plan: &Plan) -> Option<&'static str> {
    (plan.market().leave_at_least().is_none()).then_some(SPENDS_IT_ALL)
}

/// The options' columns: what a ceiling was held to, its flexible
/// spending, change and success, then the figures.
#[must_use]
pub fn option_columns() -> Vec<&'static str> {
    OPTION_COLUMNS.into_iter().chain(super::FIGURES).collect()
}

#[cfg(test)]
mod tests {
    use toml::Value;

    use super::*;
    use crate::draft::Draft;
    use crate::setup::examples::named;

    fn retired() -> Plan {
        let (_, _, text) = named("retired-couple.toml").expect("the example");
        let fewer = format!("{text}\n[market.monte_carlo]\ntrials = 100\n");
        Plan::from_toml_str(&fewer).expect("the plan parses")
    }

    fn found(plan: &Plan, target: f64) -> Found {
        let (tables, history) = (TaxTables::embedded(), History::embedded());
        search(plan, &tables, history, target, &Progress::default()).expect("searched")
    }

    #[test]
    fn the_target_is_the_comfortable_share_until_the_form_holds_another() {
        assert_eq!(rate(target_in(toml::Table::new())), "90%");
        let mut draft = Draft::new(retired(), false);
        let target = |draft: &Draft| target_in(draft.answers::<Answers>());
        assert!((target(&draft) - DEFAULT_TARGET).abs() < f64::EPSILON);
        let mut answers = toml::Table::new();
        answers.insert("success".to_owned(), Value::Float(0.8));
        draft.set_answers::<Answers>(answers);
        assert!((target(&draft) - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn a_search_lists_each_ceiling_against_the_plan() {
        let plan = retired();
        let progress = Progress::default();
        let (tables, history) = (TaxTables::embedded(), History::embedded());
        let found = search(&plan, &tables, history, 0.8, &progress).expect("searched");
        assert!(progress.done() <= total(&plan), "{}", progress.done());
        assert_eq!(total(&plan), 1_700);

        let [planned, at_target] = found.listed();
        assert_eq!((planned.key, at_target.key), (PLANNED, AT_TARGET));
        assert_eq!(planned.held_to, "In its own market");
        assert_eq!(at_target.held_to, "In 80% of markets");
        assert!(at_target.success >= 0.8 && planned.success < at_target.success);
        assert!(at_target.flexible() < planned.flexible());

        let stated = plan.flexible_spending();
        assert_eq!(
            stated,
            24_000 + 55_000 + 15_000 + 20_000,
            "the premiums are essential"
        );
        let own = found.plan_cells(&plan, true, MoneyForm::Full);
        assert_eq!(own[..2], ["$114,000".to_owned(), String::new()]);
        assert_eq!(own[2], share(found.plan_success()));
        assert_eq!(own.len(), option_columns().len() - 1);

        let cells = at_target.cells(&plan, true, MoneyForm::Full);
        assert_eq!(cells.len(), own.len());
        assert_eq!(cells[0], MoneyForm::Full.money(at_target.flexible()));
        let percent = (at_target.flexible() - stated) as f64 / stated as f64 * 100.0;
        assert_eq!(cells[1], format!("{:+}%", percent.round()));
        assert_eq!(cells[2], share(at_target.success));
        let summary = at_target.ceiling.judged.projection.summary(true);
        assert_eq!(cells[4], MoneyForm::Full.money(summary.final_net_worth));

        let items = at_target.items(&plan, MoneyForm::Full);
        assert_eq!(items.len(), 4);
        assert_eq!(items[1][..2], ["Living expenses", "$55,000"]);
        let living = &at_target.ceiling.expenses[1];
        assert_eq!(items[1][2], MoneyForm::Full.money(living.amount));

        let money = MoneyForm::Full.money(at_target.flexible());
        assert_eq!(
            at_target.take_question(&plan),
            format!(
                "Set flexible spending to {money} a year? {} against the plan.",
                cells[1]
            )
        );
        assert_eq!(
            taken(at_target.flexible()),
            format!("flexible spending is now {money} a year")
        );
        assert_eq!(note(&plan), Some(SPENDS_IT_ALL));
        assert_eq!(found.listed()[LEADING].key, AT_TARGET);
    }

    #[test]
    fn a_capped_ceiling_says_at_least_and_no_change_says_same() {
        let text = retired().to_toml_string().expect("written");
        let mut plan = Plan::from_toml_str(&text).expect("the plan parses");
        for expense in &mut plan.expenses {
            expense.amount /= 100;
        }
        let found = found(&plan, 0.9);
        let [planned, _] = found.listed();
        assert!(planned.ceiling.is_capped);
        let said = planned.flexible_said(MoneyForm::Full);
        assert_eq!(
            said,
            format!("at least {}", MoneyForm::Full.money(planned.flexible()))
        );
        assert_eq!(planned.change(&plan), "+700%");

        let mut at_plan = found.clone();
        at_plan.planned.expenses = plan
            .expenses
            .iter()
            .filter(|expense| expense.is_flexible())
            .map(|expense| retiretui_engine::optimize::ScaledExpense {
                id: expense.id.clone(),
                amount: expense.amount,
            })
            .collect();
        assert_eq!(at_plan.listed()[0].change(&plan), "same");
    }
}
