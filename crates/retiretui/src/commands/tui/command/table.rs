use std::sync::LazyLock;

use bevy_app::AppExit;
use bevy_ecs::prelude::{MessageWriter, Res, ResMut, World};
use bevy_ecs::system::SystemId;
use bevy_input::keyboard::Key;
use plurimus::ui::KeyBinding;

use super::keys::character;
use super::pickers;
use super::{CommandId, CommandSpec, Outcome, Scope};
use crate::commands::tui::compare::{self, Compared};
use crate::commands::tui::confirm::Confirm;
use crate::commands::tui::documents;
use crate::commands::tui::drawer;
use crate::commands::tui::edit::{self, Draft, DraftEditor};
use crate::commands::tui::focus;
use crate::commands::tui::issues;
use crate::commands::tui::ledger;
use crate::commands::tui::motion;
use crate::commands::tui::nav::{self, Group, LastShown, Page, Turn};
use crate::commands::tui::overview;
use crate::commands::tui::session::Basis;
use crate::commands::tui::setup;
use crate::commands::tui::sidebar;
use crate::commands::tui::theme;
use crate::commands::tui::watch::Watch;

/// Every keystroke bound to a command, in table order, which is the order
/// a key is matched in.
pub static BINDINGS: LazyLock<Vec<(KeyBinding, CommandId)>> = LazyLock::new(|| {
    super::all()
        .flat_map(|command| {
            command
                .spec()
                .keys
                .iter()
                .map(move |binding| (binding.clone(), command))
        })
        .collect()
});

/// Every command, in the order the pickers list them.
///
/// Not a `const`: a character key owns the string it is spelled with, and
/// which string type that is depends on a `bevy_input` feature, one of
/// whose spellings cannot be built in a constant.
pub static COMMANDS: LazyLock<Vec<CommandSpec>> = LazyLock::new(|| {
    let mut commands = vec![
        CommandSpec {
            name: "new",
            scope: Scope::Anywhere,
            doc: "start a new plan from a few answers",
            keys: vec![character("n").with_ctrl()],
            hint: None,
            register: Box::new(|world| world.register_system(setup::compose)),
        },
        CommandSpec {
            name: "open",
            scope: Scope::Shell,
            doc: "open a plan or scenario from the workspace",
            keys: vec![character("o").with_ctrl()],
            hint: None,
            register: Box::new(|world| world.register_system(documents::open_picker)),
        },
        CommandSpec {
            name: "save",
            scope: Scope::Anywhere,
            doc: "write the draft to the plan file",
            keys: vec![character("s").with_ctrl()],
            hint: Some("save"),
            register: Box::new(|world| world.register_system(edit::save)),
        },
        CommandSpec {
            name: "save-as",
            scope: Scope::Anywhere,
            doc: "write the draft under another name and open it",
            keys: Vec::new(),
            hint: None,
            register: Box::new(|world| world.register_system(documents::save_as)),
        },
        CommandSpec {
            name: "reload",
            scope: Scope::Anywhere,
            doc: "re-read the plan from disk",
            keys: vec![character("r")],
            hint: Some("reload"),
            register: Box::new(|world| world.register_system(reload)),
        },
        CommandSpec {
            name: "quit",
            scope: Scope::Shell,
            doc: "leave the dashboard",
            keys: vec![character("q"), character("c").with_ctrl()],
            hint: Some("quit"),
            register: Box::new(|world| world.register_system(quit)),
        },
        CommandSpec {
            name: "focus-next",
            scope: Scope::Shell,
            doc: "move to the page's next pane",
            keys: vec![KeyBinding::new(Key::Tab)],
            hint: None,
            register: Box::new(|world| world.register_system(focus::focus_next)),
        },
        CommandSpec {
            name: "focus-previous",
            scope: Scope::Shell,
            doc: "move to the page's previous pane",
            keys: vec![KeyBinding::new(Key::Tab).with_shift()],
            hint: None,
            register: Box::new(|world| world.register_system(focus::focus_previous)),
        },
        CommandSpec {
            name: "tab-next",
            scope: Scope::Anywhere,
            doc: "show the next tab",
            keys: vec![KeyBinding::new(Key::ArrowDown).with_ctrl()],
            hint: Some("tab"),
            register: Box::new(|world| world.register_system(tab_next)),
        },
        CommandSpec {
            name: "tab-previous",
            scope: Scope::Anywhere,
            doc: "show the previous tab",
            keys: vec![KeyBinding::new(Key::ArrowUp).with_ctrl()],
            hint: None,
            register: Box::new(|world| world.register_system(tab_previous)),
        },
        CommandSpec {
            name: "basis",
            scope: Scope::Anywhere,
            doc: "toggle today's and nominal dollars",
            keys: vec![character("n")],
            hint: Some("dollars"),
            register: Box::new(|world| world.register_system(toggle_basis)),
        },
        CommandSpec {
            name: "sort",
            scope: Scope::Lists,
            doc: "order the list by each column in turn, up then down",
            keys: vec![character("s")],
            hint: None,
            register: Box::new(|world| world.register_system(edit::sort)),
        },
        CommandSpec {
            name: "undo",
            scope: Scope::Anywhere,
            doc: "take back the last applied item",
            keys: vec![character("z").with_ctrl()],
            hint: None,
            register: Box::new(|world| world.register_system(edit::undo)),
        },
        CommandSpec {
            name: "redo",
            scope: Scope::Anywhere,
            doc: "put back the item undo took",
            keys: vec![character("y").with_ctrl()],
            hint: None,
            register: Box::new(|world| world.register_system(edit::redo)),
        },
        CommandSpec {
            name: "add",
            scope: Scope::Lists,
            doc: "add an item to the focused list",
            keys: vec![character("a")],
            hint: Some("add"),
            register: Box::new(|world| world.register_system(edit::add)),
        },
        CommandSpec {
            name: "delete",
            scope: Scope::Lists,
            doc: "delete the highlighted item",
            keys: vec![character("d")],
            hint: Some("delete"),
            register: Box::new(|world| world.register_system(edit::delete)),
        },
        CommandSpec {
            name: "palette",
            scope: Scope::Shell,
            doc: "run a command by name",
            keys: vec![character(":")],
            hint: None,
            register: Box::new(|world| world.register_system(pickers::open_commands)),
        },
        CommandSpec {
            name: "help",
            scope: Scope::Shell,
            doc: "find a command by what it does",
            keys: vec![character("?")],
            hint: None,
            register: Box::new(|world| world.register_system(pickers::open_help)),
        },
        CommandSpec {
            name: "theme",
            scope: Scope::Shell,
            doc: "try the themes on and keep one",
            keys: Vec::new(),
            hint: None,
            register: Box::new(|world| world.register_system(theme::picker::open)),
        },
        CommandSpec {
            name: "motion",
            scope: Scope::Shell,
            doc: "move fully, less, or not at all",
            keys: Vec::new(),
            hint: None,
            register: Box::new(|world| world.register_system(motion::cycle)),
        },
        CommandSpec {
            name: "messages",
            scope: Scope::Shell,
            doc: "show everything said this session",
            keys: vec![character("m")],
            hint: None,
            register: Box::new(|world| world.register_system(drawer::toggle)),
        },
        CommandSpec {
            name: "go-to",
            scope: Scope::Anywhere,
            doc: "show a page by name",
            keys: vec![character("g")],
            hint: None,
            register: Box::new(|world| world.register_system(pickers::open_pages)),
        },
    ];
    commands.extend(Page::ALL.into_iter().map(|page| CommandSpec {
        name: page.label(),
        scope: Scope::Anywhere,
        doc: page.doc(),
        keys: tab_keys(page),
        hint: None,
        register: Box::new(move |world| register_show(world, page)),
    }));
    commands.push(CommandSpec {
        name: "tools",
        scope: Scope::Anywhere,
        doc: "show the tools",
        keys: vec![character(&nav::tab_digit(Group::Tools.tab()).to_string())],
        hint: None,
        register: Box::new(|world| {
            world.register_system(|world: &mut World| sidebar::enter(world, Group::Tools))
        }),
    });
    commands.push(CommandSpec {
        name: "plan",
        scope: Scope::Anywhere,
        doc: "show the plan's editing domains",
        keys: vec![character(&nav::tab_digit(Group::Plan.tab()).to_string())],
        hint: None,
        register: Box::new(|world| {
            world.register_system(|world: &mut World| sidebar::enter(world, Group::Plan))
        }),
    });
    commands.push(CommandSpec {
        name: "domains",
        scope: Scope::Anywhere,
        doc: "hand the keyboard back to the tab's sidebar",
        keys: vec![
            KeyBinding::new(Key::Escape),
            KeyBinding::new(Key::ArrowLeft),
        ],
        hint: None,
        register: Box::new(|world| world.register_system(focus::enter_sidebar)),
    });
    commands.push(CommandSpec {
        name: "ledger-plan",
        scope: Scope::On(Page::Ledger),
        doc: "return the ledger from a market run to the plan",
        keys: vec![KeyBinding::new(Key::Escape)],
        hint: None,
        register: Box::new(|world| world.register_system(ledger::return_to_plan)),
    });
    commands.push(CommandSpec {
        name: "issues",
        scope: Scope::Anywhere,
        doc: "list every issue the draft has",
        keys: vec![character("i")],
        hint: None,
        register: Box::new(|world| world.register_system(issues::toggle)),
    });
    commands.push(CommandSpec {
        name: "compare-with",
        scope: Scope::On(Page::Compare),
        doc: "compare the document with a workspace file, or stop",
        keys: vec![character("c")],
        hint: Some("add"),
        register: Box::new(|world| world.register_system(documents::compare_with)),
    });
    commands.push(CommandSpec {
        name: "compare-open",
        scope: Scope::On(Page::Compare),
        doc: "open the highlighted plan, the document joining the compared",
        keys: vec![KeyBinding::new(Key::Enter)],
        hint: None,
        register: Box::new(|world| world.register_system(compare::open_highlighted)),
    });
    commands.push(CommandSpec {
        name: "compare-remove",
        scope: Scope::On(Page::Compare),
        doc: "stop comparing the highlighted plan",
        keys: vec![character("x")],
        hint: Some("remove"),
        register: Box::new(|world| world.register_system(compare::remove_highlighted)),
    });
    commands.push(CommandSpec {
        name: "compare-baseline",
        scope: Scope::On(Page::Compare),
        doc: "measure the compared plans against the highlighted one",
        keys: vec![character("b")],
        hint: Some("baseline"),
        register: Box::new(|world| world.register_system(compare::choose_baseline)),
    });
    commands.push(CommandSpec {
        name: "compare-difference",
        scope: Scope::On(Page::Compare),
        doc: "show each plan as its difference from the baseline, or as it is",
        keys: vec![character("d")],
        hint: Some("diff"),
        register: Box::new(|world| world.register_system(compare::toggle_difference)),
    });
    commands.push(CommandSpec {
        name: "compare-view",
        scope: Scope::On(Page::Compare),
        doc: "show the next view of the compared plans",
        keys: vec![character("v")],
        hint: Some("view"),
        register: Box::new(|world| world.register_system(compare::cycle_view)),
    });
    commands.push(CommandSpec {
        name: "compare-metric-next",
        scope: Scope::On(Page::Compare),
        doc: "chart the next metric, or the other set of four",
        keys: vec![KeyBinding::new(Key::ArrowRight)],
        hint: None,
        register: Box::new(|world| world.register_system(compare::next_metric)),
    });
    commands.push(CommandSpec {
        name: "compare-metric-previous",
        scope: Scope::On(Page::Compare),
        doc: "chart the previous metric, or the other set of four",
        keys: vec![KeyBinding::new(Key::ArrowLeft)],
        hint: None,
        register: Box::new(|world| world.register_system(compare::previous_metric)),
    });
    commands.push(CommandSpec {
        name: "overview-year-next",
        scope: Scope::On(Page::Overview),
        doc: "move the year a year on",
        keys: vec![KeyBinding::new(Key::ArrowRight)],
        hint: None,
        register: Box::new(|world| world.register_system(overview::next_year)),
    });
    commands.push(CommandSpec {
        name: "overview-year-previous",
        scope: Scope::On(Page::Overview),
        doc: "move the year a year back",
        keys: vec![KeyBinding::new(Key::ArrowLeft)],
        hint: None,
        register: Box::new(|world| world.register_system(overview::previous_year)),
    });
    commands.push(CommandSpec {
        name: "overview-chart",
        scope: Scope::On(Page::Overview),
        doc: "show the overview chart's next view",
        keys: vec![character("v")],
        hint: Some("chart"),
        register: Box::new(|world| world.register_system(overview::cycle_chart)),
    });
    commands.push(CommandSpec {
        name: "overview-edit",
        scope: Scope::On(Page::Overview),
        doc: "open the item behind the highlighted milestone or attention row",
        keys: vec![character("e")],
        hint: Some("edit"),
        register: Box::new(|world| world.register_system(overview::edit_row)),
    });
    commands.extend(super::tools::commands());
    commands.push(CommandSpec {
        name: "import-earnings",
        scope: Scope::On(Page::People),
        doc: "record a Social Security statement's earnings on the highlighted person",
        keys: vec![character("e")],
        hint: Some("import"),
        register: Box::new(|world| world.register_system(edit::import_earnings)),
    });
    commands
});

const UNSAVED: &str = "Quit and lose the unsaved changes?";

fn quit(
    draft: Res<Draft>,
    mut confirm: ResMut<Confirm>,
    mut exit: MessageWriter<AppExit>,
) -> Outcome {
    if draft.is_dirty() {
        confirm.ask(UNSAVED, "Quit", |commands| {
            commands.queue(|world: &mut World| {
                world.write_message(AppExit::Success);
            });
        });
    } else {
        exit.write(AppExit::Success);
    }
    Outcome::Done
}

fn toggle_basis(mut basis: ResMut<Basis>) -> Outcome {
    basis.nominal = !basis.nominal;
    Outcome::Done
}

/// The compared files are re-read with the document.
fn reload(
    mut watch: ResMut<Watch>,
    mut editor: DraftEditor,
    mut compared: ResMut<Compared>,
) -> Outcome {
    match editor.reload(&mut watch) {
        Ok(()) => {
            compared.reload(&editor.session.tables);
            Outcome::Done
        }
        Err(message) => Outcome::Refused(message),
    }
}

fn tab_next(turn: Turn, last: Res<LastShown>) -> Outcome {
    step_tab(turn, *last, 1)
}

fn tab_previous(turn: Turn, last: Res<LastShown>) -> Outcome {
    step_tab(turn, *last, -1)
}

fn step_tab(mut turn: Turn, last: LastShown, step: isize) -> Outcome {
    let tab = nav::neighbor_tab(turn.page().tab(), step);
    turn.to(nav::entering(tab, last));
    Outcome::Done
}

/// A page that is a tab of its own is reached by that tab's digit; a
/// grouped page is reached through its group's tab instead.
fn tab_keys(page: Page) -> Vec<KeyBinding> {
    if page.group().is_some() {
        Vec::new()
    } else {
        vec![character(&nav::tab_digit(page.tab()).to_string())]
    }
}

/// The system behind a page's own command.
pub fn register_show(world: &mut World, page: Page) -> SystemId<(), Outcome> {
    world.register_system(move |mut turn: Turn| {
        turn.to(page);
        Outcome::Done
    })
}
