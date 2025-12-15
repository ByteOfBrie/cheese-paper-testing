pub mod settings_page;

use crate::ui::prelude::*;

use std::path::PathBuf;

use directories::ProjectDirs;
use egui::Color32;
use rand::{Rng, rngs::ThreadRng};
use toml_edit::{DocumentMut, TableLike, value};

/// Most of the colors from https://docs.rs/egui/latest/egui/style/struct.Visuals.html
/// doesn't implement everything (because that requires more work), more can be added later
/// as requested/desired
#[derive(Debug, Clone)]
pub struct Theme {
    pub override_text_color: Option<Color32>,

    pub weak_text_color: Option<Color32>,

    pub hyperlink_color: Option<Color32>,

    /// Barely different from bg color, used for striped grids
    pub faint_bg_color: Option<Color32>,

    pub extreme_bg_color: Option<Color32>,

    /// Default: extreme_bg_color
    pub text_edit_bg_color: Option<Color32>,

    pub warn_fg_color: Option<Color32>,

    pub error_fg_color: Option<Color32>,

    pub window_fill_color: Option<Color32>,

    pub panel_fill_color: Option<Color32>,

    pub window_stroke_color: Option<Color32>,

    pub selection_bg_color: Option<Color32>,

    pub selection_fg_stroke_color: Option<Color32>,

    pub active_widget: Option<WidgetTheme>,
    pub inactive_widget: Option<WidgetTheme>,
    pub noninteractive_widget: Option<WidgetTheme>,
    pub hovered_widget: Option<WidgetTheme>,
    pub open_widget: Option<WidgetTheme>,
}

impl Theme {
    fn new_random() -> Self {
        let mut rng = rand::rng();

        Self {
            override_text_color: Some(random_color32(&mut rng)),
            weak_text_color: Some(random_color32(&mut rng)),
            hyperlink_color: Some(random_color32(&mut rng)),
            faint_bg_color: Some(random_color32(&mut rng)),
            extreme_bg_color: Some(random_color32(&mut rng)),
            text_edit_bg_color: Some(random_color32(&mut rng)),
            warn_fg_color: Some(random_color32(&mut rng)),
            error_fg_color: Some(random_color32(&mut rng)),
            window_fill_color: Some(random_color32(&mut rng)),
            panel_fill_color: Some(random_color32(&mut rng)),
            window_stroke_color: Some(random_color32(&mut rng)),
            selection_bg_color: Some(random_color32(&mut rng)),
            selection_fg_stroke_color: Some(random_color32(&mut rng)),
            active_widget: Some(WidgetTheme::new_random(&mut rng)),
            inactive_widget: Some(WidgetTheme::new_random(&mut rng)),
            noninteractive_widget: Some(WidgetTheme::new_random(&mut rng)),
            hovered_widget: Some(WidgetTheme::new_random(&mut rng)),
            open_widget: Some(WidgetTheme::new_random(&mut rng)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WidgetTheme {
    pub fg_stroke_color: Option<Color32>,
    pub bg_stroke_color: Option<Color32>,
    pub bg_fill: Option<Color32>,
    pub weak_bg_fill: Option<Color32>,
}

impl WidgetTheme {
    fn new_random(rng: &mut ThreadRng) -> Self {
        Self {
            fg_stroke_color: Some(random_color32(rng)),
            bg_stroke_color: Some(random_color32(rng)),
            bg_fill: Some(random_color32(rng)),
            weak_bg_fill: Some(random_color32(rng)),
        }
    }
}

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

    /// optional theming for visuals. will not be written back
    theme: Option<Theme>,

    modified: bool,
}

impl Default for SettingsData {
    fn default() -> Self {
        Self {
            font_size: 18.0,
            reopen_last: true,
            indent_line_start: false,
            dictionary_location: PathBuf::from("/usr/share/hunspell/en_US"),
            theme: None,
            modified: false,
        }
    }
}

fn read_color32(table: &dyn TableLike, field: &str) -> Option<Color32> {
    table
        .get(field)
        .and_then(|field| field.as_str())
        .map(Color32::from_hex)
        .and_then(|color_option| match color_option {
            Ok(color) => Some(color),
            Err(err) => {
                log::warn!("Could not parse color for {field}: {err:?}");
                None
            }
        })
}

fn random_color32(rng: &mut ThreadRng) -> Color32 {
    Color32::from_rgb(rng.random(), rng.random(), rng.random())
}

fn read_widget_theme(table: &dyn TableLike, field: &str) -> Option<WidgetTheme> {
    match table.get(field).and_then(|field| field.as_table_like()) {
        Some(widget_table) => {
            let fg_stroke_color = read_color32(widget_table, "fg_stroke_color");
            let bg_stroke_color = read_color32(widget_table, "bg_stroke_color");
            let bg_fill = read_color32(widget_table, "bg_fill");
            let weak_bg_fill = read_color32(widget_table, "weak_bg_fill");

            Some(WidgetTheme {
                fg_stroke_color,
                bg_stroke_color,
                bg_fill,
                weak_bg_fill,
            })
        }
        None => None,
    }
}

#[derive(Debug, Clone, Default)]
pub struct Settings(Rc<RefCell<SettingsData>>);

impl Settings {
    pub fn load(&mut self, table: &DocumentMut) {
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

        if let Some(theme_table) = table
            .get("theme")
            .and_then(|theme_item| theme_item.as_table_like())
        {
            let override_text_color = read_color32(theme_table, "override_text_color");
            let weak_text_color = read_color32(theme_table, "weak_text_color");
            let hyperlink_color = read_color32(theme_table, "hyperlink_color");
            let faint_bg_color = read_color32(theme_table, "faint_bg_color");
            let extreme_bg_color = read_color32(theme_table, "extreme_bg_color");
            let text_edit_bg_color = read_color32(theme_table, "text_edit_bg_color");
            let warn_fg_color = read_color32(theme_table, "warn_fg_color");
            let error_fg_color = read_color32(theme_table, "error_fg_color");
            let window_fill_color = read_color32(theme_table, "window_fill_color");
            let panel_fill_color = read_color32(theme_table, "panel_fill_color");

            let selection_bg_color = read_color32(theme_table, "selection_bg_color");
            let selection_fg_stroke_color = read_color32(theme_table, "selection_fg_stroke_color");
            let window_stroke_color = read_color32(theme_table, "window_stroke_color");

            let active_widget = read_widget_theme(theme_table, "active_widget");
            let inactive_widget = read_widget_theme(theme_table, "inactive_widget");
            let noninteractive_widget = read_widget_theme(theme_table, "noninteractive_widget");
            let hovered_widget = read_widget_theme(theme_table, "hovered_widget");
            let open_widget = read_widget_theme(theme_table, "open_widget");

            let theme = Theme {
                override_text_color,
                weak_text_color,
                hyperlink_color,
                faint_bg_color,
                extreme_bg_color,
                text_edit_bg_color,
                warn_fg_color,
                error_fg_color,
                window_fill_color,
                panel_fill_color,
                selection_bg_color,
                selection_fg_stroke_color,
                window_stroke_color,
                active_widget,
                inactive_widget,
                noninteractive_widget,
                hovered_widget,
                open_widget,
            };

            data.theme = Some(theme);
        }

        data.modified = modified
    }

    pub fn save(&self, table: &mut DocumentMut) {
        let mut data = self.0.borrow_mut();
        println!("saving settings: {:?}", data);
        table.insert("font_size", value(data.font_size as f64));
        table.insert("reopen_last", value(data.reopen_last));
        table.insert("indent_line_start", value(data.indent_line_start));

        data.modified = false;
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

    pub fn set_reopen_last(&mut self, reopen_last: bool) {
        self.0.borrow_mut().reopen_last = reopen_last;
    }

    pub fn indent_line_start(&self) -> bool {
        self.0.borrow().indent_line_start
    }

    pub fn dictionary_location(&self) -> PathBuf {
        self.0.borrow().dictionary_location.clone()
    }

    pub fn theme(&self) -> Option<Theme> {
        self.0.borrow().theme.clone()
    }

    pub fn modified(&self) -> bool {
        self.0.borrow().modified
    }

    pub fn randomize_theme(&mut self) {
        let new_theme = Theme::new_random();
        log::debug!("Generated randomized theme: {new_theme:#?}");
        self.0.borrow_mut().theme = Some(new_theme);
    }
}
