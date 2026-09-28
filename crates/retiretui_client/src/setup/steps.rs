//! The new-plan questions in the steps a page asks them in.

/// One step of the new-plan questions.
#[derive(Debug)]
pub struct Step {
    /// How the step is addressed.
    pub slug: &'static str,
    /// What the step is headed.
    pub title: &'static str,
    /// The keys of the fields it asks, in the form's order.
    pub keys: &'static [&'static str],
}

/// Every step, in order, holding every field but the example's.
pub const STEPS: &[Step] = &[
    Step {
        slug: "household",
        title: "Your household",
        keys: &["filing", "stage"],
    },
    Step {
        slug: "you",
        title: "About you",
        keys: &[
            "name",
            "birth_year",
            "retirement_age",
            "working_since",
            "salary",
            "social_security",
            "claim_age",
        ],
    },
    Step {
        slug: "partner",
        title: "About your partner",
        keys: &[
            "partner_name",
            "partner_birth_year",
            "partner_retirement_age",
            "partner_working_since",
            "partner_salary",
            "partner_social_security",
            "partner_claim_age",
        ],
    },
];

#[cfg(test)]
mod tests {
    use toml::Table;

    use retiretui_engine::tax::FIRST_WORKING_AGE;

    use super::STEPS;
    use crate::setup::FIELDS;
    use crate::setup::generate::{DEFAULT_CLAIM_AGE, DEFAULT_RETIREMENT_AGE};

    #[test]
    fn the_steps_hold_every_answer_once_in_the_form_s_order() {
        let stepped: Vec<&str> = STEPS.iter().flat_map(|step| step.keys).copied().collect();
        let answered: Vec<&str> = FIELDS[1..].iter().map(|spec| spec.key).collect();
        assert_eq!(FIELDS[0].key, "example");
        assert_eq!(stepped, answered);
    }

    fn shown(filing: &str) -> Vec<&'static str> {
        let answers: Table = toml::from_str(&format!("filing = \"{filing}\"")).expect("parses");
        let partner = STEPS.iter().find(|step| step.slug == "partner");
        let partner = partner.expect("a partner step").keys;
        FIELDS
            .iter()
            .filter(|spec| partner.contains(&spec.key) && spec.is_shown_for(&answers))
            .map(|spec| spec.key)
            .collect()
    }

    #[test]
    fn only_a_joint_filer_is_asked_about_a_partner() {
        assert!(shown("single").is_empty());
        assert_eq!(shown("married-joint").len(), 7);
    }

    #[test]
    fn a_blank_reads_as_what_the_plan_is_made_with() {
        let blank = |key: &str| {
            let spec = FIELDS.iter().find(|spec| spec.key == key);
            spec.expect("a field").blank_word()
        };
        assert_eq!(blank("retirement_age"), DEFAULT_RETIREMENT_AGE.to_string());
        assert_eq!(blank("claim_age"), DEFAULT_CLAIM_AGE.to_string());
        assert_eq!(blank("partner_claim_age"), DEFAULT_CLAIM_AGE.to_string());
        assert!(blank("working_since").ends_with(&FIRST_WORKING_AGE.to_string()));
        assert!(blank("partner_working_since").ends_with(&FIRST_WORKING_AGE.to_string()));
    }
}
