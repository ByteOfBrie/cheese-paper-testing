use egui::{Response, TextBuffer, Widget};

pub struct BaseTextEditor<'a> {
    text: &'a mut String,

    highlighter: crate::tiny_markdown::MemoizedMarkdownHighlighter,
}

impl<'a> Widget for &mut BaseTextEditor<'a> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let BaseTextEditor { text, highlighter } = self;

        let mut layouter = |ui: &egui::Ui, tinymark: &dyn TextBuffer, wrap_width: f32| {
            let mut layout_job = highlighter.highlight(ui.style(), tinymark.as_str());
            layout_job.wrap.max_width = wrap_width;
            ui.fonts(|f| f.layout_job(layout_job))
        };

        ui.add(
            egui::TextEdit::multiline(*text)
                .desired_width(f32::INFINITY)
                .layouter(&mut layouter)
                .min_size(egui::Vec2 { x: 50.0, y: 100.0 }),
        )
    }
}

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
}
