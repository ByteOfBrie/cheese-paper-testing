use super::format::MemoizedMarkdownHighlighter;
use crate::components::Text;
use crate::ui::EditorContext;
use cow_utils::CowUtils;
use egui::{Response, TextBuffer};
use std::borrow::Cow;
use std::ops::Range;

#[derive(Debug, Default)]
pub struct TextBox {
    highlighter: MemoizedMarkdownHighlighter,
}

fn get_current_word(text: &str, position: usize) -> Range<usize> {
    let before = &text[..position];

    let before_pos = before
        .char_indices()
        .rev()
        .find_map(|(pos, chr)| {
            if chr.is_whitespace() {
                Some(pos + 1) // +1 because we're looking for the first non-whitespace
            } else {
                None
            }
        })
        .unwrap_or_default();

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

pub fn trim_word_for_spellcheck(word: &str) -> (Cow<str>, Range<usize>) {
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

impl Text {
    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) -> Response {
        let ignore_range = if ctx.typing_status.is_new_word {
            Some(&ctx.typing_status.current_word)
        } else {
            None
        };

        let mut layouter = |ui: &egui::Ui, tinymark: &dyn TextBuffer, wrap_width: f32| {
            let mut layout_job = self._rdata.obtain::<TextBox>().highlighter.highlight(
                ui.style(),
                tinymark.as_str(),
                &ctx.dictionary,
                &ignore_range,
            );
            layout_job.wrap.max_width = wrap_width;
            ui.fonts(|f| f.layout_job(layout_job))
        };

        let output = egui::TextEdit::multiline(&mut self.text)
            .desired_width(f32::INFINITY)
            .layouter(&mut layouter)
            .min_size(egui::Vec2 { x: 50.0, y: 100.0 })
            .lock_focus(true)
            .show(ui);

        if let Some(cursor_range) = output.cursor_range {
            let primary_cursor_pos = cursor_range.primary.index;
            let current_word_pos = get_current_word(&self.text, primary_cursor_pos);

            if current_word_pos.is_empty() || current_word_pos.end == self.text.len() {
                ctx.typing_status.is_new_word = true;
                ctx.typing_status.current_word = current_word_pos;
            } else if ctx.typing_status.is_new_word
                && current_word_pos.contains(&ctx.typing_status.current_word.start)
            {
                ctx.typing_status.current_word = current_word_pos
            } else if !current_word_pos.contains(&primary_cursor_pos) {
                // we're editing a word elsewhere
                ctx.typing_status.is_new_word = false;
            }
        }

        // if we've just created a new word (pressed enter or space), force highlighting
        // to happen a second time. We're one frame behind on inputs which wouldn't
        // normally matter except we save highlight input. This means that the word
        // will be spellchecked again, this time not being ignored.
        //
        // We could *possibly* do a little bit better about this by detecting when a new
        // word has been created while still highlighting, but this is visually good and
        // less complicated to implement
        if ui.input(|i| i.key_pressed(egui::Key::Space) || i.key_pressed(egui::Key::Enter)) {
            self._rdata.obtain::<TextBox>().highlighter.force_highlight = true;
        }

        if output.response.clicked_by(egui::PointerButton::Secondary) {
            if let Some(cursor_range) = output.cursor_range {
                let clicked_pos = cursor_range.primary.index;

                let word_boundaries = get_current_word(&self.text, clicked_pos);

                let raw_word = &self.text[word_boundaries];

                // Will need word_range when spellcheck corrections are implemented, but it's not needed now
                let (check_word, _word_range) = trim_word_for_spellcheck(raw_word);

                ctx.spellcheck_status.selected_word = check_word.to_string();

                if let Some(dictionary) = ctx.dictionary.as_ref() {
                    if dictionary.check(&ctx.spellcheck_status.selected_word) {
                        ctx.spellcheck_status.correct = true;
                    } else {
                        ctx.spellcheck_status.correct = false;
                        ctx.spellcheck_status.suggestions.clear();
                        dictionary.suggest(
                            &ctx.spellcheck_status.selected_word,
                            &mut ctx.spellcheck_status.suggestions,
                        );
                    }
                }
            }
        }

        output.response.context_menu(|ui| {
            if ctx.spellcheck_status.selected_word.is_empty() {
                ui.close();
            }

            if ctx.spellcheck_status.correct {
                ui.label(format!(
                    "spelled {:?} correctly",
                    ctx.spellcheck_status.selected_word
                ));
            } else {
                ui.label(format!(
                    "misspelled {:?}",
                    ctx.spellcheck_status.selected_word
                ));

                for suggestion in ctx.spellcheck_status.suggestions.iter() {
                    if ui.button(suggestion).clicked() {
                        // TODO: implement replacement
                        println!("clicked {suggestion}");
                    }
                }
            }
        });

        output.response
    }

    // pub fn panels(&mut self, ctx: &egui::Context) {
    //     egui::CentralPanel::default().show(ctx, |ui| {
    //         self.ui(ui);
    //     });
    // }
}
