use std::collections::BTreeSet;

use bevy_ecs::prelude::Entity;
use bevy_input::keyboard::{KeyCode, NativeKeyCode};
use tracing::Level;

use super::*;
use crate::journal::Journal;
use crate::session::{Basis, LedgerRun, Projected};
use crate::support::{
    Headless, SIZE, headless_app_bound, picker_rows, press_ctrl, press_key, redrawn, scratch_plan,
    show, type_text,
};

fn bound(keys: &str) -> (Keymap, Remarks) {
    Keymap::with(&toml::from_str(keys).unwrap())
}

fn run_by(keymap: &Keymap, page: Page, key: &str) -> Option<&'static str> {
    let KeyBinding { key, modifiers } = keys::parse(key).unwrap();
    let pressed = KeyboardInput {
        key_code: KeyCode::Unidentified(NativeKeyCode::Unidentified),
        logical_key: key,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    };
    let command = keymap.bound_on(Some(page), &pressed, modifiers);
    command.map(|command| command.spec().name)
}

/// A key two commands answer to where one page shows both, and the two.
fn clash(keymap: &Keymap) -> Option<(String, &'static str, &'static str)> {
    let bindings = &keymap.bindings;
    bindings
        .iter()
        .enumerate()
        .find_map(|(at, (key, command))| {
            let shares = |(held, other): &&(KeyBinding, CommandId)| {
                held == key
                    && other != command
                    && !are_apart(command.spec().scope, other.spec().scope)
            };
            let (_, earlier) = bindings[..at].iter().find(shares)?;
            Some((keys::label(key), earlier.spec().name, command.spec().name))
        })
}

#[test]
fn names_are_unique_and_keys_are_unique_on_any_one_page() {
    let names: BTreeSet<&str> = COMMANDS.iter().map(|spec| spec.name).collect();
    assert_eq!(names.len(), COMMANDS.len());
    assert_eq!(clash(&Keymap::defaults()), None);
}

#[test]
fn a_pages_row_takes_a_key_the_shell_binds_while_the_page_is_shown() {
    let keymap = Keymap::defaults();
    assert_eq!(run_by(&keymap, Page::Ledger, "esc"), Some("ledger-plan"));
    assert_eq!(run_by(&keymap, Page::Accounts, "esc"), Some("domains"));
}

#[test]
fn a_key_two_pages_bind_runs_the_command_of_the_page_on_show() {
    let keymap = Keymap::defaults();
    let on = |page: Page| run_by(&keymap, page, "w");
    assert_eq!(on(Page::RothConversions), Some("write-ladder"));
    assert_eq!(on(Page::SsaBenefits), Some("write-claims"));
    assert_eq!(on(Page::Overview), None, "a page's key is its own");
}

#[test]
fn a_stated_command_answers_to_the_keys_stated_and_to_no_other() {
    let (keymap, remarks) = bound("save = [\"ctrl-w\", \"f2\"]\nsort = []");
    assert_eq!(remarks, Remarks::default());
    let on = |key: &str| run_by(&keymap, Page::Accounts, key);
    assert_eq!(on("ctrl-w"), Some("save"));
    assert_eq!(on("f2"), Some("save"));
    assert_eq!(on("ctrl-s"), None, "the default went with the entry");
    assert_eq!(
        keymap.label_named("save", 0),
        "ctrl-w",
        "the first is shown"
    );
    assert_eq!(on("s"), None, "an empty entry unbinds");
    assert_eq!(keymap.label_named("sort", 0), "");
}

#[test]
fn a_key_a_command_holds_unstated_is_taken_from_it_and_said() {
    let (keymap, remarks) = bound("save = \"q\"");
    let on = |key: &str| run_by(&keymap, Page::Overview, key);
    assert_eq!(on("q"), Some("save"));
    assert_eq!(on("ctrl-q"), Some("quit"), "the loser keeps its others");
    assert_eq!(keymap.label_named("quit", 0), "ctrl-q");
    assert_eq!(remarks.taken, ["[tui.keys] q now runs save, not quit"]);
    assert_eq!(remarks.refused, [""; 0]);

    let (_, moved) = bound("save = \"q\"\nquit = \"ctrl-q\"");
    assert_eq!(moved, Remarks::default(), "the user moved quit themselves");
}

#[test]
fn a_key_stated_twice_is_the_earlier_commands_and_the_later_keeps_its_own() {
    let (keymap, remarks) = bound("reload = [\"x\", \"y\"]\nsave = \"x\"");
    let on = |key: &str| run_by(&keymap, Page::Overview, key);
    assert_eq!(on("x"), Some("save"), "save is the earlier in the table");
    assert_eq!(on("r"), Some("reload"));
    assert_eq!(on("y"), None, "the entry is left out whole");
    assert_eq!(remarks.refused, ["[tui.keys] reload: x is kept for save"]);
    assert_eq!(remarks.taken, [""; 0]);
}

#[test]
fn a_page_and_the_shell_may_hold_one_key_and_the_page_runs_it_while_shown() {
    for (keys, key) in [("overview-chart = \"q\"", "q"), ("quit = \"v\"", "v")] {
        let (keymap, remarks) = bound(keys);
        assert_eq!(remarks, Remarks::default(), "{keys}");
        assert_eq!(run_by(&keymap, Page::Overview, key), Some("overview-chart"));
        assert_eq!(run_by(&keymap, Page::Ledger, key), Some("quit"));
    }
}

#[test]
fn the_palette_is_never_left_without_a_key() {
    for keys in ["palette = []", "save = \":\""] {
        let (keymap, remarks) = bound(keys);
        assert_eq!(run_by(&keymap, Page::Overview, ":"), Some("palette"));
        assert_eq!(run_by(&keymap, Page::Overview, "ctrl-s"), Some("save"));
        assert!(
            matches!(&remarks.refused[..], [said] if said.contains("palette")),
            "{keys}: {remarks:?}"
        );
        assert_eq!(remarks.taken, [""; 0], "{keys}");
    }

    let (keymap, remarks) = bound("save = \":\"\npalette = \"p\"");
    assert_eq!(remarks, Remarks::default(), "the palette moved first");
    assert_eq!(run_by(&keymap, Page::Overview, ":"), Some("save"));
    assert_eq!(run_by(&keymap, Page::Overview, "p"), Some("palette"));

    let (keymap, remarks) = bound("save = \"p\"\npalette = \"p\"");
    assert_eq!(run_by(&keymap, Page::Overview, "p"), Some("palette"));
    assert_eq!(remarks.refused, ["[tui.keys] save: p is kept for palette"]);
}

#[test]
fn an_entry_that_does_not_read_is_left_out_whole_and_said() {
    let (keymap, remarks) =
        bound("savee = \"x\"\nsave = [\"ctrl-w\", \"ctrl+s\"]\nreload = 3\nundo = [1]");
    let on = |key: &str| run_by(&keymap, Page::Overview, key);
    assert_eq!(on("ctrl-w"), None, "one bad key refuses the entry");
    assert_eq!(on("ctrl-s"), Some("save"));
    assert_eq!(on("r"), Some("reload"));
    assert_eq!(on("ctrl-z"), Some("undo"));
    assert_eq!(
        remarks.refused,
        [
            "[tui.keys] savee: no command has that name",
            "[tui.keys] save: \"ctrl+s\" is not a key",
            "[tui.keys] reload: a key or a list of keys is wanted",
            "[tui.keys] undo: a key or a list of keys is wanted",
        ]
    );
}

#[test]
fn an_arrow_a_page_command_is_given_is_one_the_page_binds() {
    let bound_on = |keymap: &Keymap| -> Vec<Key> {
        keymap.arrows_bound_on(Page::TaxTables).cloned().collect()
    };
    assert_eq!(bound_on(&Keymap::defaults()), []);
    let (keymap, _) = bound("tax-year-next = \"right\"\ntax-year-previous = \"ctrl-left\"");
    assert_eq!(bound_on(&keymap), [Key::ArrowRight], "a chord is no walk");
}

#[test]
fn whatever_is_stated_no_key_runs_two_commands_where_one_page_shows_both() {
    for keys in [
        "save = \"q\"",
        "reload = \"x\"\nsave = \"x\"",
        "save = [\"r\", \"n\", \"q\", \"i\"]",
        "add = \"q\"\ndelete = \"q\"\nsort = \"a\"",
        "overview-chart = \"e\"\noverview-edit = \"v\"",
        "compare-view = \"c\"",
        "save = \":\"\nhelp = \":\"",
    ] {
        let (keymap, _) = bound(keys);
        assert_eq!(clash(&keymap), None, "{keys}");
        assert_ne!(keymap.label_named("palette", 0), "", "{keys}");
    }
}

fn app_bound(keys: &str) -> Headless {
    headless_app_bound(scratch_plan(), SIZE, keys)
}

fn key_row(keys: &str, page: Page) -> String {
    let mut app = app_bound(keys);
    show(&mut app, page);
    let frame = redrawn(&mut app);
    frame.lines().last().unwrap().to_owned()
}

#[test]
fn a_hint_for_a_command_names_the_key_the_user_gave_it() {
    for page in [Page::Accounts, Page::Household] {
        assert!(
            key_row("domains = \"h\"", page).contains("h domains"),
            "{page:?}"
        );
        assert!(
            !key_row("domains = []", page).contains("domains"),
            "{page:?}"
        );
    }
    let two = "domains = [\"h\", \"j\"]";
    assert!(
        key_row(two, Page::Accounts).contains("j domains"),
        "a table names the second"
    );
    assert!(
        key_row(two, Page::Household).contains("h domains"),
        "a lone form the first"
    );
}

#[test]
fn a_hint_for_two_commands_runs_their_keys_together_and_drops_one_unbound() {
    let stepped = "overview-year-previous = \"[\"\noverview-year-next = \"]\"";
    assert!(key_row(stepped, Page::Overview).contains("[] year"));
    assert!(key_row("overview-year-next = []", Page::Overview).contains("← year"));
    let neither = "overview-year-previous = []\noverview-year-next = []";
    assert!(!key_row(neither, Page::Overview).contains("year"));
    let compared = "compare-open = \"o\"\ncompare-metric-previous = []";
    let row = key_row(compared, Page::Compare);
    assert!(row.contains("o open") && row.contains("→ metric"), "{row}");
}

#[test]
fn a_page_names_the_users_key_in_what_it_says_of_itself() {
    let said_of = |keys: &str| {
        let mut app = app_bound(keys);
        let shown = app.world().resource::<Projected>();
        let run = Projected {
            plan: shown.plan.clone(),
            projection: shown.projection.clone(),
        };
        app.world_mut().resource_mut::<LedgerRun>().0 = Some(("a run".to_owned(), run));
        show(&mut app, Page::Ledger);
        let ledger = redrawn(&mut app);
        show(&mut app, Page::Compare);
        (ledger, redrawn(&mut app))
    };
    let (ledger, compare) = said_of("ledger-plan = \"f9\"\ncompare-with = \"+\"");
    assert!(
        ledger.contains("a run · f9 returns to the plan"),
        "{ledger}"
    );
    assert!(compare.contains("in its folder: + adds one."), "{compare}");
    let (ledger, compare) = said_of("ledger-plan = []\ncompare-with = []");
    assert!(
        ledger.contains("a run ") && !ledger.contains("returns"),
        "{ledger}"
    );
    assert!(compare.contains("in its folder. "), "{compare}");
}

#[test]
fn a_rebound_command_runs_from_the_users_key_and_not_from_the_tables() {
    let mut app = app_bound("basis = \"b\"");
    let is_nominal = |app: &Headless| app.world().resource::<Basis>().nominal;
    press_key(&mut app, plurimus::term::KeyCode::Char('n'));
    assert!(!is_nominal(&app), "n is no longer the key");
    press_key(&mut app, plurimus::term::KeyCode::Char('b'));
    assert!(is_nominal(&app));
}

#[test]
fn the_key_row_and_the_help_name_the_users_keys() {
    let mut app = app_bound("save = \"ctrl-w\"\nhelp = \"ctrl-h\"\nreload = []");
    let frame = redrawn(&mut app);
    let row = frame.lines().last().unwrap();
    assert!(row.contains("ctrl-w save"), "{row}");
    assert!(!row.contains("reload"), "a command with no key has no hint");
    assert!(row.contains(": commands  ctrl-h help"), "{row}");

    press_ctrl(&mut app, plurimus::term::KeyCode::Char('h'));
    type_text(&mut app, "draft to the plan file");
    let rows = picker_rows(&mut app);
    let is_keyed = |key: &str| rows.iter().any(|(_, badge)| badge == key);
    assert!(is_keyed("ctrl-w") && !is_keyed("ctrl-s"), "{rows:?}");
}

#[test]
fn an_unbound_finder_is_left_out_of_the_key_row() {
    let mut app = app_bound("help = []\nfocus-next = []");
    let frame = redrawn(&mut app);
    let row = frame.lines().last().unwrap();
    assert!(row.contains(": commands"), "{row}");
    assert!(!row.contains("help") && !row.contains("pane"), "{row}");
}

#[test]
fn what_the_keys_table_gets_wrong_is_said_once_the_shell_is_up() {
    let app = app_bound("savee = \"x\"\nsave = \"q\"");
    let journal = app.world().resource::<Journal>();
    let said: Vec<(Level, &str)> = journal
        .entries()
        .map(|entry| (entry.level, entry.text.as_str()))
        .collect();
    assert_eq!(
        said,
        [
            (Level::WARN, "[tui.keys] savee: no command has that name"),
            (Level::INFO, "[tui.keys] q now runs save, not quit"),
        ]
    );
}
