mod format;
mod spellcheck;

use crate::ui::prelude::*;
use crate::ui::project_editor::search::textbox_search;
use egui::TextBuffer;
use egui::text::LayoutJob;

pub type Store = RenderDataStore<usize, TextBox>;

#[derive(Debug, Default)]
pub struct TextBox {
    // highlighter: MemoizedMarkdownHighlighter,

    // Memoized Layout Job
    layout_job: LayoutJob,

    word_count: usize,

    // manually force the layout to be redone
    redo_layout: bool,

    // formatting information that the highlight job was for
    // used to know when highlight needs to be redone
    text_signature: (usize, usize),
    editor_signature: usize,
    style: egui::Style,
}

impl TextBox {
    fn refresh(&mut self, text: &Text, ctx: &mut EditorContext) {
        let signature = (text.struct_uid, text.version);

        if signature != self.text_signature {
            self.text_signature = signature;

            self.word_count = spellcheck::word_count(text.as_str());
            self.redo_layout = true;
        }

        if ctx.version != self.editor_signature {
            self.editor_signature = ctx.version;
            self.redo_layout = true;
        }

        if ctx.search.active
            && let Some(search_results) = ctx.search.search_results.as_mut()
            && let Some(sr) = search_results.get_mut(&text.struct_uid)
            && sr.text_version != text.version
        {
            *sr = textbox_search::search(text, &sr.page, &sr.box_name, &ctx.search.find_text);
            ctx.search.clear_focus();
            self.redo_layout = true;
        }
    }

    fn get_layout(
        &mut self,
        ui: &egui::Ui,
        text: &dyn TextBuffer,
        ctx: &mut EditorContext,
    ) -> LayoutJob {
        let text = Text::downcast(text);

        self.refresh(text, ctx);

        if ui.style().as_ref() != &self.style {
            self.style = ui.style().as_ref().clone();

            self.redo_layout = true;
        }

        let (mut search_result, mut search_result_focus) = (None, None);

        if ctx.search.active {
            search_result = ctx
                .search
                .search_results
                .as_ref()
                .and_then(|sr| sr.get(&text.struct_uid));

            search_result_focus = ctx.search.focus.as_ref().and_then(|(uid, word_find)| {
                if *uid == text.struct_uid {
                    Some(word_find)
                } else {
                    None
                }
            });
        }

        if self.redo_layout {
            self.redo_layout = false;
            self.layout_job = format::compute_layout_job(
                text.as_str(),
                ctx,
                search_result,
                search_result_focus,
                &self.style,
            )
        }

        self.layout_job.clone()
    }
}

impl Text {
    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) -> Response {
        let rdata = ctx.stores.text_box.get(&self.struct_uid);
        let text_box: &mut TextBox = &mut rdata.borrow_mut();

        let mut layouter = |ui: &egui::Ui, text: &dyn TextBuffer, wrap_width: f32| {
            let mut layout_job = text_box.get_layout(ui, text, ctx);
            layout_job.wrap.max_width = wrap_width;
            ui.fonts(|f| f.layout_job(layout_job))
        };

        let text_box_id = self.struct_uid;

        let output = egui::TextEdit::multiline(self)
            .desired_width(f32::INFINITY)
            .layouter(&mut layouter)
            .min_size(egui::Vec2 { x: 50.0, y: 100.0 })
            .lock_focus(true)
            .id_salt(text_box_id)
            .show(ui);

        // Select the cursor text and scroll to it if requried
        if ctx.search.active
            && ctx.search.goto_focus
            && let Some((uid, word_find)) = &ctx.search.focus
            && uid == &self.struct_uid
        {
            // Cursor ranges are specified in character offsets, we have a byte offset,
            // find the word
            let mut start_char_pos: Option<usize> = None;
            let mut end_char_pos: Option<usize> = None;
            for (char_count, (offset, _char)) in self.text.char_indices().enumerate() {
                if start_char_pos.is_none() && offset >= word_find.start {
                    start_char_pos = Some(char_count);
                }
                if end_char_pos.is_none() && offset >= word_find.end {
                    end_char_pos = Some(char_count);
                    break;
                }
            }

            // Special case: if we're looking for a string that matches the end of the file,
            // end_char_pos will be None, we correct it here (rather than trying to fix my
            // logic above because that's complicated)
            if start_char_pos.is_some() && end_char_pos.is_none() {
                end_char_pos = Some(self.text.chars().count());
            }

            if let Some(start_pos) = start_char_pos
                && let Some(end_pos) = end_char_pos
                && let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), output.response.id)
            {
                let ccursor = egui::text::CCursorRange::two(
                    egui::text::CCursor::new(start_pos),
                    egui::text::CCursor::new(end_pos),
                );

                // Set the positition of the cursor in the text
                state.cursor.set_char_range(Some(ccursor));
                state.store(ui.ctx(), output.response.id);
                ui.ctx()
                    .memory_mut(|mem| mem.request_focus(output.response.id));

                // Find the position of the cursor position in the rendered text output
                let cursor_pos_in_galley = output
                    .galley
                    .pos_from_cursor(egui::text::CCursor::new(start_pos));

                let text_edit_pos = output.response.rect;

                // Add the minimum of the text edit widget to the galley position to get the
                // absolute rectangle
                let cursor_absolute_pos =
                    cursor_pos_in_galley.translate(text_edit_pos.min.to_vec2());

                ui.scroll_to_rect(cursor_absolute_pos, Some(egui::Align::Center));
            }

            // We've gone to our focus (or made our best effort), we're done
            ctx.search.goto_focus = false;
        }

        // Keep track of where we're typing and if it's new, used in spellcheck logic later on
        if let Some(cursor_range) = output.cursor_range {
            let primary_cursor_pos = cursor_range.primary.index;
            let current_word_pos = spellcheck::get_current_word(&self.text, primary_cursor_pos);

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

        // Check for paste events that contain smart quotes, and remove them from the text
        ui.input_mut(|i| {
            for event in &i.events {
                if let egui::Event::Paste(contents) = event
                    && contents.contains(['“', '”', '‘', '’'])
                {
                    self.clean_up_quotes();

                    // We've changed the text, so we need to update the layout once again
                    text_box.redo_layout = true;
                }
            }
        });

        // Draw spellcheck menu for the current word
        if output.response.clicked_by(egui::PointerButton::Secondary)
            && let Some(cursor_range) = output.cursor_range
        {
            let clicked_pos = cursor_range.primary.index;

            let word_boundaries = spellcheck::get_current_word(&self.text, clicked_pos);

            let raw_word = &self.text[word_boundaries];

            // Will need word_range when spellcheck corrections are implemented, but it's not needed now
            let (check_word, _word_range) = spellcheck::trim_word_for_spellcheck(raw_word);

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

    pub fn word_count(&self, ctx: &mut EditorContext) -> usize {
        let rdata = ctx.stores.text_box.get(&self.struct_uid);
        let text_box: &mut TextBox = &mut rdata.borrow_mut();

        text_box.refresh(self, ctx);
        text_box.word_count
    }

    /// Remove *all* smart quotes from text that was just pasted into. This could probably be made
    /// more efficient (e.g., we technically don't need to do this in a separate pass from formatting),
    /// but this works.
    fn clean_up_quotes(&mut self) {
        static SMART_QUOTE_REMOVAL_REGEX: SavedRegex =
            SavedRegex::new(|| Regex::new(r#"[“”‘’]"#).unwrap());

        // Iterate through the string backwards so we don't invalidate our own indexes
        for (replacement, replace_range) in SMART_QUOTE_REMOVAL_REGEX
            .find_iter(&self.text)
            .collect::<Vec<_>>()
            .iter()
            .rev()
            .map(|quote_match| {
                let replacement = if quote_match.as_str() == "“" || quote_match.as_str() == "”"
                {
                    "\""
                } else {
                    "\'"
                };

                (replacement, quote_match.range())
            })
            .collect::<Vec<_>>()
        {
            // Replace the string text in place
            self.text.replace_range(replace_range, replacement);
        }
    }
}
