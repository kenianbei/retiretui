//! The payee text a bank writes reduced to the words everything matches
//! on.

use crate::plan::US_STATES;

/// A run of digits this long is a reference number, not part of a name.
const REFERENCE_DIGITS: usize = 6;
/// How long a word of letters and digits mixed is taken for a code.
const CODE_LENGTHS: std::ops::RangeInclusive<usize> = 5..=10;
/// How long a word of letters without a vowel is taken for a code.
const UNVOICED_CODE_LENGTH: usize = 6;
const VOWELS: &str = "AEIOUY";

/// Uppercased, cut at its first comma, a processor's `*` taken as a space,
/// without store numbers, card suffixes, reference numbers or codes after
/// its first word, or a trailing state, its whitespace collapsed.
// ponytail: no city list, so a payee written with its city and without
// are two payees; the pattern table matches either by substring.
#[must_use]
pub fn normalize(text: &str) -> String {
    let upper = text.to_ascii_uppercase();
    let head = upper
        .split_once(',')
        .map_or(upper.as_str(), |(head, _)| head);
    let mut words: Vec<&str> = head
        .split_whitespace()
        .flat_map(pieces)
        .filter(|word| !word.is_empty() && !is_noise(word))
        .enumerate()
        .filter(|&(at, word)| at == 0 || !is_code(word))
        .map(|(_, word)| word)
        .collect();
    if words.len() > 1
        && words.last().is_some_and(|w| {
            US_STATES
                .iter()
                .any(|(code, _)| code.eq_ignore_ascii_case(w))
        })
    {
        words.pop();
    }
    words.join(" ")
}

/// A word split at each `*`, save one led by a single letter, as `E*TRADE`
/// is a name.
fn pieces(word: &str) -> std::str::Split<'_, impl FnMut(char) -> bool> {
    let is_initial = word.find('*') == Some(1);
    word.split(move |c| c == '*' && !is_initial)
}

/// A reference a processor writes beside the name: letters and digits
/// mixed, or six letters without a vowel.
fn is_code(word: &str) -> bool {
    if !word.chars().all(|c| c.is_ascii_alphanumeric()) {
        return false;
    }
    let has_digit = word.chars().any(|c| c.is_ascii_digit());
    let has_letter = word.chars().any(|c| c.is_ascii_alphabetic());
    let is_mixed = has_digit && has_letter && CODE_LENGTHS.contains(&word.len());
    let is_unvoiced = !has_digit
        && word.len() == UNVOICED_CODE_LENGTH
        && !word.chars().any(|c| VOWELS.contains(c));
    is_mixed || is_unvoiced
}

/// A pattern read as payees are, where it holds a processor's `*`.
#[must_use]
pub(super) fn starred_as_spaces(pattern: &str) -> String {
    if !pattern.contains('*') {
        return pattern.to_owned();
    }
    let words: Vec<&str> = pattern
        .split_whitespace()
        .flat_map(pieces)
        .filter(|word| !word.is_empty())
        .collect();
    words.join(" ")
}

fn is_noise(word: &str) -> bool {
    let digits = word.chars().filter(char::is_ascii_digit).count();
    word.starts_with('#')
        || digits >= REFERENCE_DIGITS
        || (digits > 0
            && word
                .trim_start_matches('X')
                .chars()
                .all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn a_payee_loses_its_store_number_city_and_references() {
        assert_eq!(normalize("Safeway #1234, San Jose"), "SAFEWAY");
        assert_eq!(normalize("SAFEWAY #1234 SAN JOSE CA"), "SAFEWAY SAN JOSE");
        assert_eq!(
            normalize("AMAZON.COM*2K4L9  AMZN.COM/BILL WA"),
            "AMAZON.COM AMZN.COM/BILL"
        );
        assert_eq!(
            normalize("CHECKCARD 1018 SHELL OIL 57442831 XXXX4321"),
            "CHECKCARD SHELL OIL"
        );
        assert_eq!(normalize("  pg&e   autopay "), "PG&E AUTOPAY");
        assert_eq!(normalize("CA"), "CA");
        assert_eq!(
            normalize("GOOGLE *STORAGE pKQ7ZB g.co/helppay#"),
            "GOOGLE STORAGE G.CO/HELPPAY#"
        );
        assert_eq!(
            normalize("TST*BLUE DOOR CAFE AUSTIN TX"),
            "TST BLUE DOOR CAFE AUSTIN"
        );
        assert_eq!(normalize("HOP*04KQZRT"), "HOP");
        assert_eq!(normalize("MCDONALD'S F31407 DAYTON"), "MCDONALD'S DAYTON");
        assert_eq!(normalize("E*TRADE SECURITIES"), "E*TRADE SECURITIES");
        assert_eq!(normalize("7ELEVEN 7ELEVEN"), "7ELEVEN");
        assert_eq!(normalize("GOOGLE*SPANISH4KIDS"), "GOOGLE SPANISH4KIDS");
        assert_eq!(normalize("GOOGLE *FI XHTVKQ"), "GOOGLE FI");
    }
}
