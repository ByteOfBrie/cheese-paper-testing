use egui::{Response, TextBuffer, Widget};
use spellbook::Dictionary;

use crate::ui::project_editor::SpellCheckStatus;

pub struct BaseTextEditor<'a> {
    text: &'a mut String,

    highlighter: crate::tiny_markdown::MemoizedMarkdownHighlighter,

    dictionary: &'a Option<&'a mut Dictionary>,

    // shitty hack for persistence
    spellcheck_status: &'a mut SpellCheckStatus,
}

impl<'a> Widget for &mut BaseTextEditor<'a> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let mut layouter = |ui: &egui::Ui, tinymark: &dyn TextBuffer, wrap_width: f32| {
            let mut layout_job = self.highlighter.highlight(ui.style(), tinymark.as_str());
            layout_job.wrap.max_width = wrap_width;
            ui.fonts(|f| f.layout_job(layout_job))
        };

        let output = egui::TextEdit::multiline(self.text)
            .desired_width(f32::INFINITY)
            .layouter(&mut layouter)
            .min_size(egui::Vec2 { x: 50.0, y: 100.0 })
            .show(ui);

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

                self.spellcheck_status.selected_word = word.to_string();

                if let Some(dictionary) = self.dictionary {
                    if dictionary.check(&self.spellcheck_status.selected_word) {
                        self.spellcheck_status.correct = true;
                    } else {
                        self.spellcheck_status.correct = false;
                        self.spellcheck_status.suggestions.clear();
                        dictionary.suggest(
                            &self.spellcheck_status.selected_word,
                            &mut self.spellcheck_status.suggestions,
                        );
                    }
                }
            }
        }

        output.response.context_menu(|ui| {
            if self.spellcheck_status.selected_word.is_empty() {
                ui.close();
            }

            if self.spellcheck_status.correct {
                ui.label(format!(
                    "spelled {:?} correctly",
                    self.spellcheck_status.selected_word
                ));
            } else {
                ui.label(format!(
                    "misspelled {:?}",
                    self.spellcheck_status.selected_word
                ));

                for suggestion in self.spellcheck_status.suggestions.iter() {
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
    pub fn new(
        text: &'a mut String,
        dictionary: &'a Option<&'a mut Dictionary>,
        spellcheck_status: &'a mut SpellCheckStatus,
    ) -> Self {
        Self {
            text,
            highlighter: Default::default(),
            dictionary,
            spellcheck_status,
        }
    }

    pub fn panels(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui(ui);
        });
    }
}
