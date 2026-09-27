//! Text measured in terminal cells, and cut to a width of them.

use plurimus::core::ratatui_core::text::Span;

/// The cell a clipped text ends in.
const ELLIPSIS: char = '…';

/// What `text` takes to draw, in terminal cells.
pub fn cells_of(text: &str) -> u16 {
    u16::try_from(Span::raw(text).width()).unwrap_or(u16::MAX)
}

/// `text` cut to `width` cells, the last spent saying it was cut.
pub fn clipped(text: String, width: u16) -> String {
    if cells_of(&text) <= width {
        return text;
    }
    let mut kept: String = fitting(text.chars(), width.saturating_sub(1)).collect();
    kept.push(ELLIPSIS);
    kept
}

/// `text` cut to `width` cells in its middle, the one there spent saying
/// it was cut, so texts told apart only at their ends still read apart.
pub fn clipped_middle(text: String, width: u16) -> String {
    if cells_of(&text) <= width {
        return text;
    }
    let room = width.saturating_sub(1);
    let tail: String = fitting(text.chars().rev(), room / 2).rev().collect();
    let head: String = fitting(text.chars(), room - cells_of(&tail)).collect();
    format!("{head}{ELLIPSIS}{tail}")
}

/// As many of `characters` as fit in `width` cells.
fn fitting(
    characters: impl DoubleEndedIterator<Item = char>,
    width: u16,
) -> impl DoubleEndedIterator<Item = char> {
    let mut taken = 0;
    let kept: Vec<char> = characters
        .take_while(|character| {
            taken += cells_of(character.encode_utf8(&mut [0; 4]));
            taken <= width
        })
        .collect();
    kept.into_iter()
}

/// `text` broken between words into lines of at most `width` cells, each
/// after the first led by `indent`; a word wider than a line stands alone
/// on one.
pub fn wrapped(text: &str, width: u16, indent: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut has_word = false;
    for word in text.split_whitespace() {
        if has_word && cells_of(&line) + 1 + cells_of(word) > width {
            lines.push(std::mem::replace(&mut line, indent.to_owned()));
            has_word = false;
        }
        if has_word {
            line.push(' ');
        }
        line.push_str(word);
        has_word = true;
    }
    lines.push(line);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_too_wide_goes_on_indented_lines() {
        assert_eq!(wrapped("age 60 → age 62", 20, "  "), ["age 60 → age 62"]);
        assert_eq!(
            wrapped("Income › salary › Ends: age 60", 16, "  "),
            ["Income › salary", "  › Ends: age 60"]
        );
        assert_eq!(wrapped("a unbreakable", 4, "  "), ["a", "  unbreakable"]);
    }

    #[test]
    fn a_text_cut_in_its_middle_keeps_both_ends() {
        let name = "retirement-plan-2031.toml";
        assert_eq!(clipped_middle(name.to_owned(), 25), name);
        assert_eq!(clipped_middle(name.to_owned(), 15), "retirem…31.toml");
        assert_eq!(clipped_middle(name.to_owned(), 16), "retireme…31.toml");
        assert_eq!(
            clipped_middle("王小明的帳戶".to_owned(), 7),
            "王小…戶",
            "two cells each, the head taking what the tail cannot"
        );
    }

    #[test]
    fn a_text_too_wide_says_it_was_cut() {
        assert_eq!(clipped("housing-rental".to_owned(), 14), "housing-rental");
        assert_eq!(clipped("housing-rental".to_owned(), 10), "housing-r…");
        assert_eq!(
            clipped("王小明的帳戶".to_owned(), 7),
            "王小明…",
            "two cells each"
        );
    }
}
