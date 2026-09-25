//! The root README stays true to the binary: its command table lists exactly
//! the subcommands, and the demo tape opens a path that exists.

mod common;

use std::fs;

use common::{retiretui, workspace_root};

const COMMANDS_HEADING: &str = "Commands:";
const HELP_COMMAND: &str = "help";
const TABLE_ROW_PREFIX: &str = "| `";
const TAPE_TUI_PREFIX: &str = "Type \"retiretui tui ";

fn subcommands() -> Vec<String> {
    let help = String::from_utf8(retiretui(&["--help"]).stdout).unwrap();
    let mut names: Vec<String> = help
        .lines()
        .skip_while(|line| *line != COMMANDS_HEADING)
        .skip(1)
        .take_while(|line| line.starts_with(' '))
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| *name != HELP_COMMAND)
        .map(str::to_owned)
        .collect();
    assert!(!names.is_empty(), "no subcommands in:\n{help}");
    names.sort();
    names
}

fn readme_commands() -> Vec<String> {
    let readme = fs::read_to_string(workspace_root().join("README.md")).unwrap();
    let mut names: Vec<String> = readme
        .lines()
        .filter_map(|line| line.strip_prefix(TABLE_ROW_PREFIX))
        .filter_map(|rest| rest.split_once('`'))
        .map(|(name, _)| name.to_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn readme_lists_exactly_the_subcommands() {
    assert_eq!(readme_commands(), subcommands());
}

#[test]
fn demo_tape_opens_an_existing_path() {
    let tape = fs::read_to_string(workspace_root().join("assets/demo.tape")).unwrap();
    let path = tape
        .lines()
        .find_map(|line| line.strip_prefix(TAPE_TUI_PREFIX))
        .and_then(|rest| rest.strip_suffix('"'))
        .expect("the tape types no `retiretui tui` command");
    assert!(
        workspace_root().join(path).exists(),
        "{path} does not exist"
    );
}
