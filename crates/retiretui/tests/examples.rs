//! The plans in `examples/` stay runnable: each projects, each command its
//! header suggests succeeds, and the folder's README names each.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::workspace_root;

/// How many examples ship, so a glob that finds nothing, or a deleted example,
/// cannot pass.
const EXAMPLE_COUNT: usize = 13;
const TRY_PREFIX: &str = "# try: retiretui ";
/// Needs a terminal, so a suggestion cannot be run by a test.
const INTERACTIVE_COMMAND: &str = "tui";
/// Would write into `examples/` if a suggestion carried it.
const WRITE_FLAG: &str = "--write";

fn examples_dir() -> PathBuf {
    workspace_root().join("examples")
}

fn examples() -> Vec<String> {
    let folder = fs::read_dir(examples_dir()).unwrap();
    let mut names: Vec<String> = folder
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| Path::new(name).extension().is_some_and(|ext| ext == "toml"))
        .collect();
    names.sort();
    assert_eq!(names.len(), EXAMPLE_COUNT, "{names:?}");
    names
}

fn read_example(name: &str) -> String {
    fs::read_to_string(examples_dir().join(name)).unwrap()
}

/// Runs the binary from the workspace root, as a reader of the examples would.
fn assert_runs(args: &[&str]) {
    let output = Command::new(env!("CARGO_BIN_EXE_retiretui"))
        .current_dir(workspace_root())
        .args(args)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "`{}` failed: {stderr}",
        args.join(" ")
    );
    assert!(
        !output.stdout.is_empty(),
        "`{}` printed nothing",
        args.join(" ")
    );
}

#[test]
fn every_example_projects() {
    for name in examples() {
        assert_runs(&["project", &format!("examples/{name}")]);
    }
}

#[test]
fn every_suggested_command_runs() {
    for name in examples() {
        let text = read_example(&name);
        let suggestions: Vec<&str> = text
            .lines()
            .filter_map(|line| line.strip_prefix(TRY_PREFIX))
            .collect();
        assert!(!suggestions.is_empty(), "{name} suggests no command");
        for suggestion in suggestions {
            let args: Vec<&str> = suggestion.split_whitespace().collect();
            assert!(
                args[0] != INTERACTIVE_COMMAND && !args.contains(&WRITE_FLAG),
                "{name}: {suggestion}"
            );
            assert_runs(&args);
        }
    }
}

#[test]
fn examples_readme_names_every_example() {
    let readme = read_example("README.md");
    for name in examples() {
        assert!(
            readme.contains(&format!("`{name}`")),
            "{name} is not listed"
        );
    }
}
