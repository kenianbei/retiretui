use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use retiretui_engine::plan::resolve::{ResolveError, resolve_plan};

use super::{Position, position, said, text};

const STARTER: &str = include_str!("../setup/examples/starter.toml");

/// What opening `/plans/plan.toml` among `files` fails with.
fn refused(files: &[(&str, &str)]) -> ResolveError {
    let files: BTreeMap<PathBuf, String> = files
        .iter()
        .map(|(name, text)| (Path::new("/plans").join(name), (*text).to_owned()))
        .collect();
    let start = PathBuf::from("/plans/plan.toml");
    let text = files[&start].clone();
    let mut read = |file: &Path| {
        files
            .get(file)
            .cloned()
            .ok_or_else(|| "no such file".to_owned())
    };
    let mut locate = |referrer: &Path, base: &str| Ok(referrer.parent().unwrap().join(base));
    resolve_plan(start, text, &mut read, &mut locate).unwrap_err()
}

fn scenario_over(base: &str, rest: &str) -> String {
    format!("base = \"{base}\"\nschema = 1\n{rest}")
}

#[test]
fn a_syntax_error_is_said_at_its_line_and_column() {
    let broken = "schema = 1\n\n[household]\nfiling\n";
    let error = refused(&[("plan.toml", broken)]);
    assert_eq!(position(&error), Some(Position { line: 4, column: 7 }));
    assert_eq!(
        said(&error),
        "plan.toml, line 4, column 7: key with no value, expected `=`"
    );
    assert_eq!(text(&error), Some(broken));
    let dump = error.to_string();
    assert!(
        dump.starts_with("/plans/plan.toml: TOML parse error at line 4, column 7\n"),
        "{dump}"
    );
    assert!(dump.contains('^'), "{dump}");
}

#[test]
fn a_column_counts_characters_not_bytes() {
    let broken = "schema = 1\nname = \"é\" x\n";
    let error = refused(&[("plan.toml", broken)]);
    assert_eq!(
        position(&error),
        Some(Position {
            line: 2,
            column: 12
        })
    );
}

#[test]
fn a_misspelt_field_is_said_by_its_name() {
    let misspelt = STARTER.replacen("[household]", "[household]\nfilng = 1", 1);
    let error = refused(&[("plan.toml", &misspelt)]);
    assert_eq!(
        said(&error),
        "plan.toml, line 17, column 1: unknown field `filng`, expected `filing` or `people`"
    );
}

#[test]
fn a_base_that_breaks_is_named_with_its_own_text() {
    let error = refused(&[
        ("plan.toml", &scenario_over("base.toml", "")),
        ("base.toml", "schema = = 1\n"),
    ]);
    assert_eq!(error.file, Path::new("/plans/base.toml"));
    assert_eq!(text(&error), Some("schema = = 1\n"));
    assert!(said(&error).starts_with("base.toml, line 1, column "));
}

#[test]
fn a_merged_scenario_names_no_line() {
    let overlay = scenario_over("base.toml", "[household]\nfiling_status = 3\n");
    let error = refused(&[("plan.toml", &overlay), ("base.toml", STARTER)]);
    assert_eq!(position(&error), None);
    assert_eq!(text(&error), None);
    assert_eq!(
        said(&error),
        "plan.toml, with its scenarios applied: unknown field `filing_status`, expected \
         `filing` or `people`"
    );
    assert!(error.to_string().starts_with("TOML parse error"), "{error}");
}

#[test]
fn a_missing_base_is_said_as_not_read() {
    let error = refused(&[("plan.toml", &scenario_over("gone.toml", ""))]);
    assert_eq!(said(&error), "gone.toml could not be read");
    assert_eq!(error.to_string(), "no such file");
    assert_eq!(text(&error), None);
}

#[test]
fn a_chain_that_loops_is_said_as_one() {
    let error = refused(&[
        ("plan.toml", &scenario_over("other.toml", "")),
        ("other.toml", &scenario_over("plan.toml", "")),
    ]);
    assert_eq!(
        said(&error),
        "plan.toml's scenario chain loops back on itself"
    );
    assert_eq!(
        error.to_string(),
        "/plans/plan.toml: scenario base chain forms a cycle"
    );
}

#[test]
fn a_base_that_is_no_file_name_is_said_without_the_text() {
    let error = refused(&[("plan.toml", "base = 3\nschema = 1\n")]);
    assert_eq!(said(&error), "plan.toml: `base` must be a non-empty string");
    assert_eq!(text(&error), None);
    assert_eq!(
        error.to_string(),
        "/plans/plan.toml: `base` must be a non-empty string"
    );
}
