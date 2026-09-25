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
    let mut kept = String::new();
    let mut taken = 0;
    for character in text.chars() {
        taken += cells_of(character.encode_utf8(&mut [0; 4]));
        if taken >= width {
            break;
        }
        kept.push(character);
    }
    kept.push(ELLIPSIS);
    kept
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
