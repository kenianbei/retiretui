use bevy_input::keyboard::Key;
use plurimus::ui::KeyBinding;

/// A character key, in the spelling the terminal reports it under.
pub fn character(text: &str) -> KeyBinding {
    KeyBinding::new(Key::Character(text.into()))
}

const CTRL: &str = "ctrl-";
const ALT: &str = "alt-";
const SHIFT: &str = "shift-";

/// What a held modifier is spelled as ahead of its key, and what holds it.
const HELD: [(&str, fn(KeyBinding) -> KeyBinding); 3] = [
    (CTRL, KeyBinding::with_ctrl),
    (ALT, KeyBinding::with_alt),
    (SHIFT, KeyBinding::with_shift),
];

/// The keys spelled by a name rather than by the character they type.
const NAMED: [Key; 27] = [
    Key::Space,
    Key::Escape,
    Key::Enter,
    Key::Tab,
    Key::Backspace,
    Key::Delete,
    Key::Insert,
    Key::Home,
    Key::End,
    Key::PageUp,
    Key::PageDown,
    Key::ArrowUp,
    Key::ArrowDown,
    Key::ArrowLeft,
    Key::ArrowRight,
    Key::F1,
    Key::F2,
    Key::F3,
    Key::F4,
    Key::F5,
    Key::F6,
    Key::F7,
    Key::F8,
    Key::F9,
    Key::F10,
    Key::F11,
    Key::F12,
];

/// What a key shown as a glyph may be typed as instead.
const TYPED: [(&str, &str); 6] = [
    ("up", "↑"),
    ("down", "↓"),
    ("left", "←"),
    ("right", "→"),
    ("tab", "⇥"),
    ("enter", "⏎"),
];

fn name(key: &Key) -> String {
    match key {
        Key::Character(character) => character.to_string(),
        Key::Space => "space".to_owned(),
        Key::Escape => "esc".to_owned(),
        Key::Enter => "⏎".to_owned(),
        Key::Tab => "⇥".to_owned(),
        Key::ArrowUp => "↑".to_owned(),
        Key::ArrowDown => "↓".to_owned(),
        Key::ArrowLeft => "←".to_owned(),
        Key::ArrowRight => "→".to_owned(),
        named => format!("{named:?}").to_lowercase(),
    }
}

/// How a keystroke is shown beside its command.
pub fn label(binding: &KeyBinding) -> String {
    let modifiers = binding.modifiers;
    let held = [
        (modifiers.ctrl, CTRL),
        (modifiers.alt, ALT),
        (modifiers.shift, SHIFT),
    ];
    let mut label: String = held
        .into_iter()
        .filter_map(|(is_held, prefix)| is_held.then_some(prefix))
        .collect();
    label.push_str(&name(&binding.key));
    label
}

/// The keystroke `text` spells, as [`label`] shows it or as [`TYPED`]
/// lets it be typed.
pub fn parse(text: &str) -> Result<KeyBinding, String> {
    let mut spelled = text;
    let mut holds = Vec::new();
    while let Some((rest, hold)) = HELD
        .iter()
        .find_map(|(prefix, hold)| Some((spelled.strip_prefix(prefix)?, hold)))
    {
        spelled = rest;
        holds.push(hold);
    }
    let binding = KeyBinding::new(key(spelled).ok_or_else(|| format!("\"{text}\" is not a key"))?);
    let binding = holds
        .into_iter()
        .fold(binding, |binding, hold| hold(binding));
    if binding.modifiers.shift && matches!(binding.key, Key::Character(_)) {
        return Err(format!(
            "\"{text}\": a shifted character is written as itself, as G is"
        ));
    }
    Ok(binding)
}

/// The key `spelled` names, or the one character it is.
fn key(spelled: &str) -> Option<Key> {
    let glyph = TYPED.iter().find(|(typed, _)| *typed == spelled);
    let shown = glyph.map_or(spelled, |(_, glyph)| glyph);
    if let Some(named) = NAMED.into_iter().find(|key| name(key) == shown) {
        return Some(named);
    }
    let mut characters = spelled.chars();
    let is_one = characters.next().is_some() && characters.next().is_none();
    is_one.then(|| Key::Character(spelled.into()))
}

#[cfg(test)]
mod tests {
    use super::super::COMMANDS;
    use super::*;

    #[test]
    fn labels_name_the_key_and_modifier() {
        assert_eq!(label(&character("q")), "q");
        assert_eq!(label(&character("s").with_ctrl()), "ctrl-s");
        assert_eq!(label(&KeyBinding::new(Key::Tab).with_shift()), "shift-⇥");
        assert_eq!(label(&KeyBinding::new(Key::Escape)), "esc");
        assert_eq!(label(&KeyBinding::new(Key::Enter)), "⏎");
        assert_eq!(label(&KeyBinding::new(Key::Backspace)), "backspace");
        assert_eq!(
            label(&KeyBinding::new(Key::ArrowDown).with_ctrl()),
            "ctrl-↓"
        );
        assert_eq!(label(&character("f").with_alt()), "alt-f");
    }

    #[test]
    fn a_key_reads_back_from_the_label_it_is_shown_by() {
        let bound = COMMANDS.iter().flat_map(|spec| &spec.keys);
        let named = NAMED.into_iter().map(KeyBinding::new);
        let held = [
            character("s").with_ctrl().with_alt(),
            KeyBinding::new(Key::Tab).with_shift(),
            character("-").with_ctrl(),
            character("-"),
            character("G"),
        ];
        for binding in bound.cloned().chain(named).chain(held) {
            assert_eq!(parse(&label(&binding)), Ok(binding));
        }
    }

    #[test]
    fn a_key_shown_as_a_glyph_is_typed_by_its_name() {
        assert_eq!(parse("up"), Ok(KeyBinding::new(Key::ArrowUp)));
        assert_eq!(
            parse("ctrl-right"),
            Ok(KeyBinding::new(Key::ArrowRight).with_ctrl())
        );
        assert_eq!(
            parse("shift-tab"),
            Ok(KeyBinding::new(Key::Tab).with_shift())
        );
        assert_eq!(parse("enter"), Ok(KeyBinding::new(Key::Enter)));
        assert_eq!(parse("f5"), Ok(KeyBinding::new(Key::F5)));
        assert_eq!(parse("pagedown"), Ok(KeyBinding::new(Key::PageDown)));
    }

    #[test]
    fn what_spells_no_key_is_refused_and_a_shifted_character_is_told_its_spelling() {
        for text in ["", "ctrl-", "ctrl+s", "escape", "return", "ab", "f13"] {
            assert_eq!(
                parse(text),
                Err(format!("\"{text}\" is not a key")),
                "{text}"
            );
        }
        let shifted = parse("shift-g").unwrap_err();
        assert!(shifted.contains("written as itself"), "{shifted}");
        assert!(parse("ctrl-shift-g").is_err());
    }
}
