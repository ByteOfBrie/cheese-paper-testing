use egui::text::LayoutJob;
use regex::Regex;

use super::SavedRegex;
use crate::ui::{EditorContext, text_box::spellcheck::find_misspelled_words};

use egui::{FontFamily, FontId, Stroke};

#[derive(Debug, Clone, Copy)]
enum StyleOption {
    Strong,
    Italic,
    Misspelled,
    NewLine,
    None,
}

#[derive(Debug, Clone, Copy)]
struct StyleMarker {
    idx: usize,
    style: StyleOption,
    on: bool,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
struct Style {
    strong: bool,
    italic: bool,
    misspelled: bool,
    newline: bool,
}

impl Style {
    fn update(&mut self, marker: &StyleMarker) {
        match marker.style {
            StyleOption::Strong => self.strong = marker.on,
            StyleOption::Italic => self.italic = marker.on,
            StyleOption::Misspelled => self.misspelled = marker.on,
            StyleOption::NewLine => self.newline = marker.on,
            _ => (),
        }
    }
}

fn format_from_style(egui_style: &egui::Style, text_style: &Style) -> egui::text::TextFormat {
    let color = if text_style.strong {
        egui_style.visuals.strong_text_color()
    } else {
        egui_style.visuals.text_color()
    };

    let underline = if text_style.misspelled {
        Stroke {
            width: 2.0,
            color: egui_style.visuals.error_fg_color,
        }
    } else {
        Stroke::NONE
    };

    egui::text::TextFormat {
        color,
        underline,
        italics: text_style.italic,
        font_id: FontId {
            // TODO: update this based on actual font size (or figure out why it doesn't update)
            size: 24.0,
            family: FontFamily::Proportional,
        },
        ..Default::default()
    }
}

// format rules

fn format_rule_bold_italic(
    text: &str,
    _ctx: &EditorContext,
) -> (Vec<StyleMarker>, Vec<StyleMarker>) {
    let mut bold = Vec::new();
    let mut italic = Vec::new();

    static ASTERIX_GROUPS: SavedRegex = SavedRegex::new(|| Regex::new(r#"\*+"#).unwrap());

    let mut italic_start = None;
    let mut bold_start = None;

    for ag in ASTERIX_GROUPS.captures_iter(text) {
        let ag = ag.get(0).unwrap();

        match ag.len() {
            1 => {
                if let Some(start) = italic_start {
                    italic.push(StyleMarker {
                        idx: start,
                        style: StyleOption::Italic,
                        on: true,
                    });
                    italic.push(StyleMarker {
                        idx: ag.end(),
                        style: StyleOption::Italic,
                        on: false,
                    });
                    italic_start = None;
                } else {
                    italic_start = Some(ag.start());
                }
            }
            2 => {
                if let Some(start) = bold_start {
                    bold.push(StyleMarker {
                        idx: start,
                        style: StyleOption::Strong,
                        on: true,
                    });
                    bold.push(StyleMarker {
                        idx: ag.end(),
                        style: StyleOption::Strong,
                        on: false,
                    });
                    bold_start = None;
                } else {
                    bold_start = Some(ag.start());
                }
            }
            _ => (),
        }
    }

    (bold, italic)
}

fn format_rule_newlines(text: &str, _ctx: &EditorContext) -> Vec<StyleMarker> {
    let mut res = vec![StyleMarker {
        idx: 0,
        style: StyleOption::NewLine,
        on: true,
    }];

    for (idx, _) in text.match_indices('\n') {
        res.push(StyleMarker {
            idx: (idx + 1),
            style: StyleOption::NewLine,
            on: true,
        })
    }

    res
}

fn format_rule_spellcheck(text: &str, ctx: &EditorContext) -> Vec<StyleMarker> {
    find_misspelled_words(text, ctx)
        .into_iter()
        .flat_map(|(start, end)| {
            [
                StyleMarker {
                    idx: start,
                    style: StyleOption::Misspelled,
                    on: true,
                },
                StyleMarker {
                    idx: end,
                    style: StyleOption::Misspelled,
                    on: false,
                },
            ]
        })
        .collect()
}

// end format rules

pub fn compute_layout_job(text: &str, ctx: &EditorContext, egui_style: &egui::Style) -> LayoutJob {
    let (bold, italic) = format_rule_bold_italic(text, ctx);
    let newlines = format_rule_newlines(text, ctx);
    let spellcheck = format_rule_spellcheck(text, ctx);

    let mut styles = vec_merge([bold, italic, newlines, spellcheck]);
    styles.push(StyleMarker {
        idx: text.len(),
        style: StyleOption::None,
        on: false,
    });

    let mut job = LayoutJob::default();
    let mut text_style = Style::default();
    let mut start = 0;

    for marker in styles {
        let end = marker.idx;
        debug_assert!(end >= start);
        debug_assert!(end <= text.len());

        if end > start {
            let leading_space = if text_style.newline { 20.0 } else { 0.0 };

            job.append(
                &text[start..end],
                leading_space,
                format_from_style(egui_style, &text_style),
            );
            text_style.newline = false;

            start = end;
        }

        text_style.update(&marker);
    }

    debug_assert!(start == text.len());

    job
}

fn vec_merge(formats: [Vec<StyleMarker>; 4]) -> Vec<StyleMarker> {
    let mut res = Vec::new();
    let mut iters: Vec<_> = formats
        .into_iter()
        .map(|v| v.into_iter().peekable())
        .collect();

    loop {
        let mut next: Option<(StyleMarker, usize)> = None;
        for (idx, it) in iters.iter_mut().enumerate() {
            match (next, it.peek()) {
                (None, Some(v)) => {
                    next = Some((*v, idx));
                }
                (Some((v0, _)), Some(v1)) if v1.idx < v0.idx => {
                    next = Some((*v1, idx));
                }
                _ => (),
            }
        }
        if let Some((v, idx)) = next {
            iters[idx].next();
            res.push(v);
        } else {
            return res;
        }
    }
}
