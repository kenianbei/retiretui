//! The payee text a bank writes reduced to the words everything matches
//! on.

use crate::plan::US_STATES;

/// A run of digits this long is a reference number, not part of a name.
const REFERENCE_DIGITS: usize = 6;

/// Uppercased, cut at its first comma, without store numbers, card
/// suffixes, reference numbers and a trailing state, its whitespace
/// collapsed.
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
        .map(|word| word.split_once('*').map_or(word, |(name, _)| name))
        .filter(|word| !word.is_empty() && !is_noise(word))
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
    }
}
