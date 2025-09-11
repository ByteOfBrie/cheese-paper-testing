use crate::cheese_error;
use crate::components::file_objects::{
    FileInfo, FileObject, FileObjectMetadata, FileObjectStore, Folder, from_file,
    write_with_temp_file,
};
use crate::components::text::Text;
use crate::util::CheeseError;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use toml_edit::DocumentMut;

use std::rc::Rc;

use crate::components::file_objects::utils::{process_name_for_filename, write_outline_property};

use crate::components::file_objects::base::{
    FileID, FileObjectCreation, load_base_metadata, metadata_extract_bool, metadata_extract_string,
    metadata_extract_u64,
};

/// An entire project. This is somewhat file_object like, but we don't implement everything,
/// so it's separate (for now)
#[derive(Debug)]
pub struct Project {
    pub metadata: ProjectMetadata,
    pub base_metadata: FileObjectMetadata,
    pub file: FileInfo,
    pub text_id: FileID,
    pub characters_id: FileID,
    pub worldbuilding_id: FileID,
    pub objects: FileObjectStore,
    toml_header: DocumentMut,
}

#[derive(Debug, Default)]
pub struct ProjectMetadata {
    pub summary: Text,
    pub notes: Text,
    pub genre: String,
    pub author: String,
    pub email: String,

    pub export: ProjectExportSettings,
}

#[derive(Debug)]
pub struct ProjectExportSettings {
    pub include_all_folder_titles: bool,
    /// how many levels deep to include folder titles, ignored if include_all_folder_titles is set
    pub include_folder_title_depth: u64,

    pub include_all_scene_titles: bool,
    /// how many levels deep to include scene titles, ignored if include_all_scene_titles is set
    pub include_scene_title_depth: u64,

    pub insert_break_at_end: bool,
}

impl Default for ProjectExportSettings {
    fn default() -> Self {
        Self {
            include_all_folder_titles: false,
            include_folder_title_depth: 1,
            include_all_scene_titles: false,
            include_scene_title_depth: 1,
            insert_break_at_end: true,
        }
    }
}

impl ProjectMetadata {
    pub fn for_each_textbox<'a>(&'a self, f: &mut dyn FnMut(&Text, &'static str)) {
        f(&self.summary, "summary");
        f(&self.notes, "notes");
    }

    #[allow(dead_code)] // included for the sake of completeness
    pub fn for_each_textbox_mut<'a>(&'a mut self, f: &mut dyn FnMut(&mut Text, &'static str)) {
        f(&mut self.summary, "summary");
        f(&mut self.notes, "notes");
    }
}

const PROJECT_INFO_NAME: &str = "project.toml";

fn load_top_level_folder(
    project_path: &Path,
    name: &str,
) -> Result<(Folder, FileObjectStore), CheeseError> {
    log::debug!("loading top level folder: {name}");

    let folder_path = &Path::join(project_path, name);
    if folder_path.exists() {
        let created_object = from_file(folder_path, None)
            .map_err(|err| cheese_error!("failed to load top level folder {name}\n{}", err))?;
        match created_object {
            FileObjectCreation::Folder(folder, contents) => Ok((folder, contents)),
            _ => Err(cheese_error!(
                "somehow loaded a non-folder as a top level folder",
            )),
        }
    } else {
        log::debug!("top level folder {name} does not exist, creating...");
        Ok((
            Folder::new_top_level(project_path.to_owned(), name).map_err(|err| {
                cheese_error!(
                    "An error occured while creating the top level folder\n{}",
                    err
                )
            })?,
            HashMap::new(),
        ))
    }
}

impl Project {
    /// Create a new project
    pub fn new(dirname: PathBuf, project_name: String) -> Result<Self, CheeseError> {
        // Not truncating here (for now)
        let file_safe_name = process_name_for_filename(&project_name);
        let project_path = dirname.join(&file_safe_name);

        if project_path.exists() {
            return Err(cheese_error!(
                "attempted to initialize {project_path:?}, which already exists"
            ));
        } else {
            std::fs::create_dir(&project_path)?;
        }

        let text = Folder::new_top_level(project_path.clone(), "text")?;
        let characters = Folder::new_top_level(project_path.clone(), "characters")?;
        let worldbuilding = Folder::new_top_level(project_path.clone(), "worldbuilding")?;

        let mut project = Self {
            base_metadata: FileObjectMetadata {
                name: project_name,
                ..Default::default()
            },
            metadata: ProjectMetadata::default(),
            text_id: text.get_base().metadata.id.clone(),
            characters_id: characters.get_base().metadata.id.clone(),
            worldbuilding_id: worldbuilding.get_base().metadata.id.clone(),
            file: FileInfo {
                dirname,
                basename: OsString::from(file_safe_name),
                modtime: None,
                modified: true, // Newly added files are modified (they don't exist on disk)
            },
            toml_header: DocumentMut::new(),
            objects: HashMap::new(),
        };

        project.add_object(Box::new(RefCell::new(text)));
        project.add_object(Box::new(RefCell::new(characters)));
        project.add_object(Box::new(RefCell::new(worldbuilding)));

        project.save()?;

        Ok(project)
    }

    /// Load an existing project from disk
    pub fn load(path: PathBuf) -> Result<Self, CheeseError> {
        if !path.exists() {
            return Err(cheese_error!(
                "attempted to load {path:?}, was not a directory"
            ));
        }

        let mut file_info = FileInfo {
            dirname: match path.parent() {
                Some(dirname) => dirname,
                None => {
                    return Err(cheese_error!("no directory component in {path:?}"));
                }
            }
            .to_path_buf(),
            basename: match path.file_name() {
                Some(basename) => basename,
                None => {
                    return Err(cheese_error!("no filename component in {path:?}"));
                }
            }
            .to_owned(),
            modtime: None,
            modified: false,
        };

        let mut base_metadata = FileObjectMetadata::default();
        let metadata = ProjectMetadata::default();

        // Load project metadata
        let project_info_path = Path::join(&path, PROJECT_INFO_NAME);

        let toml_header = if project_info_path.exists() {
            log::debug!("Found `project_info.toml`, loading project");

            let project_info_data =
                std::fs::read_to_string(project_info_path).expect("could not read file");

            project_info_data
                .parse::<DocumentMut>()
                .expect("invalid file metadata header")
        } else {
            // If the `project.toml` doesn't exist, check for a `text/` folder so we don't accidentally
            // load and hijack another folder
            if !Path::join(&path, "text").is_dir() {
                log::error!(
                    "Attempted to load a folder without `project_info.toml` or a `text/` folder. \
                     Consider creating a new project instead. If this was intended, please create \
                     one of the expected artifacts in that folder."
                );
                return Err(cheese_error!(
                    "attempted to load {path:?}, did not \
                         contain {PROJECT_INFO_NAME} or text folder"
                ));
            }
            log::debug!("Found `text/` but no project info file, creating it and continuing");
            DocumentMut::new()
        };

        // Load or create folders
        let (text, mut descendents) = load_top_level_folder(&path, "text")?;

        let (characters, characters_descendents) = load_top_level_folder(&path, "characters")?;

        let (worldbuilding, worldbuilding_descendents) =
            load_top_level_folder(&path, "worldbuilding")?;

        log::debug!("Finished loading all project file objects, continuing");

        // merge all of the descendents into a single hashmap that owns all of them
        descendents.extend(characters_descendents);
        descendents.extend(worldbuilding_descendents);

        load_base_metadata(&toml_header, &mut base_metadata, &mut file_info)?;

        let mut project = Self {
            metadata,
            base_metadata,
            file: file_info,
            text_id: text.get_base().metadata.id.clone(),
            characters_id: characters.get_base().metadata.id.clone(),
            worldbuilding_id: worldbuilding.get_base().metadata.id.clone(),
            toml_header,
            objects: descendents,
        };

        let metadata_modified = project.load_metadata()?;
        if metadata_modified {
            project.file.modified = true
        }

        project.add_object(Box::new(RefCell::new(text)));
        project.add_object(Box::new(RefCell::new(characters)));
        project.add_object(Box::new(RefCell::new(worldbuilding)));

        project.save()?;

        Ok(project)
    }

    pub fn add_object(&mut self, new_object: Box<RefCell<dyn FileObject>>) {
        let id = new_object.borrow().id().clone();
        self.objects.insert(id, new_object);
    }

    pub fn save(&mut self) -> Result<(), CheeseError> {
        // First, try saving the children

        let text_result = self
            .objects
            .get(&self.text_id)
            .unwrap()
            .borrow_mut()
            .save(&self.objects);
        let characters_result = self
            .objects
            .get(&self.characters_id)
            .unwrap()
            .borrow_mut()
            .save(&self.objects);
        let worldbuilding_result = self
            .objects
            .get(&self.worldbuilding_id)
            .unwrap()
            .borrow_mut()
            .save(&self.objects);

        // Now save the project itself
        // unlike other file objects, this one doesn't rename automatically. This might be something
        // I want to add later, but it's currently intentional

        if self.file.modified {
            self.write_metadata();

            let final_str = self.toml_header.to_string();

            write_with_temp_file(&self.get_project_info_file(), final_str.as_bytes())?;

            let new_modtime = std::fs::metadata(self.get_project_info_file())
                .expect("attempted to load file that does not exist")
                .modified()
                .expect("Modtime not available");

            // Update modtime based on what we just wrote
            self.file.modtime = Some(new_modtime);
        }

        text_result?;
        characters_result?;
        worldbuilding_result?;

        Ok(())
    }

    fn write_metadata(&mut self) {
        self.toml_header["version"] = toml_edit::value(self.base_metadata.version as i64);
        self.toml_header["name"] = toml_edit::value(&self.base_metadata.name);
        self.toml_header["id"] = toml_edit::value(&*self.base_metadata.id);

        self.toml_header["summary"] = toml_edit::value(&*self.metadata.summary);
        self.toml_header["notes"] = toml_edit::value(&*self.metadata.notes);
        self.toml_header["genre"] = toml_edit::value(&self.metadata.genre);
        self.toml_header["author"] = toml_edit::value(&self.metadata.author);
        self.toml_header["email"] = toml_edit::value(&self.metadata.email);

        self.toml_header["export.include_all_folder_titles"] =
            toml_edit::value(self.metadata.export.include_all_folder_titles);
        self.toml_header["export.include_folder_title_depth"] = toml_edit::value(
            u64_to_i64_drop_msb(self.metadata.export.include_folder_title_depth),
        );
        self.toml_header["export.include_all_scene_files"] =
            toml_edit::value(self.metadata.export.include_all_scene_titles);
        self.toml_header["export.include_scene_title_depth"] = toml_edit::value(
            u64_to_i64_drop_msb(self.metadata.export.include_scene_title_depth),
        );
        self.toml_header["export.insert_break_at_end"] =
            toml_edit::value(self.metadata.export.insert_break_at_end);
    }

    pub fn get_path(&self) -> PathBuf {
        Path::join(&self.file.dirname, &self.file.basename)
    }

    pub fn get_project_info_file(&self) -> PathBuf {
        let mut path = self.get_path();
        path.push(PROJECT_INFO_NAME);

        path
    }

    fn load_metadata(&mut self) -> Result<bool, CheeseError> {
        let mut modified = false;

        match metadata_extract_string(&self.toml_header, "summary")? {
            Some(summary) => self.metadata.summary = summary.into(),
            None => modified = true,
        }

        match metadata_extract_string(&self.toml_header, "notes")? {
            Some(notes) => self.metadata.notes = notes.into(),
            None => modified = true,
        }

        match metadata_extract_string(&self.toml_header, "genre")? {
            Some(genre) => self.metadata.genre = genre,
            None => modified = true,
        }

        match metadata_extract_string(&self.toml_header, "author")? {
            Some(author) => self.metadata.author = author,
            None => modified = true,
        }

        match metadata_extract_string(&self.toml_header, "email")? {
            Some(email) => self.metadata.email = email,
            None => modified = true,
        }

        match metadata_extract_bool(&self.toml_header, "export.include_all_folder_titles")? {
            Some(val) => self.metadata.export.include_all_folder_titles = val,
            None => modified = true,
        }

        match metadata_extract_u64(
            &self.toml_header,
            "export.include_folder_title_depth",
            false,
        )? {
            Some(val) => self.metadata.export.include_folder_title_depth = val,
            None => modified = true,
        }

        match metadata_extract_bool(&self.toml_header, "export.include_all_scene_files")? {
            Some(val) => self.metadata.export.include_all_scene_titles = val,
            None => modified = true,
        }

        match metadata_extract_u64(&self.toml_header, "export.include_scene_title_depth", false)? {
            Some(val) => self.metadata.export.include_scene_title_depth = val,
            None => modified = true,
        }

        match metadata_extract_bool(&self.toml_header, "export.insert_break_at_end")? {
            Some(val) => self.metadata.export.insert_break_at_end = val,
            None => modified = true,
        }

        Ok(modified)
    }

    /// Determine if the file should be loaded
    fn should_load(&mut self, file_to_read: &Path) -> Result<bool, CheeseError> {
        let current_modtime = std::fs::metadata(file_to_read)
            .expect("attempted to load file that does not exist")
            .modified()
            .expect("Modtime not available");

        if let Some(old_modtime) = self.file.modtime
            && old_modtime == current_modtime
        {
            // We've already loaded the latest revision, nothing to do
            return Ok(false);
        }

        Ok(true)
    }

    pub fn reload_file(&mut self) -> Result<(), CheeseError> {
        let file_to_read = self.get_project_info_file();

        if !self.should_load(&file_to_read)? {
            return Ok(());
        }

        let project_info_data = std::fs::read_to_string(file_to_read)?;

        let new_toml_header = project_info_data
            .parse::<DocumentMut>()
            .expect("invalid file metadata header");

        self.toml_header = new_toml_header;

        load_base_metadata(&self.toml_header, &mut self.base_metadata, &mut self.file)?;
        self.load_metadata()?;

        Ok(())
    }

    /// Given a path, find the file ID. Right now, this is a pretty dumb algorithm that
    /// just visits every file object, gets its path, and compares it. This means it's
    /// O(n) path allocations, but it should be reliable.
    pub fn find_object_by_path(&self, object_path: &Path) -> Option<Rc<String>> {
        // If we have the metadata path, we're trying to find the object with the
        // parent of it, so we compute that path instead
        let compare_path = if object_path.ends_with("metadata.toml") {
            object_path.parent().expect("path should have a parent")
        } else {
            object_path
        };

        for (id, file_object) in self.objects.iter() {
            if file_object.borrow().get_path() == compare_path {
                return Some(id.clone());
            }
        }

        None
    }

    /// Export an outline to a string (which can be written to a file)
    pub fn export_outline(&self) -> String {
        let mut export_string = String::new();

        // Property at the top
        export_string.push_str("# ");
        export_string.push_str(&self.base_metadata.name);
        export_string.push_str("\n\n");

        write_outline_property("Story Summary", &self.metadata.summary, &mut export_string);

        let text = self.objects.get(&self.text_id).unwrap().borrow();

        if !text.get_base().children.is_empty() {
            export_string.push_str("# Scenes\n\n");

            for child_id in text.get_base().children.iter() {
                self.objects
                    .get(child_id)
                    .unwrap()
                    .borrow()
                    .generate_outline(2, &mut export_string, &self.objects);
            }

            export_string.push_str("\n\n");
        }

        let characters = self.objects.get(&self.characters_id).unwrap().borrow();

        if !characters.get_base().children.is_empty() {
            export_string.push_str("# Characters\n\n");

            for child_id in characters.get_base().children.iter() {
                self.objects
                    .get(child_id)
                    .unwrap()
                    .borrow()
                    .generate_outline(2, &mut export_string, &self.objects);
            }

            export_string.push_str("\n\n");
        }

        let worldbuilding = self.objects.get(&self.worldbuilding_id).unwrap().borrow();

        if !worldbuilding.get_base().children.is_empty() {
            export_string.push_str("# Worldbuilding\n\n");

            for child_id in worldbuilding.get_base().children.iter() {
                self.objects
                    .get(child_id)
                    .unwrap()
                    .borrow()
                    .generate_outline(2, &mut export_string, &self.objects);
            }

            export_string.push_str("\n\n");
        }

        export_string
    }

    /// Export the story to a string (which can be written to a file)
    fn export_text(&self) -> String {
        let mut export_string = String::new();

        for child_id in self
            .objects
            .get(&self.text_id)
            .unwrap()
            .borrow()
            .get_base()
            .children
            .iter()
        {
            let testing_options = ExportOptions {
                folder_title_depth: ExportDepth::Some(1),
                scene_title_depth: ExportDepth::None,
                insert_breaks: true,
            };

            self.objects
                .get(child_id)
                .unwrap()
                .borrow()
                .generate_export(
                    1,
                    &mut export_string,
                    &self.objects,
                    &testing_options,
                    false,
                );
        }

        export_string
    }
}

fn u64_to_i64_drop_msb(val: u64) -> i64 {
    const MSB_MASK: u64 = u64::MAX >> 1;
    (val & MSB_MASK) as i64
}

pub struct ExportOptions {
    pub folder_title_depth: ExportDepth,
    pub scene_title_depth: ExportDepth,
    pub insert_breaks: bool,
}

pub enum ExportDepth {
    All,
    Some(u64),
    None,
}

impl ExportDepth {
    pub fn should_display(&self, depth: u64) -> bool {
        match self {
            ExportDepth::All => true,
            ExportDepth::Some(max_depth) => depth <= *max_depth,
            ExportDepth::None => false,
        }
    }
}
