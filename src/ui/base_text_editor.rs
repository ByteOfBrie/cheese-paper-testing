use egui::{Response, TextBuffer, Widget};
use spellbook::Dictionary;

pub struct BaseTextEditor<'a> {
    text: &'a mut String,

    highlighter: crate::tiny_markdown::MemoizedMarkdownHighlighter,

    dictionary: &'a Option<&'a mut Dictionary>,

    // shitty hack for persistence
    cursor_pos: &'a mut usize,
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

        if let Some(cursor_range) = output.cursor_range {
            *self.cursor_pos = cursor_range.as_sorted_char_range().start;
        }

        // println!("{:?}", output.cursor_range);
        // println!("current_pos: {}", self.cursor_pos);

        // output.response.context_menu(|ui| {
        //     // println!("ui!");
        //     if ui.button("asdfjlkasdfjlk").clicked() {
        //         println!("clicked button!");
        //         println!("{:?}", );
        //     }
        // });

        // TODO: move logic here, hopefully improving speed
        if output.response.clicked_by(egui::PointerButton::Secondary) {
            println!("right clicking!");
        }

        output.response.context_menu(|ui| {
            let (before, after) = self.text.split_at(*self.cursor_pos);
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

            let word = &self.text[*self.cursor_pos - begin_offset..*self.cursor_pos + end_offset];

            if word.is_empty() {
                ui.close();
            }

            if let Some(dictionary) = self.dictionary {
                if dictionary.check(word) {
                    ui.label(format!("spelled {word:?} correctly"));
                } else {
                    let mut suggestions = Vec::new();
                    dictionary.suggest(word, &mut suggestions);
                    ui.label(format!("misspelled {word:?}"));

                    for suggestion in suggestions {
                        if ui.button(&suggestion).clicked() {
                            println!("clicked {suggestion}");
                        }
                    }
                }
            }

            // ui.button(format!("do nothing: {word}"));
            // This should be necessary, but sometimes a weird series of inputs results in the width
            // trying to get super narrow, this ensures that doesn't look as awful
            // ui.set_min_width(150.0);
        });

        output.response
    }
}

impl<'a> BaseTextEditor<'a> {
    pub fn new(
        text: &'a mut String,
        dictionary: &'a Option<&'a mut Dictionary>,
        cursor_pos: &'a mut usize,
    ) -> Self {
        Self {
            text,
            highlighter: Default::default(),
            dictionary,
            cursor_pos,
        }
    }

    pub fn panels(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui(ui);
        });
    }
}
