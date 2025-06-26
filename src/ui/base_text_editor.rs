use egui::{Response, Widget};

pub struct BaseTextEditor<'a> {
    text: &'a mut String,

    highlighter: crate::tiny_markdown::MemoizedMarkdownHighlighter,
}

impl<'a> Widget for &mut BaseTextEditor<'a> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let BaseTextEditor { text, highlighter } = self;

        let mut layouter = |ui: &egui::Ui, tinymark: &str, wrap_width: f32| {
            let mut layout_job = highlighter.highlight(ui.style(), tinymark);
            layout_job.wrap.max_width = wrap_width;
            ui.fonts(|f| f.layout_job(layout_job))
        };

        ui.add(
            egui::TextEdit::multiline(*text)
                .desired_width(f32::INFINITY)
                .layouter(&mut layouter),
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
