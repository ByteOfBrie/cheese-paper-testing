use crate::components::Project;
use crate::components::file_objects::from_file;
use crate::ui::project_editor::ProjectEditor;
use directories::ProjectDirs;
use egui::{FontFamily, FontId, ScrollArea, TextStyle};
use rfd::FileDialog;
use std::{
    collections::HashMap,
    fs::read_to_string,
    io::Result,
    path::PathBuf,
    time::{Duration, Instant},
};
use toml_edit::{DocumentMut, value};

#[derive(Debug)]
pub struct Settings {
    font_size: f32,
    reopen_last: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            font_size: 18.0,
            reopen_last: true,
        }
    }
}

impl Settings {
    fn load(&mut self, table: &DocumentMut) -> bool {
        let mut modified = false;

        match table.get("font_size") {
            Some(font_size_item) => {
                if let Some(font_size) = font_size_item.as_float() {
                    self.font_size = font_size as f32;
                } else if let Some(font_size) = font_size_item.as_integer() {
                    self.font_size = font_size as f32;
                } else {
                    modified = true;
                }
            }
            None => modified = true,
        }

        match table.get("reopen_last").and_then(|val| val.as_bool()) {
            Some(reopen_last) => self.reopen_last = reopen_last,
            None => modified = true,
        }

        modified
    }

    fn save(&self, table: &mut DocumentMut) {
        table.insert("font_size", value(self.font_size as f64));
        table.insert("reopen_last", value(self.reopen_last));
    }
}

#[derive(Debug)]
struct Data {
    recent_projects: Vec<PathBuf>,
    last_project_parent_folder: PathBuf,
    last_export_folder: PathBuf,
    last_open_file_ids: HashMap<String, Vec<String>>,
}

impl Default for Data {
    fn default() -> Self {
        Self {
            recent_projects: Vec::new(),
            last_project_parent_folder: directories::UserDirs::new()
                .unwrap()
                .home_dir()
                .to_path_buf(),
            last_export_folder: directories::UserDirs::new()
                .unwrap()
                .home_dir()
                .to_path_buf(),
            last_open_file_ids: HashMap::new(),
        }
    }
}

impl Data {
    fn load(&mut self, table: &DocumentMut) {
        if let Some(recent_projects_array) =
            table.get("recent_projects").and_then(|val| val.as_array())
        {
            let recent_projects_str: Vec<_> = recent_projects_array
                .iter()
                .map(|val| val.as_str())
                .flatten()
                .map(|val| val.to_string())
                .collect();

            let mut recent_projects = Vec::new();

            for project in recent_projects_str {
                let project_path = PathBuf::from(project);
                if project_path.exists() {
                    recent_projects.push(project_path);
                }
            }

            self.recent_projects = recent_projects;
        }

        if let Some(last_project_parent_folder_value) = table.get("last_project_parent_folder") {
            if let Some(last_export_folder) = last_project_parent_folder_value.as_str() {
                self.last_project_parent_folder = PathBuf::from(last_export_folder)
            }
        }

        if let Some(last_export_folder_value) = table.get("last_export_folder") {
            if let Some(last_export_folder) = last_export_folder_value.as_str() {
                self.last_export_folder = PathBuf::from(last_export_folder)
            }
        }

        if let Some(last_open_file_ids) = table
            .get("last_open_file_ids")
            .and_then(|val| val.as_table_like())
        {
            for (key, val) in last_open_file_ids.iter() {
                if let Some(file_id_list) = val.as_array() {
                    self.last_open_file_ids.insert(
                        key.to_string(),
                        file_id_list
                            .iter()
                            .map(|val| val.as_str())
                            .flatten()
                            .map(|val| val.to_string())
                            .collect(),
                    );
                }
            }
        }
    }

    fn save(&self, table: &mut DocumentMut) {
        let mut recent_projects = toml_edit::Array::new();
        for project in self.recent_projects.iter() {
            recent_projects.push(project.to_string_lossy().to_string());
        }
        table.insert("recent_projects", value(recent_projects));

        table.insert(
            "last_project_parent_folder",
            value(
                self.last_project_parent_folder
                    .to_string_lossy()
                    .to_string(),
            ),
        );

        table.insert(
            "last_export_folder",
            value(self.last_export_folder.to_string_lossy().to_string()),
        );

        let mut last_open_file_ids = toml_edit::InlineTable::new();
        for (project_id, open_file_ids) in self.last_open_file_ids.iter() {
            let mut open_file_ids_arr = toml_edit::Array::new();
            for file_id in open_file_ids.iter() {
                open_file_ids_arr.push(file_id);
            }
            last_open_file_ids.insert(project_id, value(open_file_ids_arr).into_value().unwrap());
        }
        table.insert("last_open_file_ids", value(last_open_file_ids));
    }
}

struct EditorState {
    settings: Settings,
    settings_toml: DocumentMut,
    data: Data,
    data_toml: DocumentMut,
    modified: bool,
    project_dirs: ProjectDirs,
    error_message: Option<(String, Instant)>,
}

impl std::fmt::Debug for EditorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditorState")
            .field("settings", &self.settings)
            .field("data", &self.data)
            .field("modified", &self.modified)
            .field("project_dirs", &self.project_dirs)
            .finish()
    }
}

impl Default for EditorState {
    fn default() -> Self {
        let project_dirs = ProjectDirs::from("", "", "cheese-paper")
            .expect("it should be possible to write to system dirs");

        let mut settings = Settings::default();

        let settings_toml = match read_to_string(project_dirs.config_dir().join("settings.toml")) {
            Ok(config) => config
                .parse::<DocumentMut>()
                .expect("invalid toml settings file"),
            Err(err) => match err.kind() {
                // It's perfectly normal for there not to be a file, but any other IO error is a problem
                std::io::ErrorKind::NotFound => DocumentMut::new(),
                _ => {
                    log::error!("Unknown error while reading editor settings: {err}");
                    panic!("Unknown error while reading editor settings: {err}");
                }
            },
        };

        let modified = settings.load(&settings_toml);

        let mut data = Data::default();

        let data_toml = match read_to_string(project_dirs.data_dir().join("data.toml")) {
            Ok(config) => config
                .parse::<DocumentMut>()
                .expect("invalid toml data file"),
            Err(err) => match err.kind() {
                std::io::ErrorKind::NotFound => DocumentMut::new(),
                _ => {
                    log::error!("Unknown error while reading editor settings: {err}");
                    panic!("Unknown error while reading editor settings: {err}");
                }
            },
        };

        data.load(&data_toml);

        Self {
            settings,
            settings_toml,
            data,
            data_toml,
            modified,
            project_dirs,
            error_message: None,
        }
    }
}

pub struct CheesePaperApp {
    pub project_editor: Option<ProjectEditor>,

    state: EditorState,

    /// Time for autosaves
    ///
    ///  Shockingly, it actually makes some amount of sense to keep the logic here (instead of in
    ///`ProjectEditor`), since we'll eventually want to save editor configs as well, and it's better
    /// to propagate the event downwards
    last_save: Instant,
}

impl eframe::App for CheesePaperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match &mut self.project_editor {
            Some(project_editor) => {
                project_editor.panels(ctx);

                let current_time = Instant::now();
                if current_time.duration_since(self.last_save) > Duration::from_secs(5) {
                    project_editor.save();
                    self.last_save = current_time;
                }
            }
            None => {
                self.choose_project_ui(ctx);
            }
        }
    }
}

impl Drop for CheesePaperApp {
    fn drop(&mut self) {
        if let Some(project_editor) = &mut self.project_editor {
            project_editor.save();
        }
    }
}

fn configure_text_styles(ctx: &egui::Context) {
    use FontFamily::{Monospace, Proportional};

    // TODO: when configs are read, scale all of these off of the configured font size
    let text_styles: std::collections::BTreeMap<TextStyle, FontId> = [
        (TextStyle::Heading, FontId::new(28.0, Proportional)),
        (TextStyle::Body, FontId::new(24.0, Proportional)),
        (TextStyle::Monospace, FontId::new(24.0, Monospace)),
        (TextStyle::Button, FontId::new(20.0, Proportional)),
        (TextStyle::Small, FontId::new(20.0, Proportional)),
    ]
    .into();

    ctx.all_styles_mut(move |style| style.text_styles = text_styles.clone());
}

impl CheesePaperApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_text_styles(&cc.egui_ctx);

        let state = EditorState::default();

        println!("{:#?}", state);

        let mut app = Self {
            project_editor: None,
            state: state,
            last_save: Instant::now(),
        };

        if app.state.settings.reopen_last {
            if let Some(last_open_project) = app.state.data.recent_projects.get(0) {
                let last_open_project = last_open_project.clone();
                if let Err(err) = app.load_project(PathBuf::from(&last_open_project)) {
                    log::error!(
                        "error while trying to open most recent project: {last_open_project:?}: {err}"
                    );
                }
            }
        }

        app
    }

    fn choose_project_ui(&mut self, ctx: &egui::Context) {
        if let Some((_message, time)) = &self.state.error_message {
            if time.elapsed().as_secs() > 7 {
                self.state.error_message = None;
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ScrollArea::vertical()
                    .id_salt("recent projects")
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            let projects = self.state.data.recent_projects.clone();
                            for project in projects {
                                if ui.button(&project.to_string_lossy().to_string()).clicked() {
                                    if let Err(err) = self.load_project(project.clone()) {
                                        log::error!(
                                            "Error while attempting to load {project:?}: {err}"
                                        );
                                    }
                                }
                            }
                        })
                    });
            });

            ui.add_space(50.0);

            let label_size = match &self.state.error_message {
                Some((message, _time)) => {
                    let response = ui.vertical_centered(|ui| {
                        ui.label(message);
                    });

                    response.response.rect.height()
                }
                None => 0.0,
            };

            ui.add_space(80.0 - label_size);

            ui.horizontal_centered(|ui| {
                ui.columns(5, |cols| {
                    cols[0].vertical_centered_justified(|_ui| {});
                    cols[1].vertical_centered_justified(|ui| {
                        if ui.button("new project").clicked() {
                            unimplemented!();
                        }
                    });
                    cols[2].vertical_centered_justified(|_ui| {});
                    cols[3].vertical_centered_justified(|ui| {
                        if ui.button("load project").clicked() {
                            let project_dir = FileDialog::new()
                                .set_directory(&self.state.data.last_project_parent_folder)
                                .pick_folder();

                            if let Some(project_dir) = project_dir {
                                if let Err(err) = self.load_project(project_dir.clone()) {
                                    log::error!(
                                        "Error while attempting to load {project_dir:?}: {err}"
                                    );
                                }
                            }
                        }
                    });
                    cols[4].vertical_centered_justified(|_ui| {});
                });
            });
        });
    }

    fn load_project(&mut self, project_path: PathBuf) -> Result<()> {
        match Project::load(project_path) {
            Ok(project) => {
                self.project_editor = Some(ProjectEditor::new(project));
                Ok(())
            }
            Err(err) => {
                log::error!("encountered error while trying to load project: {err}");
                let error_message = format!("unable to load project: {err}");
                self.state.error_message = Some((error_message, Instant::now()));
                Err(err)
            }
        }
    }
}
