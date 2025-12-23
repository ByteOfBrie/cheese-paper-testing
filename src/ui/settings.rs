pub mod settings_page;
pub mod theme;

use crate::ui::prelude::*;

use crate::components::file_objects::utils::{create_dir_if_missing, write_with_temp_file};

use std::fs::read_dir;
use std::{fs::read_to_string, path::PathBuf};

use directories::ProjectDirs;
use toml_edit::{DocumentMut, value};

pub use theme::Theme;

#[derive(Debug)]
struct SettingsData {
    /// size of the text font
    font_size: f32,

    /// visual indentation at the start of lines (buggy)
    indent_line_start: bool,

    /// re-open the last project when launching the app
    reopen_last: bool,

    /// Location of the Dictionary
    dictionary_location: PathBuf,

    /// theming for visuals.
    theme: Theme,

    available_themes: Vec<(String, Theme)>,

    modified: bool,
}

impl Default for SettingsData {
    fn default() -> Self {
        Self {
            font_size: 18.0,
            reopen_last: true,
            indent_line_start: false,
            dictionary_location: PathBuf::from("/usr/share/hunspell/en_US"),
            theme: Theme::default(),
            available_themes: Vec::new(),
            modified: false,
        }
    }
}

impl SettingsData {
    pub fn load(&mut self, table: &DocumentMut) {
        match table.get("font_size") {
            Some(font_size_item) => {
                if let Some(font_size) = font_size_item.as_float() {
                    self.font_size = font_size as f32;
                } else if let Some(font_size) = font_size_item.as_integer() {
                    self.font_size = font_size as f32;
                } else {
                    self.modified = true;
                }
            }
            None => self.modified = true,
        }

        match table.get("reopen_last").and_then(|val| val.as_bool()) {
            Some(reopen_last) => self.reopen_last = reopen_last,
            None => self.modified = true,
        }

        match table.get("indent_line_start").and_then(|val| val.as_bool()) {
            Some(indent_line_start) => self.indent_line_start = indent_line_start,
            None => self.modified = true,
        }

        if let Some(dictionary_location) = table
            .get("dictionary_location")
            .and_then(|location| location.as_str())
        {
            self.dictionary_location = PathBuf::from(dictionary_location);
        }

        if let Some(theme_table) = table
            .get("theme")
            .and_then(|theme_item| theme_item.as_table_like())
        {
            self.theme = Theme::load(theme_table);
        }
    }

    pub fn save(&self, table: &mut DocumentMut) {
        table.insert("font_size", value(self.font_size as f64));
        table.insert("reopen_last", value(self.reopen_last));
        table.insert("indent_line_start", value(self.indent_line_start));
    }
}

#[derive(Debug, Clone, Default)]
pub struct Settings(Rc<RefCell<SettingsData>>);

impl Settings {
    pub fn load(&mut self, project_dirs: &ProjectDirs) -> Result<(), CheeseError> {
        let mut data = self.0.borrow_mut();

        let settings_toml = match read_to_string(Settings::config_file_path(&project_dirs)) {
            Ok(config) => config
                .parse::<DocumentMut>()
                .map_err(|err| cheese_error!("invalid toml settings file: {err}"))?,
            Err(err) => match err.kind() {
                // It's perfectly normal for there not to be a file, but any other IO error is a problem
                std::io::ErrorKind::NotFound => DocumentMut::new(),
                _ => {
                    return Err(cheese_error!(
                        "Unknown error while reading editor settings: {err}"
                    ));
                }
            },
        };

        data.load(&settings_toml);

        let mut available_themes = Vec::new();

        if let Ok(dir) = read_dir(&Self::themes_path(project_dirs)) {
            for entry in dir {
                let entry_path = entry?.path();
                if entry_path.is_file()
                    && entry_path.extension().is_some_and(|ext| ext == "toml")
                    && let Ok(theme_config) = read_to_string(&entry_path)
                {
                    let theme_config = match theme_config.parse::<DocumentMut>() {
                        Ok(doc) => doc,
                        Err(err) => {
                            log::error!(
                                "Error encountered while reading config at {} : {err}",
                                entry_path.to_str().unwrap_or_default()
                            );
                            continue;
                        }
                    };

                    let Some(name) = theme_config.get("name").and_then(|item| item.as_str()) else {
                        log::error!(
                            "Error while parsing theme at {}: theme must have a name",
                            entry_path.to_str().unwrap_or_default()
                        );
                        continue;
                    };

                    let Some(theme_table) = theme_config
                        .get("config")
                        .and_then(|theme_item| theme_item.as_table_like())
                    else {
                        log::error!(
                            "Error while parsing theme {name}: theme must contain a table of configs"
                        );
                        continue;
                    };

                    let theme = Theme::load(theme_table);

                    available_themes.push((name.to_string(), theme));
                }
            }
        }

        data.available_themes = available_themes;

        Ok(())
    }

    pub fn save(&self, project_dirs: &ProjectDirs) -> Result<(), CheeseError> {
        let mut settings_toml = DocumentMut::new();
        let mut data = self.0.borrow_mut();

        data.save(&mut settings_toml);
        write_with_temp_file(
            create_dir_if_missing(&Self::config_file_path(project_dirs))?,
            settings_toml.to_string().as_bytes(),
        )
        .map_err(|err| cheese_error!("Error while saving app settings\n{}", err))?;

        data.modified = false;

        Ok(())
    }

    fn config_file_path(project_dirs: &ProjectDirs) -> PathBuf {
        project_dirs.config_dir().join("settings.toml")
    }

    fn themes_path(project_dirs: &ProjectDirs) -> PathBuf {
        project_dirs.config_dir().join("themes")
    }

    pub fn font_size(&self) -> f32 {
        self.0.borrow().font_size
    }

    pub fn reopen_last(&self) -> bool {
        self.0.borrow().reopen_last
    }

    pub fn set_reopen_last(&mut self, reopen_last: bool) {
        self.0.borrow_mut().reopen_last = reopen_last;
    }

    pub fn indent_line_start(&self) -> bool {
        self.0.borrow().indent_line_start
    }

    pub fn dictionary_location(&self) -> PathBuf {
        self.0.borrow().dictionary_location.clone()
    }

    pub fn theme(&self) -> Theme {
        self.0.borrow().theme.clone()
    }

    pub fn modified(&self) -> bool {
        self.0.borrow().modified
    }

    pub fn randomize_theme(&mut self) {
        let new_theme = Theme::new_random();
        log::debug!("Generated randomized theme: {new_theme:#?}");
        self.0.borrow_mut().theme = new_theme;
    }
}
