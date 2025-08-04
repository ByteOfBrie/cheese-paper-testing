use cow_utils::CowUtils;
use egui::{FontFamily, FontId, Stroke};
use regex::Regex;
use spellbook::Dictionary;
use std::{collections::VecDeque, ops::Range};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Style {
    pub strong: bool,
    pub italic: bool,
    pub misspelled: bool,
}

#[derive(Default, Debug)]
pub struct MemoizedMarkdownHighlighter {
    style: egui::Style,
    text: String,
    output: egui::text::LayoutJob,
    pub force_highlight: bool,
}

impl MemoizedMarkdownHighlighter {
    pub fn highlight(
        &mut self,
        egui_style: &egui::Style,
        text: &str,
        dictionary: &Option<Dictionary>,
        ignore_spellcheck: &Option<&Range<usize>>,
    ) -> egui::text::LayoutJob {
        if self.force_highlight || (&self.style, self.text.as_str()) != (egui_style, text) {
            self.style = egui_style.clone();
            text.clone_into(&mut self.text);
            self.output = highlight_tinymark(egui_style, text, dictionary, ignore_spellcheck);
            self.force_highlight = false;
        }
        self.output.clone()
    }
}

fn find_misspelled_words(
    text: &str,
    dictionary: &Option<Dictionary>,
    ignore_spellcheck: &Option<&Range<usize>>,
) -> VecDeque<usize> {
    // Indexes of all of the misspelled words
    let mut misspelled_words = VecDeque::new();

    // we only spellcheck if we have a dictionary:
    if let Some(dict) = &dictionary {
        // words in this case means everything that isn't whitespace, we'll take care of
        // trimming
        let word_regex = Regex::new(r"([^\s]+)").unwrap();

        for word_match in word_regex.find_iter(text) {
            let raw_word = word_match.as_str();

            // We need to filter out anything attached to our words
            let punctuation: &[_] = &['.', '\'', '"', ',', '-', '!', '*', '_'];
            // Keep track of how much we trimmed in each step (since that shouldn't be
            // marked as misspelled). This could also be done by a regex, but that seems
            // more complicated
            // possible regex: ^(['".,\-!*_]*)(\w.*\w)?(['".,\-!*_]*)$
            let start_trimmed_word = raw_word.trim_start_matches(punctuation);
            let trimmed_word = start_trimmed_word.trim_end_matches(punctuation);

            // TODO: filter out links and stuff (and maybe numbers?)

            // Rare case, allow for mid-word formatting changes (without unnecessary allocation)
            let check_word = trimmed_word.cow_replace("*", "");

            // floating punctuation isn't misspelled
            if !check_word.is_empty() {
                if !dict.check(&check_word) {
                    // We have a misspelled word now, compute boundaries

                    let chars_trimmed_start = raw_word.len() - start_trimmed_word.len();
                    let chars_trimmed_end = start_trimmed_word.len() - trimmed_word.len();

                    let start_pos = word_match.start() + chars_trimmed_start;
                    let end_pos = word_match.end() - chars_trimmed_end;

                    assert!(start_pos < end_pos);

                    // Check for the word that's currently being typed and
                    // avoid adding it to the list of misspelled words. This delays
                    // the detection a little bit, but I don't have a super nice way
                    // of getting that to work
                    if let Some(ignore_range) = ignore_spellcheck {
                        if ignore_range.contains(&start_pos) {
                            continue;
                        }
                    }

                    misspelled_words.push_back(start_pos);
                    misspelled_words.push_back(end_pos);
                }
            }
        }
    }

    misspelled_words
}

pub fn highlight_tinymark(
    egui_style: &egui::Style,
    mut text: &str,
    dictionary: &Option<Dictionary>,
    ignore_spellcheck: &Option<&Range<usize>>,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let mut style = Style::default();

    let mut misspelled_words = find_misspelled_words(text, dictionary, ignore_spellcheck);
    let mut text_pos: usize = 0;
    let mut start_of_line = true;

    while !text.is_empty() {
        let mut skip: usize;

        // first check for misspelled words
        if Some(&text_pos) == misspelled_words.front() {
            misspelled_words.pop_front();
            // We've already computed word boundaries so there's nothing to do here
            // besides mark the style
            style.misspelled ^= true;
        }

        if text.starts_with("**") {
            skip = 2;
            if style.strong {
                job.append(&text[..skip], 0.0, format_from_style(egui_style, &style));
                text = &text[skip..];
                text_pos += skip;
                skip = 0;
            }
            style.strong ^= true;
        } else if text.starts_with('*') {
            skip = 1;
            if style.italic {
                job.append(&text[..skip], 0.0, format_from_style(egui_style, &style));
                text = &text[skip..];
                text_pos += skip;
                skip = 0;
            }
            style.italic ^= true;
        } else {
            skip = 0;
        }

        // Check again, in case the formatting moved the text position. We advance
        // the text position every loop (at least one char), so we can't just loop
        // again. It might be better to get rid of the advancement requirement and
        // then let this get handled by two different loops, but that seems harder
        // to implement
        if Some(&text_pos) == misspelled_words.front() {
            misspelled_words.pop_front();
            // We've already computed word boundaries so there's nothing to do here
            // besides mark the style
            style.misspelled ^= true;
        }

        // read up to the next special character
        let line_end = text[skip..]
            .find('\n')
            .map_or_else(|| text.len(), |i| (skip + i + 1));

        let next_token_pos = if let Some(next_token) = text[skip..].find("*") {
            (skip + next_token).max(1)
        } else {
            text.len()
        };

        let next_misspelled_relative = if let Some(next_misspelled) = misspelled_words.front() {
            (next_misspelled - text_pos).max(1)
        } else {
            text.len()
        };

        let end = next_token_pos.min(next_misspelled_relative);

        let text_to_format = std::cmp::min(line_end, end);

        let leading_space = if start_of_line {
            start_of_line = false;
            20.0
        } else {
            0.0
        };

        job.append(
            &text[..text_to_format],
            leading_space,
            format_from_style(egui_style, &style),
        );

        text = &text[text_to_format..];
        text_pos += text_to_format;

        if line_end < end {
            style = Default::default();
            start_of_line = true;
        }
    }

    job
}

fn format_from_style(egui_style: &egui::Style, tinymark_style: &Style) -> egui::text::TextFormat {
    let color = if tinymark_style.strong {
        egui_style.visuals.strong_text_color()
    } else {
        egui_style.visuals.text_color()
    };

    let underline = if tinymark_style.misspelled {
        Stroke {
            width: 2.0,
            color: egui_style.visuals.error_fg_color,
        }
    } else {
        Stroke::NONE
    };

    egui::text::TextFormat {
        color,
        underline,
        italics: tinymark_style.italic,
        font_id: FontId {
            // TODO: update this based on actual font size (or figure out why it doesn't update)
            size: 24.0,
            family: FontFamily::Proportional,
        },
        ..Default::default()
    }
}
