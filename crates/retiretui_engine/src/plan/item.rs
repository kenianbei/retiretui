use std::collections::HashSet;

use super::{Account, Cliff, Contribution, Conversion, Event, Expense, Income, Person, Transfer};

/// An item a plan lists and other items and scenarios refer to by id.
pub trait Item {
    /// The id the item is referred to by.
    fn id(&self) -> &str;

    /// The name the plan gives the item, if any.
    fn name(&self) -> Option<&str>;

    /// What the item is called where it is shown: its name, or its id where
    /// it has none.
    fn display_name(&self) -> &str {
        self.name().unwrap_or_else(|| self.id())
    }
}

macro_rules! impl_item {
    ($($item:ty),*) => {$(
        impl Item for $item {
            fn id(&self) -> &str {
                &self.id
            }

            fn name(&self) -> Option<&str> {
                self.name.as_deref()
            }
        }
    )*};
}

impl_item!(
    Account,
    Cliff,
    Contribution,
    Conversion,
    Event,
    Expense,
    Income,
    Person,
    Transfer
);

/// The first `{prefix}-n`, counting from 1, that none of `taken` is.
#[must_use]
pub fn fresh_id<'a>(prefix: &str, taken: impl IntoIterator<Item = &'a str>) -> String {
    let taken: HashSet<&str> = taken.into_iter().collect();
    let mut n = 1;
    loop {
        let id = format!("{prefix}-{n}");
        if !taken.contains(id.as_str()) {
            return id;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_id_counts_from_one_past_what_is_taken() {
        assert_eq!(fresh_id("account", []), "account-1");
        assert_eq!(
            fresh_id("account", ["account-1", "account-3", "cash"]),
            "account-2"
        );
        assert_eq!(fresh_id("alice", ["alice-1", "alice-2"]), "alice-3");
    }
}
