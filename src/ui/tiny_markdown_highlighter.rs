use egui::{FontFamily, FontId};
use spellbook::Dictionary;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Style {
    pub strong: bool,
    pub italic: bool,
    pub misspelled: bool,
}

#[derive(Default)]
pub struct MemoizedMarkdownHighlighter {
    style: egui::Style,
    text: String,
    output: egui::text::LayoutJob,
}

impl MemoizedMarkdownHighlighter {
    pub fn highlight(
        &mut self,
        egui_style: &egui::Style,
        text: &str,
        dictionary: &Option<&mut Dictionary>,
    ) -> egui::text::LayoutJob {
        if (&self.style, self.text.as_str()) != (egui_style, text) {
            self.style = egui_style.clone();
            text.clone_into(&mut self.text);
            self.output = highlight_tinymark(egui_style, text, dictionary);
        }
        self.output.clone()
    }
}

pub fn highlight_tinymark(
    egui_style: &egui::Style,
    mut text: &str,
    dictionary: &Option<&mut Dictionary>,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let mut style = Style::default();

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
        } else if text.starts_with(' ') {
            skip = 1;
            if let Some(word_end) = text[skip..].find(&[' ', '\n'][..]) {
                let word_pre_strip = &text[skip..word_end + skip];
                let punctuation: &[_] = &['.', '\'', '"', ',', '-', '!', '*'];
                let trimmed_word = word_pre_strip.trim_matches(punctuation);
                if let Some(dict) = &dictionary {
                    if !dict.check(trimmed_word) {
                        job.append(&text[..skip], 0.0, format_from_style(egui_style, &style));

                        style.misspelled = true;
                        // try to compute the length of the word and apend early
                        // super fucking hacky but it might work
                        //
                        // if this does work, it'll prioritize highlighting over other text formatting
                        // and also will break the ending of formatting for stuff like
                        // `*correct inncorrect* other`
                        // "other" will still be italicized because the incorrect will consume that `*`
                        // thankfully it'll probably won't work at all and so there won't be any problems
                        //
                        // I probably need to do something where I separate out the words in the line and
                        // keep track of those positions or something?
                        let word_length = word_pre_strip.len();
                        job.append(
                            &text[skip..skip + word_length],
                            0.0,
                            format_from_style(egui_style, &style),
                        );
                        text = &text[skip + word_length..];
                        style.misspelled = false;
                        skip = 0;
                    }
                }
            }
        } else {
            skip = 0;
        }

        // read up to the next special character
        let line_end = text[skip..]
            .find('\n')
            .map_or_else(|| text.len(), |i| (skip + i + 1));

        let end = text[skip..]
            // .find("*")
            .find(&['*', ' '][..])
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

fn format_from_style(egui_style: &egui::Style, tinymark_style: &Style) -> egui::text::TextFormat {
    let color = if tinymark_style.strong {
        egui_style.visuals.strong_text_color()
    } else if tinymark_style.misspelled {
        egui_style.visuals.error_fg_color
    } else {
        egui_style.visuals.text_color()
    };

    egui::text::TextFormat {
        color,
        italics: tinymark_style.italic,
        font_id: FontId {
            // TODO: update this based on actual font size (or figure out why it doesn't update)
            size: 24.0,
            family: FontFamily::Proportional,
        },
        ..Default::default()
    }
}
