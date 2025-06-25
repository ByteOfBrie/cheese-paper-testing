use egui::ScrollArea;

use crate::ui::default_text::DEFAULT_TEXT;

pub struct BaseTextEditor<'a> {
    text: &'a mut String,

    highlighter: crate::tiny_markdown::MemoizedMarkdownHighlighter,
}

/*impl Default for BaseTextEditor {
    fn default() -> Self {
        Self {
            text_owned: DEFAULT_TEXT.trim().to_owned(),
            text: &text_owned,
            highlighter: Default::default(),
        }
    }
}*/

impl<'a> BaseTextEditor<'a> {
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            highlighter: Default::default(),
        }
    }

    pub fn panels(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui(ui);
        });
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical()
            .id_salt("text")
            .show(ui, |ui| self.editor_ui(ui));
    }

    fn editor_ui(&mut self, ui: &mut egui::Ui) {
        let BaseTextEditor { text, highlighter } = self;

        let mut layouter = |ui: &egui::Ui, tinymark: &str, wrap_width: f32| {
            let mut layout_job = highlighter.highlight(ui.style(), tinymark);
            layout_job.wrap.max_width = wrap_width;
            ui.fonts(|f| f.layout_job(layout_job))
        };

        let response = ui.add(
            egui::TextEdit::multiline(*text)
                .desired_width(f32::INFINITY)
                .layouter(&mut layouter),
        );

        if response.changed() {
            println!("Changed lines: {text}")
        }
    }
}
