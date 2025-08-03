use std::ops::Range;

use crate::ui::EditorContext;
use egui::{Response, TextBuffer, Widget};

pub struct BaseTextEditor<'a> {
    text: &'a mut String,

    ctx: &'a mut EditorContext,
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
        .unwrap_or_else(|| after.len());

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

impl<'a> Widget for &mut BaseTextEditor<'a> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let ignore_range = if self.ctx.typing_status.is_new_word {
            Some(&self.ctx.typing_status.current_word)
        } else {
            None
        };

        let mut layouter = |ui: &egui::Ui, tinymark: &dyn TextBuffer, wrap_width: f32| {
            let mut layout_job = self.ctx.highlighters.entry(ui.id()).or_default().highlight(
                ui.style(),
                tinymark.as_str(),
                &self.ctx.dictionary,
                &ignore_range,
            );
            layout_job.wrap.max_width = wrap_width;
            ui.fonts(|f| f.layout_job(layout_job))
        };

        let output = egui::TextEdit::multiline(self.text)
            .desired_width(f32::INFINITY)
            .layouter(&mut layouter)
            .min_size(egui::Vec2 { x: 50.0, y: 100.0 })
            .lock_focus(true)
            .show(ui);

        if let Some(cursor_range) = output.cursor_range {
            let primary_cursor_pos = cursor_range.primary.index;
            let current_word_pos = get_current_word(&self.text, primary_cursor_pos);

            if current_word_pos.is_empty() || current_word_pos.end == self.text.len() {
                self.ctx.typing_status.is_new_word = true;
                self.ctx.typing_status.current_word = current_word_pos;
            } else if self.ctx.typing_status.is_new_word
                && current_word_pos.contains(&self.ctx.typing_status.current_word.start)
            {
                self.ctx.typing_status.current_word = current_word_pos
            } else if !current_word_pos.contains(&primary_cursor_pos) {
                // we're editing a word elsewhere
                self.ctx.typing_status.is_new_word = false;
            }
        }

        if output.response.clicked_by(egui::PointerButton::Secondary) {
            if let Some(cursor_range) = output.cursor_range {
                let clicked_pos = cursor_range.as_sorted_char_range().start;

                let (before, after) = self.text.split_at(clicked_pos);
                let word_boundry_regex = regex::Regex::new(r#"[^\w'-]"#).unwrap();
                let end_offset = match word_boundry_regex.find(after) {
                    Some(mat) => mat.start(),
                    None => after.len(),
                };
                let begin_offset =
                    match word_boundry_regex.find(&before.chars().rev().collect::<String>()) {
                        Some(mat) => mat.start(),
                        None => before.len(),
                    };

                let word = &self.text[clicked_pos - begin_offset..clicked_pos + end_offset];

                self.ctx.spellcheck_status.selected_word = word.to_string();

                if let Some(dictionary) = self.ctx.dictionary.as_ref() {
                    if dictionary.check(&self.ctx.spellcheck_status.selected_word) {
                        self.ctx.spellcheck_status.correct = true;
                    } else {
                        self.ctx.spellcheck_status.correct = false;
                        self.ctx.spellcheck_status.suggestions.clear();
                        dictionary.suggest(
                            &self.ctx.spellcheck_status.selected_word,
                            &mut self.ctx.spellcheck_status.suggestions,
                        );
                    }
                }
            }
        }

        output.response.context_menu(|ui| {
            if self.ctx.spellcheck_status.selected_word.is_empty() {
                ui.close();
            }

            if self.ctx.spellcheck_status.correct {
                ui.label(format!(
                    "spelled {:?} correctly",
                    self.ctx.spellcheck_status.selected_word
                ));
            } else {
                ui.label(format!(
                    "misspelled {:?}",
                    self.ctx.spellcheck_status.selected_word
                ));

                for suggestion in self.ctx.spellcheck_status.suggestions.iter() {
                    if ui.button(suggestion).clicked() {
                        // TODO: implement replacement
                        println!("clicked {suggestion}");
                    }
                }
            }
        });

        output.response
    }
}

impl<'a> BaseTextEditor<'a> {
    pub fn new(text: &'a mut String, ctx: &'a mut EditorContext) -> Self {
        Self { text, ctx }
    }

    pub fn panels(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui(ui);
        });
    }
}
