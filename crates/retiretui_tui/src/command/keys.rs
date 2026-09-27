use bevy_input::keyboard::Key;
use plurimus::ui::KeyBinding;

/// A character key, in the spelling the terminal reports it under.
pub fn character(text: &str) -> KeyBinding {
    KeyBinding::new(Key::Character(text.into()))
}

/// How a keystroke is shown beside its command.
pub fn label(binding: &KeyBinding) -> String {
    let key = match &binding.key {
        Key::Character(character) => character.to_string(),
        Key::Space => "space".to_owned(),
        Key::Escape => "esc".to_owned(),
        Key::ArrowUp => "↑".to_owned(),
        Key::ArrowDown => "↓".to_owned(),
        Key::ArrowLeft => "←".to_owned(),
        Key::ArrowRight => "→".to_owned(),
        named => format!("{named:?}").to_lowercase(),
    };
    let modifiers = binding.modifiers;
    let held = [
        (modifiers.ctrl, "ctrl-"),
        (modifiers.alt, "alt-"),
        (modifiers.shift, "shift-"),
    ];
    let mut label: String = held
        .into_iter()
        .filter_map(|(is_held, prefix)| is_held.then_some(prefix))
        .collect();
    label.push_str(&key);
    label
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_name_the_key_and_modifier() {
        assert_eq!(label(&character("q")), "q");
        assert_eq!(label(&character("s").with_ctrl()), "ctrl-s");
        assert_eq!(label(&KeyBinding::new(Key::Tab).with_shift()), "shift-tab");
        assert_eq!(label(&KeyBinding::new(Key::Escape)), "esc");
        assert_eq!(label(&KeyBinding::new(Key::Enter)), "enter");
        assert_eq!(
            label(&KeyBinding::new(Key::ArrowDown).with_ctrl()),
            "ctrl-↓"
        );
        assert_eq!(label(&character("f").with_alt()), "alt-f");
    }
}
