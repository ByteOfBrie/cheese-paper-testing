use crate::components::{Project, file_objects::write_with_temp_file};
use crate::ui::project_editor::ProjectEditor;
use directories::ProjectDirs;
use egui::{FontFamily, FontId, ScrollArea, TextStyle};
use rfd::FileDialog;
use spellbook::Dictionary;
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
    dictionary_location: PathBuf,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            font_size: 18.0,
            reopen_last: true,
            dictionary_location: PathBuf::from("/usr/share/hunspell/en_US"),
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

        if let Some(dictionary_location) = table
            .get("dictionary_location")
            .and_then(|location| location.as_str())
        {
            self.dictionary_location = PathBuf::from(dictionary_location);
        }

        modified
    }

    fn save(&self, table: &mut DocumentMut) {
        table.insert("font_size", value(self.font_size as f64));
        table.insert("reopen_last", value(self.reopen_last));
    }

    fn get_path(project_dirs: &ProjectDirs) -> PathBuf {
        project_dirs.config_dir().join("settings.toml")
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
                .filter_map(|val| val.as_str())
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

        if let Some(last_project_parent_folder_value) = table.get("last_project_parent_folder")
            && let Some(last_export_folder) = last_project_parent_folder_value.as_str()
        {
            self.last_project_parent_folder = PathBuf::from(last_export_folder)
        }

        if let Some(last_export_folder_value) = table.get("last_export_folder")
            && let Some(last_export_folder) = last_export_folder_value.as_str()
        {
            self.last_export_folder = PathBuf::from(last_export_folder)
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
                            .filter_map(|val| val.as_str())
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

    fn get_path(project_dirs: &ProjectDirs) -> PathBuf {
        project_dirs.data_dir().join("data.toml")
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
    new_project_dir: Option<PathBuf>,
    new_project_name: String,
    /// Hacky (?) variable to get around borrows (set in the state rather than close directly)
    closing_project: bool,
    next_project: Option<PathBuf>,
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

        let settings_toml = match read_to_string(Settings::get_path(&project_dirs)) {
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

        let data_toml = match read_to_string(Data::get_path(&project_dirs)) {
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
            new_project_dir: None,
            new_project_name: String::new(),
            closing_project: false,
            next_project: None,
        }
    }
}

impl EditorState {
    fn save(&mut self) -> std::io::Result<()> {
        if self.modified {
            self.data.save(&mut self.data_toml);
            write_with_temp_file(
                &Data::get_path(&self.project_dirs),
                self.data_toml.to_string().as_bytes(),
            )?;

            self.settings.save(&mut self.settings_toml);
            write_with_temp_file(
                &Settings::get_path(&self.project_dirs),
                self.settings_toml.to_string().as_bytes(),
            )?;
        }

        Ok(())
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

    /// Dictionary for spellchecking, if we managed to load it
    dictionary: Option<Dictionary>,
}

impl eframe::App for CheesePaperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.state.closing_project {
            self.project_editor = None;
            self.state.closing_project = false;
            if let Some(new_project_path) = self.state.next_project.take()
                && let Err(err) = self.load_project(new_project_path)
            {
                log::error!("Could not load project: {err}");
            }
        }

        match &mut self.project_editor {
            Some(project_editor) => {
                egui::TopBottomPanel::top("menu_bar_panel")
                    .show_separator_line(false)
                    .show(ctx, |ui| {
                        egui::MenuBar::new().ui(ui, |ui| {
                            ui.menu_button("File", |ui| {
                                if ui.button("Close Project").clicked() {
                                    self.state.closing_project = true;
                                }
                                ui.menu_button("Recent Projects", |ui| {
                                    for project in self.state.data.recent_projects.iter() {
                                        if ui.button(project.to_string_lossy()).clicked() {
                                            self.state.closing_project = true;
                                            self.state.next_project = Some(project.clone());
                                        }
                                    }
                                });
                                if ui.button("Quit").clicked() {
                                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                            });
                        });
                    });

                project_editor.panels(ctx);

                let current_time = Instant::now();
                if current_time.duration_since(self.last_save) > Duration::from_secs(5) {
                    project_editor.save();
                    self.last_save = current_time;
                }
            }
            None => match self.state.new_project_dir.is_none() {
                true => self.choose_project_ui(ctx),
                false => self.new_project_name_ui(ctx),
            },
        }
    }
}

impl Drop for CheesePaperApp {
    fn drop(&mut self) {
        if let Some(project_editor) = &mut self.project_editor {
            project_editor.save();
        }
        self.save();
    }
}

fn configure_text_styles(ctx: &egui::Context, font_size: f32) {
    use FontFamily::{Monospace, Proportional};

    let scalar = (font_size / 10.0).ceil();

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(font_size + 2.0 * scalar, Proportional),
        ),
        (TextStyle::Body, FontId::new(font_size, Proportional)),
        (TextStyle::Monospace, FontId::new(font_size, Monospace)),
        (
            TextStyle::Button,
            FontId::new(font_size - scalar, Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(font_size - 2.0 * scalar, Proportional),
        ),
    ]
    .into();

    ctx.set_style(style);
}

impl CheesePaperApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let state = EditorState::default();

        configure_text_styles(&cc.egui_ctx, state.settings.font_size);

        let mut dictionary = None;

        // Attempt to load dictionary:
        let mut aff_path = state.settings.dictionary_location.clone();
        aff_path.set_extension("aff");
        let mut dic_path = state.settings.dictionary_location.clone();
        dic_path.set_extension("dic");

        if aff_path.exists() && dic_path.exists() {
            match (
                std::fs::read_to_string(aff_path),
                std::fs::read_to_string(dic_path),
            ) {
                (Ok(aff), Ok(dic)) => match Dictionary::new(&aff, &dic) {
                    Ok(dict) => dictionary = Some(dict),
                    Err(err) => {
                        log::warn!("Encountered error while trying to load dictionary: {err}")
                    }
                },
                (Err(aff_err), _) => {
                    log::warn!(
                        "Error while trying to read aff in {:?}: {aff_err}",
                        state.settings.dictionary_location
                    )
                }
                (_, Err(dic_err)) => {
                    log::warn!(
                        "Error while trying to read dic in {:?}: {dic_err}",
                        state.settings.dictionary_location
                    )
                }
            }
        } else {
            log::info!(
                "Unable to load at least one dictionary file ({aff_path:?}, {dic_path:?}, set \
                `dictionary_location` in settings to a path that contains the dictionary files or \
                put the files in the proper location."
            );
        }

        // Load the actual app
        let mut app = Self {
            project_editor: None,
            state,
            last_save: Instant::now(),
            dictionary,
        };

        if app.state.settings.reopen_last
            && let Some(last_open_project) = app.state.data.recent_projects.first()
        {
            let last_open_project = last_open_project.clone();
            if let Err(err) = app.load_project(PathBuf::from(&last_open_project)) {
                log::error!(
                    "error while trying to open most recent project: {last_open_project:?}: {err}"
                );
            }
        }

        app
    }

    fn choose_project_ui(&mut self, ctx: &egui::Context) {
        if let Some((_message, time)) = &self.state.error_message
            && time.elapsed().as_secs() > 7
        {
            self.state.error_message = None;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ScrollArea::vertical()
                    .id_salt("recent projects")
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            let projects = self.state.data.recent_projects.clone();
                            for project in projects {
                                if ui.button(project.to_string_lossy().to_string()).clicked()
                                    && let Err(err) = self.load_project(project.clone())
                                {
                                    log::error!(
                                        "Error while attempting to load {project:?}: {err}"
                                    );
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

            ui.vertical_centered(|ui| {
                let checkbox_response = ui.checkbox(
                    &mut self.state.settings.reopen_last,
                    "Automatically reopen project",
                );
                if checkbox_response.clicked() {
                    self.state.modified = true;
                }
            });

            ui.horizontal_centered(|ui| {
                ui.columns(5, |cols| {
                    cols[0].vertical_centered_justified(|_ui| {});
                    cols[1].vertical_centered_justified(|ui| {
                        if ui.button("new project").clicked() {
                            self.state.new_project_dir = FileDialog::new()
                                .set_title("New Project Parent Folder")
                                .set_directory(&self.state.data.last_project_parent_folder)
                                .pick_folder();
                        }
                    });
                    cols[2].vertical_centered_justified(|_ui| {});
                    cols[3].vertical_centered_justified(|ui| {
                        if ui.button("load project").clicked() {
                            let project_dir = FileDialog::new()
                                .set_title("Load Folder")
                                .set_directory(&self.state.data.last_project_parent_folder)
                                .pick_folder();

                            if let Some(project_dir) = project_dir
                                && let Err(err) = self.load_project(project_dir.clone())
                            {
                                log::error!(
                                    "Error while attempting to load {project_dir:?}: {err}"
                                );
                            }
                        }
                    });
                    cols[4].vertical_centered_justified(|_ui| {});
                });
            });
        });
    }

    fn new_project_name_ui(&mut self, ctx: &egui::Context) {
        let owned_folder_dir = self.state.new_project_dir.as_mut().unwrap().clone();

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Modal::new(egui::Id::new("new project name")).show(ui.ctx(), |ui| {
                ui.heading("New Project");
                ui.label("Project Name:");
                ui.text_edit_singleline(&mut self.state.new_project_name);

                ui.separator();

                egui::Sides::new().show(
                    ui,
                    |_ui| {},
                    |ui| {
                        if ui.button("Ok").clicked() {
                            match Project::new(
                                owned_folder_dir.clone(),
                                self.state.new_project_name.clone(),
                            ) {
                                Ok(project) => {
                                    self.state.data.last_project_parent_folder =
                                        owned_folder_dir.clone();
                                    self.state
                                        .data
                                        .recent_projects
                                        .insert(0, project.get_path());
                                    self.state.modified = true;
                                    self.project_editor = Some(ProjectEditor::new(
                                        project,
                                        Vec::new(),
                                        self.dictionary.clone(),
                                    ));
                                }
                                Err(err) => {
                                    log::error!("Error while attempting to create project: {err}");
                                    let error_message = format!("unable to create project: {err}");
                                    self.state.error_message =
                                        Some((error_message, Instant::now()));
                                }
                            }
                            self.state.new_project_dir = None;
                        }

                        ui.add_space(10.0);

                        if ui.button("Cancel").clicked() {
                            self.state.new_project_dir = None;
                        }
                    },
                );
            });
        });
    }

    fn load_project(&mut self, project_path: PathBuf) -> Result<()> {
        match Project::load(project_path) {
            Ok(project) => {
                // open the project
                let project_path = project.get_path();

                // update recent projects
                if project_path.parent()
                    != Some(self.state.data.last_project_parent_folder.as_path())
                    && let Some(path) = project_path.parent()
                {
                    self.state.data.last_project_parent_folder = path.to_path_buf();
                    self.state.modified = true;
                }

                let project_path_position = self
                    .state
                    .data
                    .recent_projects
                    .iter()
                    .position(|id| id == &project_path);

                match project_path_position {
                    Some(position) => {
                        if position != 0 {
                            let project_pathbuf = self.state.data.recent_projects.remove(position);
                            self.state.data.recent_projects.insert(0, project_pathbuf);
                            self.state.modified = true;
                        }
                    }
                    None => {
                        self.state
                            .data
                            .recent_projects
                            .insert(0, project_path.clone());
                        self.state.modified = true;
                    }
                };

                // load tabs
                self.project_editor = match self
                    .state
                    .data
                    .last_open_file_ids
                    .get(&*project.base_metadata.id)
                {
                    Some(open_tabs) => Some(ProjectEditor::new(
                        project,
                        open_tabs.clone(),
                        self.dictionary.clone(),
                    )),
                    None => Some(ProjectEditor::new(
                        project,
                        Vec::new(),
                        self.dictionary.clone(),
                    )),
                };

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

    fn update_open_tabs(&mut self) {
        if let Some(project_editor) = &self.project_editor {
            let open_tabs = project_editor.get_open_tabs();

            if Some(&open_tabs)
                != self
                    .state
                    .data
                    .last_open_file_ids
                    .get(&*project_editor.project.base_metadata.id)
            {
                self.state.data.last_open_file_ids.insert(
                    project_editor.project.base_metadata.id.to_string(),
                    open_tabs,
                );

                self.state.modified = true;
            }
        }
    }

    fn save(&mut self) {
        self.update_open_tabs();

        if let Err(err) = self.state.save() {
            log::error!("Error while attempting to save editor state: {err}")
        }
    }
}
