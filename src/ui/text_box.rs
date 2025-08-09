mod format;
mod spellcheck;

use crate::components::Text;
use crate::ui::EditorContext;
use egui::text::LayoutJob;
use egui::{Response, TextBuffer};
use spellcheck::*;

type SavedRegex = std::sync::LazyLock<regex::Regex>;

#[derive(Debug, Default)]
pub struct TextBox {
    // highlighter: MemoizedMarkdownHighlighter,

    // Memoized Layout Job
    layout_job: LayoutJob,

    // manually force the layout to be redone
    redo_layout: bool,

    // formatting information that the highlight job was for
    // used to know when highlight needs to be redone
    text: String,
    style: egui::Style,
}

impl TextBox {
    fn get_layout(&mut self, ui: &egui::Ui, text: &str, ctx: &mut EditorContext) -> LayoutJob {
        if (text, ui.style().as_ref()) != (&self.text, &self.style) {
            self.text = String::from(text);
            self.style = ui.style().as_ref().clone();

            self.redo_layout = true;
        }

        if self.redo_layout {
            self.layout_job = format::compute_layout_job(text, ctx, &self.style);
            self.redo_layout = false;
        }

        self.layout_job.clone()
    }
}

impl Text {
    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) -> Response {
        let text_box = self._rdata.obtain::<TextBox>();

        let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, wrap_width: f32| {
            let mut layout_job = text_box.get_layout(ui, text.as_str(), ctx);
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
            text_box.redo_layout = true;
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
}
