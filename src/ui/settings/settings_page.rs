use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use egui::{Color32, RichText};

use crate::ui::prelude::*;

#[derive(Debug)]
pub struct SettingsPage {
    font_size_config: String,

    font_size_error: Option<String>,

    indent_line_start_config: bool,

    reopen_last_config: bool,

    dictionary_location_config: String,

    dictionary_location_error: Option<String>,

    next_update: Option<SystemTime>,
}

impl SettingsPage {
    const UPDATE_DELAY: Duration = Duration::from_millis(400);

    pub fn load(ctx: &mut EditorContext) -> Self {
        let data = ctx.settings.0.borrow();

        let font_size_config = format!("{}", data.font_size);

        let indent_line_start_config = data.indent_line_start;

        let reopen_last_config = data.reopen_last;

        let dictionary_location_config = match data.dictionary_location.to_str() {
            Some(s) => s.into(),
            None => String::new(),
        };

        Self {
            font_size_config,
            font_size_error: None,
            indent_line_start_config,
            reopen_last_config,
            dictionary_location_config,
            dictionary_location_error: None,
            next_update: None,
        }
    }

    // validate the entered data and propagate it to the settings
    fn validate_and_update(&mut self, ctx: &mut EditorContext) {
        println!("validating and updating");

        let mut settings_data = ctx.settings.0.borrow_mut();

        match self.font_size_config.parse::<f32>() {
            Ok(val) => {
                // todo! check range
                settings_data.font_size = val;
                self.font_size_error = None;
            }
            Err(_) => {
                self.font_size_error = Some("Font Size must be an integer".to_string());
            }
        }

        settings_data.indent_line_start = self.indent_line_start_config;
        settings_data.reopen_last = self.reopen_last_config;

        match self.dictionary_location_config.parse::<PathBuf>() {
            Ok(val) => {
                // todo! check range
                settings_data.dictionary_location = val;
                self.dictionary_location_error = None;
            }
            Err(_) => {
                self.dictionary_location_error =
                    Some("Dictionary Location must be a valid path".to_string());
            }
        }

        settings_data.modified = true;
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditorContext) -> Vec<egui::Id> {
        let mut ids = Vec::new();

        ui.label("Font Size");

        let response = ui.text_edit_singleline(&mut self.font_size_config);
        self.process_response(&response);
        ids.push(response.id);

        if let Some(err) = &self.font_size_error {
            ui.label(RichText::new(err).color(Color32::RED));
        }

        ui.label("Indent Line Start");

        let response = ui.checkbox(&mut self.indent_line_start_config, "");
        self.process_response(&response);
        ids.push(response.id);

        ui.label("Reopen Last Project on Launch");

        let response = ui.checkbox(&mut self.reopen_last_config, "");
        self.process_response(&response);
        ids.push(response.id);

        ui.label("Dictionary Location");

        let response = ui.text_edit_singleline(&mut self.dictionary_location_config);
        self.process_response(&response);
        ids.push(response.id);

        if let Some(err) = &self.dictionary_location_error {
            ui.label(RichText::new(err).color(Color32::RED));
        }

        if let Some(next_update) = self.next_update {
            let now = SystemTime::now();
            if now >= next_update {
                self.next_update = None;
                self.validate_and_update(ctx);
            } else {
                ui.ctx()
                    .request_repaint_after(next_update.duration_since(now).unwrap());
            }
        }

        ids
    }

    fn process_response(&mut self, response: &egui::Response) {
        if response.changed() {
            let next_update = SystemTime::now() + Self::UPDATE_DELAY;
            if let Some(prev_ne) = self.next_update {
                self.next_update = Some(std::cmp::max(next_update, prev_ne));
            } else {
                self.next_update = Some(next_update);
            }
        }
    }
}
