//! Turns file names, content and user queries into index terms.
//!
//! Postgres' own text parser keeps `IMG_2024-holidayPhoto.jpg` in awkward pieces, so terms are
//! produced here and indexed with the `simple` configuration.

use std::collections::HashSet;

const MAX_QUERY_TERMS: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Lower,
    Upper,
    Digit,
}

fn class(c: char) -> Class {
    if c.is_numeric() {
        Class::Digit
    } else if c.is_uppercase() {
        Class::Upper
    } else {
        Class::Lower
    }
}

fn runs(input: &str) -> impl Iterator<Item = &str> {
    input
        .split(|c: char| !c.is_alphanumeric())
        .filter(|run| !run.is_empty())
}

fn splits_before(chars: &[char], index: usize) -> bool {
    match index
        .checked_sub(1)
        .map(|prev| (class(chars[prev]), class(chars[index])))
    {
        None => false,
        Some((Class::Lower, Class::Upper)) => true,
        Some((Class::Digit, Class::Lower | Class::Upper)) => true,
        Some((Class::Lower | Class::Upper, Class::Digit)) => true,
        Some((Class::Upper, Class::Upper)) => chars
            .get(index + 1)
            .is_some_and(|next| class(*next) == Class::Lower),
        Some(_) => false,
    }
}

fn parts(run: &str) -> Vec<String> {
    let chars = run.chars().collect::<Vec<_>>();
    let mut parts = Vec::<String>::new();
    for (index, c) in chars.iter().enumerate() {
        if index == 0 || splits_before(&chars, index) {
            parts.push(String::new());
        }
        if let Some(part) = parts.last_mut() {
            part.extend(c.to_lowercase());
        }
    }
    parts
}

fn push_unique(terms: &mut Vec<String>, seen: &mut HashSet<String>, term: String) {
    if !seen.contains(&term) {
        seen.insert(term.clone());
        terms.push(term);
    }
}

/// Space-separated index terms for text: every word, its camelCase and letter/digit parts, at
/// most `max_terms` distinct terms.
pub fn index_terms(text: &str, max_terms: usize) -> String {
    let mut terms = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for run in runs(text) {
        let pieces = parts(run);
        if pieces.len() > 1 {
            push_unique(&mut terms, &mut seen, run.to_lowercase());
        }
        for piece in pieces {
            push_unique(&mut terms, &mut seen, piece);
        }
        if terms.len() >= max_terms {
            terms.truncate(max_terms);
            break;
        }
    }
    terms.join(" ")
}

/// A `tsquery` requiring every typed word (as a prefix) to match, or `None` for a blank query.
///
/// Terms are alphanumeric only, so user input cannot inject `tsquery` operators.
pub fn tsquery(input: &str) -> Option<String> {
    let mut seen = HashSet::<String>::new();
    let terms = runs(input)
        .flat_map(parts)
        .filter(|term| seen.insert(term.clone()))
        .take(MAX_QUERY_TERMS)
        .map(|term| format!("{term}:*"))
        .collect::<Vec<_>>();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" & "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO_LIMIT: usize = usize::MAX;

    fn terms(text: &str) -> Vec<String> {
        index_terms(text, NO_LIMIT)
            .split(' ')
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn separators_split_words_and_extension() {
        assert_eq!(
            terms("my-file_v2.txt"),
            ["my", "file", "v2", "v", "2", "txt"]
        );
    }

    #[test]
    fn camel_case_keeps_whole_word_and_parts() {
        assert_eq!(terms("holidayPhoto"), ["holidayphoto", "holiday", "photo"]);
    }

    #[test]
    fn acronyms_split_before_the_next_word() {
        assert_eq!(terms("HTMLParser"), ["htmlparser", "html", "parser"]);
    }

    #[test]
    fn digits_split_from_letters() {
        assert_eq!(terms("IMG_2024"), ["img", "2024"]);
        assert_eq!(terms("photo2024"), ["photo2024", "photo", "2024"]);
    }

    #[test]
    fn unicode_letters_are_lowercased() {
        assert_eq!(terms("Отчёт-Годовой"), ["отчёт", "годовой"]);
    }

    #[test]
    fn repeated_terms_are_indexed_once() {
        assert_eq!(terms("a a A"), ["a"]);
    }

    #[test]
    fn term_count_is_capped() {
        assert_eq!(index_terms("one two three four", 2), "one two");
    }

    #[test]
    fn blank_text_has_no_terms() {
        assert_eq!(index_terms(" - _ ", NO_LIMIT), "");
    }

    #[test]
    fn query_requires_every_word_as_prefix() {
        assert_eq!(tsquery("holi pho").as_deref(), Some("holi:* & pho:*"));
    }

    #[test]
    fn query_splits_camel_case_like_the_index() {
        assert_eq!(
            tsquery("holidayPhoto").as_deref(),
            Some("holiday:* & photo:*")
        );
    }

    #[test]
    fn query_drops_tsquery_syntax() {
        assert_eq!(tsquery("a' | !b & (c)").as_deref(), Some("a:* & b:* & c:*"));
    }

    #[test]
    fn blank_query_is_none() {
        assert_eq!(tsquery("  -- "), None);
    }

    #[test]
    fn query_term_count_is_capped() {
        let letter = |n: u8| char::from(b'a' + n);
        let input = (0..40u8)
            .map(|n| format!("{}{}", letter(n / 26), letter(n % 26)))
            .collect::<Vec<_>>()
            .join(" ");
        let query = tsquery(&input).expect("query");
        assert!(query.split(" & ").count() <= MAX_QUERY_TERMS);
    }
}
