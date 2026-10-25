//! The closed vocabulary a transaction is sorted into: what a household
//! spends on, and the two kinds of row that are never spending.

use serde::{Deserialize, Serialize};

use crate::plan::Expense;

/// What may follow a word for an expense or a bank to be taken as saying it.
const PLURAL_ENDINGS: [&str; 3] = ["", "s", "es"];

/// Categories that land in one another's expense where they have none of
/// their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs, reason = "each variant is its own word")]
pub enum Family {
    Food,
    Home,
    Health,
    Transport,
    Personal,
}

/// What a row was for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
#[allow(missing_docs, reason = "each variant is its own word")]
pub enum Category {
    Groceries,
    Dining,
    Utilities,
    Housing,
    Fuel,
    Transit,
    Insurance,
    Medical,
    Pharmacy,
    Subscriptions,
    Phone,
    Internet,
    Shopping,
    Clothing,
    Travel,
    Entertainment,
    Gifts,
    Pets,
    Childcare,
    Education,
    Fees,
    /// Money coming in: a paycheck, a benefit, a refund of tax.
    Income,
    /// Money moved, not spent: a card payment, a transfer, cash out.
    Transfer,
}

impl Category {
    /// Every category, spending first, in the order proposals are listed.
    pub const ALL: [Self; 23] = [
        Self::Groceries,
        Self::Dining,
        Self::Utilities,
        Self::Housing,
        Self::Fuel,
        Self::Transit,
        Self::Insurance,
        Self::Medical,
        Self::Pharmacy,
        Self::Subscriptions,
        Self::Phone,
        Self::Internet,
        Self::Shopping,
        Self::Clothing,
        Self::Travel,
        Self::Entertainment,
        Self::Gifts,
        Self::Pets,
        Self::Childcare,
        Self::Education,
        Self::Fees,
        Self::Income,
        Self::Transfer,
    ];

    /// The slug: the file's spelling, and the id of an expense proposed
    /// for it.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        self.words()[0]
    }

    /// The slug with its first letter raised: the name of an expense
    /// proposed for it.
    #[must_use]
    pub fn title(self) -> String {
        let slug = self.slug();
        let mut title = String::with_capacity(slug.len());
        title.extend(slug.chars().next().map(|c| c.to_ascii_uppercase()));
        title.push_str(&slug[1..]);
        title
    }

    /// Whether rows of this category are spending the plan could hold an
    /// expense for.
    #[must_use]
    pub const fn is_spending(self) -> bool {
        self.family().is_some()
    }

    /// The slug first, then what else a bank or a plan calls it.
    const fn words(self) -> &'static [&'static str] {
        match self {
            Self::Groceries => &["groceries", "grocery", "supermarket", "food"],
            Self::Dining => &[
                "dining",
                "restaurant",
                "food & drink",
                "eating out",
                "takeout",
                "coffee",
            ],
            Self::Utilities => &[
                "utilities",
                "utility",
                "bills & utilities",
                "electric",
                "water",
                "power",
            ],
            Self::Housing => &["housing", "rent", "mortgage", "home", "hoa"],
            Self::Fuel => &["fuel", "gas", "gasoline", "automotive"],
            Self::Transit => &[
                "transit",
                "transport",
                "transportation",
                "commute",
                "parking",
                "tolls",
            ],
            Self::Insurance => &["insurance"],
            Self::Medical => &["medical", "health", "healthcare", "doctor", "dental"],
            Self::Pharmacy => &["pharmacy", "drugstore", "prescriptions", "drugs"],
            Self::Subscriptions => &["subscriptions", "subscription", "streaming"],
            Self::Phone => &["phone", "mobile", "cell", "wireless"],
            Self::Internet => &["internet", "broadband", "cable"],
            Self::Shopping => &["shopping", "merchandise", "personal", "household"],
            Self::Clothing => &["clothing", "clothes", "apparel"],
            Self::Travel => &["travel", "vacation", "hotel", "airfare"],
            Self::Entertainment => &["entertainment", "recreation", "hobbies", "fun"],
            Self::Gifts => &["gifts", "gift", "donations", "charity", "giving"],
            Self::Pets => &["pets", "pet"],
            Self::Childcare => &["childcare", "child care", "daycare", "kids"],
            Self::Education => &["education", "tuition", "school"],
            Self::Fees => &["fees", "fee", "bank charges", "interest charge"],
            Self::Income => &["income", "paycheck", "salary", "deposit"],
            Self::Transfer => &["transfer", "payment", "credit card payment"],
        }
    }

    /// The category a bank's own category text names: the one a word of
    /// equals, else the first a word of is found in as whole words.
    #[must_use]
    pub fn from_text(text: &str) -> Option<Self> {
        let text = text.trim().to_lowercase();
        if text.is_empty() {
            return None;
        }
        let exact = Self::ALL
            .iter()
            .find(|c| c.words().contains(&text.as_str()));
        let held = words_in(&text);
        exact
            .or_else(|| {
                Self::ALL
                    .iter()
                    .find(|c| c.words().iter().any(|w| holds(&held, w)))
            })
            .copied()
    }

    /// The family a spending category folds into an expense by; none for
    /// income and transfers.
    #[must_use]
    pub const fn family(self) -> Option<Family> {
        Some(match self {
            Self::Groceries | Self::Dining => Family::Food,
            Self::Housing | Self::Utilities | Self::Phone | Self::Internet => Family::Home,
            Self::Medical | Self::Pharmacy | Self::Insurance => Family::Health,
            Self::Fuel | Self::Transit | Self::Travel => Family::Transport,
            Self::Shopping
            | Self::Clothing
            | Self::Entertainment
            | Self::Subscriptions
            | Self::Gifts
            | Self::Pets
            | Self::Childcare
            | Self::Education
            | Self::Fees => Family::Personal,
            Self::Income | Self::Transfer => return None,
        })
    }

    /// Whether the expense is this category's: its id or name holds one of
    /// its words.
    #[must_use]
    pub fn matches(self, expense: &Expense) -> bool {
        names_any(expense, self.words())
    }
}

/// Whether the expense's id or its name holds one of `words` as whole
/// words.
pub(super) fn names_any(expense: &Expense, words: &[&str]) -> bool {
    let id = words_in(&expense.id);
    let name = words_in(expense.name.as_deref().unwrap_or_default());
    words.iter().any(|w| holds(&id, w) || holds(&name, w))
}

/// The words of `text`, lowercase: each a run of letters, digits or `&`.
fn words_in(text: &str) -> Vec<String> {
    let text = text.to_lowercase();
    let words = text.split(|c: char| !c.is_alphanumeric() && c != '&');
    words.filter(|w| !w.is_empty()).map(str::to_owned).collect()
}

/// Whether `held` holds the words of `entry` in a run, each word or its
/// plural.
fn holds(held: &[String], entry: &str) -> bool {
    let entry = words_in(entry);
    let is_word = |held: &String, word: &String| {
        let rest = held.strip_prefix(word.as_str());
        rest.is_some_and(|rest| PLURAL_ENDINGS.contains(&rest))
    };
    held.windows(entry.len()).any(|run| {
        run.iter()
            .zip(&entry)
            .all(|(held, word)| is_word(held, word))
    })
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::{Category, names_any};
    use crate::plan::Plan;

    #[test]
    fn a_category_and_its_kin_share_a_family() {
        assert_eq!(Category::Dining.family(), Category::Groceries.family());
        assert_eq!(Category::Transfer.family(), None);
    }

    #[test]
    fn a_banks_category_text_reads_exactly_before_by_part() {
        assert_eq!(Category::from_text("Food & Drink"), Some(Category::Dining));
        assert_eq!(Category::from_text("Food"), Some(Category::Groceries));
        assert_eq!(
            Category::from_text("Bills & Utilities"),
            Some(Category::Utilities)
        );
        assert_eq!(Category::from_text("Gas"), Some(Category::Fuel));
        assert_eq!(
            Category::from_text("Fees & Adjustments"),
            Some(Category::Fees)
        );
        assert_eq!(
            Category::from_text("Supermarkets"),
            Some(Category::Groceries)
        );
        assert_eq!(
            Category::from_text("Fun Money"),
            Some(Category::Entertainment)
        );
        assert_eq!(
            Category::from_text("Funds Transfer"),
            Some(Category::Transfer)
        );
        assert_eq!(Category::from_text("Somesuch"), None);
        assert_eq!(Category::from_text(""), None);
    }

    /// A plan holding `expenses`, each `(id, name)` with an amount of 1.
    fn plan_of(expenses: &[(&str, &str)]) -> Plan {
        let mut text = "schema = 1\n[plan]\nstart_year = 2026\nhorizon_age = 90\ninflation = 0.03\n[household]\nfiling = \"single\"\n[[household.people]]\nid = \"me\"\nbirth = 1960-01-01\n".to_owned();
        for (id, name) in expenses {
            writeln!(
                text,
                "[[expenses]]\nid = \"{id}\"\nname = \"{name}\"\namount = 1"
            )
            .unwrap();
        }
        Plan::from_toml_str(&text).unwrap()
    }

    #[test]
    fn an_expense_matches_by_id_or_name() {
        let plan = plan_of(&[("groceries", ""), ("eating-out", ""), ("x", "Cell phone")]);
        let [groceries, eating_out, phone] = &plan.expenses[..] else {
            panic!()
        };
        assert!(Category::Groceries.matches(groceries));
        assert!(!Category::Dining.matches(groceries));
        assert!(Category::Dining.matches(eating_out));
        assert!(Category::Phone.matches(phone));
        assert_eq!(Category::Groceries.title(), "Groceries");
        assert_eq!(Category::Transfer.slug(), "transfer");
    }

    #[test]
    fn a_word_matches_whole_or_as_its_plural_never_within_another() {
        let plan = plan_of(&[
            ("coffee", "Coffee"),
            ("college-fund", ""),
            ("auto-insurance", ""),
            ("care", "Mother's care"),
            ("restaurants", ""),
        ]);
        let [coffee, fund, insurance, care, restaurants] = &plan.expenses[..] else {
            panic!()
        };
        assert!(Category::Dining.matches(coffee) && !Category::Fees.matches(coffee));
        assert!(!Category::Entertainment.matches(fund));
        assert!(Category::Insurance.matches(insurance) && !Category::Fuel.matches(insurance));
        assert!(!names_any(care, &["other"]));
        assert!(Category::Dining.matches(restaurants));
    }
}
