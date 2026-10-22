//! The closed vocabulary a transaction is sorted into: what a household
//! spends on, and the two kinds of row that are never spending.

use serde::{Deserialize, Serialize};

use crate::plan::Expense;

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
        !matches!(self, Self::Income | Self::Transfer)
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
            Self::Fuel => &["fuel", "gas", "gasoline", "automotive", "auto"],
            Self::Transit => &[
                "transit",
                "transport",
                "transportation",
                "commute",
                "parking",
                "tolls",
            ],
            Self::Insurance => &["insurance"],
            Self::Medical => &["medical", "health", "doctor", "dental", "health & wellness"],
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
    /// equals, else the first a word of is found in.
    #[must_use]
    pub fn from_text(text: &str) -> Option<Self> {
        let text = text.trim().to_lowercase();
        if text.is_empty() {
            return None;
        }
        let exact = Self::ALL
            .iter()
            .find(|c| c.words().contains(&text.as_str()));
        exact
            .or_else(|| {
                Self::ALL
                    .iter()
                    .find(|c| c.words().iter().any(|w| text.contains(w)))
            })
            .copied()
    }

    /// Whether the expense is this category's: its id or name is one of
    /// its words, or holds one.
    #[must_use]
    pub fn matches(self, expense: &Expense) -> bool {
        let id = expense.id.replace('-', " ").to_lowercase();
        let name = expense.name.as_deref().unwrap_or_default().to_lowercase();
        let words = self.words();
        words.contains(&id.as_str())
            || words.contains(&name.as_str())
            || words.iter().any(|w| id.contains(w) || name.contains(w))
    }
}

#[cfg(test)]
mod tests {
    use super::Category;
    use crate::plan::Plan;

    #[test]
    fn a_banks_category_text_reads_exactly_before_by_part() {
        assert_eq!(Category::from_text("Food & Drink"), Some(Category::Dining));
        assert_eq!(Category::from_text("Food"), Some(Category::Groceries));
        assert_eq!(
            Category::from_text("Bills & Utilities"),
            Some(Category::Utilities)
        );
        assert_eq!(Category::from_text("Gas"), Some(Category::Fuel));
        assert_eq!(Category::from_text("Somesuch"), None);
        assert_eq!(Category::from_text(""), None);
    }

    #[test]
    fn an_expense_matches_by_id_or_name() {
        let plan = Plan::from_toml_str(
            r#"
schema = 1
[plan]
start_year = 2026
horizon_age = 90
inflation = 0.03
[household]
filing = "single"
[[household.people]]
id = "me"
birth = 1960-01-01
[[expenses]]
id = "groceries"
amount = 1
[[expenses]]
id = "eating-out"
amount = 1
[[expenses]]
id = "x"
name = "Cell phone"
amount = 1
"#,
        )
        .unwrap();
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
}
