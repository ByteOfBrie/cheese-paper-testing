use cow_utils::CowUtils;
use regex::Regex;

use super::SavedRegex;
use crate::ui::EditorContext;

use std::borrow::Cow;
use std::ops::Range;

pub fn get_current_word(text: &str, mut position: usize) -> Range<usize> {
    // Use `ceil_char_boundary` once it's stable
    while !text.is_char_boundary(position) {
        position += 1;
    }

    let before = &text[..position];

    let mut before_pos_option = None;

    for (pos, chr) in before.char_indices().rev() {
        if chr.is_whitespace() {
            // The last character we found was the correct spot, before_pos_option is already set
            break;
        } else {
            before_pos_option = Some(pos);
        }
    }

    let before_pos = before_pos_option.unwrap_or(position);

    let after = &text[position..];

    let after_whitespace_offset = &text[position..]
        .char_indices()
        .find_map(|(pos, chr)| if chr.is_whitespace() { Some(pos) } else { None })
        .unwrap_or(after.len());

    let after_pos = position + after_whitespace_offset;

    before_pos..after_pos
}

#[test]
fn test_get_current_word() {
    assert_eq!(get_current_word("asdf jkl qwerty", 2), 0..4);
    assert_eq!(get_current_word("asdf jkl qwerty", 4), 0..4);
    assert_eq!(get_current_word("asdf jkl qwerty", 6), 5..8);
    assert_eq!(get_current_word("asdf  qwerty", 5), 5..5);
    assert_eq!(get_current_word("asdf  qwerty", 6), 6..12);
}

pub fn trim_word_for_spellcheck(word: &str) -> (Cow<'_, str>, Range<usize>) {
    // Keep track of how much we trimmed in each step (since that shouldn't be
    // marked as misspelled). This could also be done by a regex, but that seems
    // more complicated
    // possible regex: ^(['".,\-!*_]*)(\w.*\w)?(['".,\-!*_]*)$
    let start_trimmed_word = word.trim_start_matches(|chr: char| chr.is_ascii_punctuation());
    let trimmed_word = start_trimmed_word.trim_end_matches(|chr: char| chr.is_ascii_punctuation());

    // TODO: filter out links and stuff (and maybe numbers?)

    // Rare case, allow for mid-word formatting changes (without unnecessary allocation)
    let check_word = trimmed_word.cow_replace("*", "");

    let chars_trimmed_start = word.len() - start_trimmed_word.len();
    let chars_trimmed_end = start_trimmed_word.len() - trimmed_word.len();

    let end_pos = word.len() - chars_trimmed_end;

    (check_word, chars_trimmed_start..end_pos)
}

#[test]
fn test_trim_word_for_spellcheck() {
    assert_eq!(trim_word_for_spellcheck("word").0, "word");
    assert_eq!(trim_word_for_spellcheck("word").1, 0..4);

    assert_eq!(trim_word_for_spellcheck("word,").0, "word");
    assert_eq!(trim_word_for_spellcheck("word,").1, 0..4);

    assert_eq!(trim_word_for_spellcheck("*word*").0, "word");
    assert_eq!(trim_word_for_spellcheck("*word*").1, 1..5);

    assert_eq!(trim_word_for_spellcheck("*wo*rd").0, "word");
    assert_eq!(trim_word_for_spellcheck("*wo*rd").1, 1..6);
}

pub fn find_misspelled_words(text: &str, ctx: &EditorContext) -> Vec<(usize, usize)> {
    // Indexes of all of the misspelled words
    let mut misspelled_words = Vec::new();

    // we only spellcheck if we have a dictionary:
    if let Some(dict) = &ctx.dictionary {
        // words in this case means everything that isn't whitespace, we'll take care of
        // trimming
        static WORD_REGEX: SavedRegex = SavedRegex::new(|| Regex::new(r"([^\s]+)").unwrap());

        for word_match in WORD_REGEX.find_iter(text) {
            let (check_word, word_range) = trim_word_for_spellcheck(word_match.as_str());

            // floating punctuation isn't misspelled
            if !check_word.is_empty() && !dict.check(&check_word) {
                // We have a misspelled word now, compute boundaries

                let start_pos = word_match.start() + word_range.start;
                let end_pos = word_match.start() + word_range.end;

                assert!(start_pos < end_pos);

                // Check for the word that's currently being typed and
                // avoid adding it to the list of misspelled words. This delays
                // the detection a little bit, but I don't have a super nice way
                // of getting that to work
                if ctx.typing_status.is_new_word
                    && ctx.typing_status.current_word.contains(&start_pos)
                {
                    continue;
                }

                misspelled_words.push((start_pos, end_pos));
            }
        }
    }

    misspelled_words
}

pub fn word_count(text: &str) -> usize {
    static WORD_COUNT_REGEX: SavedRegex = SavedRegex::new(|| Regex::new(r"\s+").unwrap());
    WORD_COUNT_REGEX.split(text).count()
}
