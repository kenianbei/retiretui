//! The example plans a new plan can start from: the repository's own, kept
//! beside this module so the crate packages them, each offered under the
//! household it describes.

/// Each example: the file it is kept as, the words it is offered under, and
/// the plan.
pub const EXAMPLES: &[(&str, &str, &str)] = &[
    (
        "starter.toml",
        "Starter: Sam, 30, Texas",
        include_str!("examples/starter.toml"),
    ),
    (
        "mid-career-couple.toml",
        "Mid-career couple: Priya and Marcus, Oregon",
        include_str!("examples/mid-career-couple.toml"),
    ),
    (
        "early-retiree.toml",
        "Early retiree: Morgan, 45, Nevada",
        include_str!("examples/early-retiree.toml"),
    ),
    (
        "public-pension.toml",
        "Public pension: Dana and Chris, Nevada",
        include_str!("examples/public-pension.toml"),
    ),
    (
        "retired-couple.toml",
        "Retired couple: Ruth and Walter, Florida",
        include_str!("examples/retired-couple.toml"),
    ),
    (
        "landlord.toml",
        "Landlord: Gloria, 55, Tennessee",
        include_str!("examples/landlord.toml"),
    ),
    (
        "moving-states.toml",
        "Moving states: Kenji and Maria, Oregon",
        include_str!("examples/moving-states.toml"),
    ),
    (
        "aca-bridge.toml",
        "ACA bridge: Lee and Pat, Wyoming",
        include_str!("examples/aca-bridge.toml"),
    ),
    (
        "market-mix.toml",
        "Market mix: Taylor, 40, South Dakota",
        include_str!("examples/market-mix.toml"),
    ),
    (
        "with-earnings.toml",
        "Earnings record: Robin, 60, New Hampshire",
        include_str!("examples/with-earnings.toml"),
    ),
];

/// The example kept as `file`.
pub fn named(file: &str) -> Option<&'static (&'static str, &'static str, &'static str)> {
    EXAMPLES.iter().find(|(kept, _, _)| *kept == file)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    /// The repository's examples folder, which these are copies of.
    fn repository_examples() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
    }

    #[test]
    fn every_plan_in_the_examples_folder_is_embedded_as_it_stands() {
        let folder = repository_examples();
        let mut plans: Vec<String> = std::fs::read_dir(&folder)
            .unwrap()
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .filter(|name| Path::new(name).extension().is_some_and(|ext| ext == "toml"))
            .filter(|name| {
                let text = std::fs::read_to_string(folder.join(name)).unwrap();
                !text.lines().any(|line| line.starts_with("base ="))
            })
            .collect();
        plans.sort();
        let mut embedded: Vec<&str> = EXAMPLES.iter().map(|(file, _, _)| *file).collect();
        embedded.sort_unstable();
        assert_eq!(plans, embedded, "every plan, and no scenario, is offered");
        for (file, _, text) in EXAMPLES {
            let kept = std::fs::read_to_string(folder.join(file)).unwrap();
            assert_eq!(*text, kept, "{file} has drifted from the examples folder");
        }
    }
}
