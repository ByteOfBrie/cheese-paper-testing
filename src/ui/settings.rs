use crate::ui::prelude::*;

use std::path::PathBuf;

use directories::ProjectDirs;
use toml_edit::{DocumentMut, value};

#[derive(Debug)]
struct SettingsData {
    /// size of the text font
    font_size: f32,

    /// auto indentation at the start of lines ?
    indent_line_start: bool,

    /// re-open the last project when launching the app ?
    reopen_last: bool,

    /// Location of the Dictionary
    dictionary_location: PathBuf,
}

impl Default for SettingsData {
    fn default() -> Self {
        Self {
            font_size: 18.0,
            reopen_last: true,
            indent_line_start: false,
            dictionary_location: PathBuf::from("/usr/share/hunspell/en_US"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Settings(Rc<RefCell<SettingsData>>);

impl Settings {
    pub fn load(&mut self, table: &DocumentMut) -> bool {
        let mut modified = false;

        let mut data = self.0.borrow_mut();

        match table.get("font_size") {
            Some(font_size_item) => {
                if let Some(font_size) = font_size_item.as_float() {
                    data.font_size = font_size as f32;
                } else if let Some(font_size) = font_size_item.as_integer() {
                    data.font_size = font_size as f32;
                } else {
                    modified = true;
                }
            }
            None => modified = true,
        }

        match table.get("reopen_last").and_then(|val| val.as_bool()) {
            Some(reopen_last) => data.reopen_last = reopen_last,
            None => modified = true,
        }

        match table.get("indent_line_start").and_then(|val| val.as_bool()) {
            Some(indent_line_start) => data.indent_line_start = indent_line_start,
            None => modified = true,
        }

        if let Some(dictionary_location) = table
            .get("dictionary_location")
            .and_then(|location| location.as_str())
        {
            data.dictionary_location = PathBuf::from(dictionary_location);
        }

        modified
    }

    pub fn save(&self, table: &mut DocumentMut) {
        let data = self.0.borrow();
        table.insert("font_size", value(data.font_size as f64));
        table.insert("reopen_last", value(data.reopen_last));
        table.insert("indent_line_start", value(data.indent_line_start));
    }

    pub fn get_path(project_dirs: &ProjectDirs) -> PathBuf {
        project_dirs.config_dir().join("settings.toml")
    }

    pub fn font_size(&self) -> f32 {
        self.0.borrow().font_size
    }

    pub fn reopen_last(&self) -> bool {
        self.0.borrow().reopen_last
    }

    pub fn indent_line_start(&self) -> bool {
        self.0.borrow().indent_line_start
    }

    pub fn dictionary_location(&self) -> PathBuf {
        self.0.borrow().dictionary_location.clone()
    }
}
