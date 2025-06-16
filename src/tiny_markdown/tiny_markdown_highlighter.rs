use crate::tiny_markdown::tiny_markdown_parser;

#[derive(Default)]
pub struct MemoizedMarkdownHighlighter {
    style: egui::Style,
    text: String,
    output: egui::text::LayoutJob,
}

impl MemoizedMarkdownHighlighter {
    pub fn highlight(&mut self, egui_style: &egui::Style, text: &str) -> egui::text::LayoutJob {
        if (&self.style, self.text.as_str()) != (egui_style, text) {
            self.style = egui_style.clone();
            text.clone_into(&mut self.text);
            self.output = highlight_tinymark(egui_style, text);
        }
        self.output.clone()
    }
}

pub fn highlight_tinymark(egui_style: &egui::Style, mut text: &str) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let mut style = tiny_markdown_parser::Style::default();

    while !text.is_empty() {
        let mut skip: usize;

        if text.starts_with("**") {
            skip = 2;
            if style.strong {
                job.append(&text[..skip], 0.0, format_from_style(egui_style, &style));
                text = &text[skip..];
                skip = 0;
            }
            style.strong ^= true;
        } else if text.starts_with('*') {
            skip = 1;
            if style.italic {
                job.append(&text[..skip], 0.0, format_from_style(egui_style, &style));
                text = &text[skip..];
                skip = 0;
            }
            style.italic ^= true;
        } else {
            skip = 0;
        }

        // read up to the next special character
        let line_end = text[skip..]
            .find('\n')
            .map_or_else(|| text.len(), |i| (skip + i + 1));

        let end = text[skip..]
            .find("*")
            .map_or_else(|| text.len(), |i| (skip + i).max(1));

        if line_end < end {
            job.append(
                &text[..line_end],
                0.0,
                format_from_style(egui_style, &style),
            );

            text = &text[line_end..];
            style = Default::default();
        } else {
            job.append(&text[..end], 0.0, format_from_style(egui_style, &style));
            text = &text[end..];
        }
    }

    job
}

fn format_from_style(
    egui_style: &egui::Style,
    tinymark_style: &tiny_markdown_parser::Style,
) -> egui::text::TextFormat {
    use egui::{Align, Color32, Stroke, TextStyle};

    let color = if tinymark_style.strong {
        egui_style.visuals.strong_text_color()
    } else {
        egui_style.visuals.text_color()
    };

    // TextStyle::Body.

    egui::text::TextFormat {
        color,
        italics: tinymark_style.italic,
        ..Default::default()
    }
}
