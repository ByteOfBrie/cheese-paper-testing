use crate::cheese_error;
use crate::components::file_objects::{
    FileInfo, FileObject, FileObjectMetadata, FileObjectStore, Folder, from_file,
    write_with_temp_file,
};
use crate::components::text::Text;
use crate::util::CheeseError;
use notify::event::RenameMode;
use notify::{EventKind, event::ModifyKind};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebouncedEvent, Debouncer, RecommendedCache, new_debouncer};
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use toml_edit::DocumentMut;

use crate::components::file_objects::utils::{process_name_for_filename, write_outline_property};

use crate::components::file_objects::base::{
    FileID, FileObjectCreation, load_base_metadata, metadata_extract_bool, metadata_extract_string,
    metadata_extract_u64,
};

type RecommendedDebouncer = Debouncer<RecommendedWatcher, RecommendedCache>;
type WatcherReceiver = std::sync::mpsc::Receiver<Result<Vec<DebouncedEvent>, Vec<notify::Error>>>;

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

    file_event_rx: WatcherReceiver,

    /// We don't need to do anything to the watcher, but we stop getting events if it's dropped
    _watcher: RecommendedDebouncer,
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

/// Loads a special top level folder (e.g., "project/text/", "project/worldbuilding"), creating it if
/// it doesn't already exist.
///
/// Name will be used directly in the metadata name, but will be converted to lowercase for the filename
fn load_top_level_folder(
    project_path: &Path,
    name: &str,
) -> Result<(Folder, FileObjectStore), CheeseError> {
    log::debug!("loading top level folder: {name}");

    let folder_path = &Path::join(project_path, name.to_lowercase());
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

#[cfg(not(test))]
const WATCHER_MSEC_DURATION: u64 = 1000;

#[cfg(test)]
const WATCHER_MSEC_DURATION: u64 = 50;

fn create_watcher() -> notify::Result<(RecommendedDebouncer, WatcherReceiver)> {
    let (tx, rx) = std::sync::mpsc::channel();

    let watcher = new_debouncer(
        std::time::Duration::from_millis(WATCHER_MSEC_DURATION),
        None,
        tx,
    )?;

    Ok((watcher, rx))
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

        let text = Folder::new_top_level(project_path.clone(), "Text")?;
        let characters = Folder::new_top_level(project_path.clone(), "Characters")?;
        let worldbuilding = Folder::new_top_level(project_path.clone(), "Worldbuilding")?;

        let file = FileInfo {
            dirname,
            basename: OsString::from(file_safe_name),
            modtime: None,
            modified: true, // Newly added files are modified (they don't exist on disk)
        };

        // Create the watcher path by hand since we can't call get_path() yet
        let watcher_path = file.dirname.join(&file.basename);

        // this might later get wrapped in an optional block or something but not worth it right now
        let (mut watcher, file_event_rx) =
            create_watcher().expect("Should always be able to create a watcher");

        watcher
            .watch(watcher_path, RecursiveMode::Recursive)
            .unwrap();

        let mut project = Self {
            base_metadata: FileObjectMetadata {
                name: project_name,
                ..Default::default()
            },
            metadata: ProjectMetadata::default(),
            text_id: text.get_base().metadata.id.clone(),
            characters_id: characters.get_base().metadata.id.clone(),
            worldbuilding_id: worldbuilding.get_base().metadata.id.clone(),
            file,
            toml_header: DocumentMut::new(),
            objects: HashMap::new(),
            file_event_rx,
            _watcher: watcher,
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
        let (text, mut descendents) = load_top_level_folder(&path, "Text")?;

        let (characters, characters_descendents) = load_top_level_folder(&path, "Characters")?;

        let (worldbuilding, worldbuilding_descendents) =
            load_top_level_folder(&path, "Worldbuilding")?;

        log::debug!("Finished loading all project file objects, continuing");

        // merge all of the descendents into a single hashmap that owns all of them
        descendents.extend(characters_descendents);
        descendents.extend(worldbuilding_descendents);

        load_base_metadata(&toml_header, &mut base_metadata, &mut file_info)?;

        // Create the watcher path by hand since we can't call get_path() yet
        let watcher_path = file_info.dirname.join(&file_info.basename);

        // this might later get wrapped in an optional block or something but not worth it right now
        let (mut watcher, file_event_rx) =
            create_watcher().expect("Should always be able to create a watcher");

        watcher
            .watch(watcher_path, RecursiveMode::Recursive)
            .unwrap();

        let mut project = Self {
            metadata,
            base_metadata,
            file: file_info,
            text_id: text.get_base().metadata.id.clone(),
            characters_id: characters.get_base().metadata.id.clone(),
            worldbuilding_id: worldbuilding.get_base().metadata.id.clone(),
            toml_header,
            objects: descendents,
            file_event_rx,
            _watcher: watcher,
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
        self.toml_header["file_format_version"] =
            toml_edit::value(self.base_metadata.version as i64);
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

    /// Given a FileID, try to find the FileID of its parent
    pub fn find_object_parent(&self, needle: &FileID) -> Option<FileID> {
        for object in self.objects.values() {
            if object.borrow().get_base().children.contains(needle) {
                return Some(object.borrow().id().clone());
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
    pub fn export_text(&self, export_options: ExportOptions) -> String {
        let mut export_string = String::new();

        let mut include_break = false;

        for child_id in self
            .objects
            .get(&self.text_id)
            .unwrap()
            .borrow()
            .get_base()
            .children
            .iter()
        {
            include_break = self
                .objects
                .get(child_id)
                .unwrap()
                .borrow()
                .generate_export(
                    1,
                    &mut export_string,
                    &self.objects,
                    &export_options,
                    include_break,
                );
        }

        export_string
    }

    pub fn process_updates(&mut self) {
        // check for file system events and process them
        if let Ok(response) = self.file_event_rx.try_recv() {
            match response {
                Ok(events) => {
                    let mut file_objects_needing_rescan = HashSet::new();
                    let mut found_events = false;
                    for event in events {
                        let mut git_event = false;
                        for event_path in event.paths.iter() {
                            if event_path.iter().any(|component| component == ".git") {
                                git_event = true;
                            }
                        }
                        if git_event {
                            continue;
                        }
                        if let EventKind::Access(_) = event.kind {
                            continue;
                        }

                        // We now have an event that isn't noise from .git or file opens:
                        log::debug!("found event: {event:?}");
                        found_events = true;

                        match event.kind {
                            EventKind::Create(_create_kind) => {
                                let modify_path = event.paths.first().unwrap();
                                log::debug!("processing creation event: {event:?}");
                                if let Some(need_rescan_vec) =
                                    self.process_modify_event(modify_path)
                                {
                                    for need_rescan_id in need_rescan_vec {
                                        file_objects_needing_rescan.insert(need_rescan_id);
                                    }
                                }
                            }
                            EventKind::Modify(ModifyKind::Data(_data_change)) => {
                                let modify_path = event.paths.first().unwrap();
                                log::debug!("processing modify event: {event:?}");
                                if let Some(need_rescan_vec) =
                                    self.process_modify_event(modify_path)
                                {
                                    for need_rescan_id in need_rescan_vec {
                                        file_objects_needing_rescan.insert(need_rescan_id);
                                    }
                                }
                            }
                            EventKind::Modify(ModifyKind::Name(rename_mode)) => {
                                if let Some(need_rescan_vec) =
                                    self.process_rename_event(event, rename_mode)
                                {
                                    for need_rescan_id in need_rescan_vec {
                                        file_objects_needing_rescan.insert(need_rescan_id);
                                    }
                                }
                            }
                            EventKind::Remove(_remove_kind) => {
                                let delete_path = event
                                    .paths
                                    .first()
                                    .expect("Rename event should have source");

                                if let Some(fileid) = self.process_delete(delete_path) {
                                    file_objects_needing_rescan.insert(fileid);
                                }
                            }
                            _ => {}
                        }
                    }
                    for object_needing_rescan in file_objects_needing_rescan {
                        self.objects
                            .get(&object_needing_rescan)
                            .unwrap()
                            .borrow_mut()
                            .rescan_indexing(&self.objects);
                    }
                    if found_events {
                        log::debug!("finished processing events");
                    }
                }
                Err(err) => log::warn!("Error while trying to watch files: {err:?}"),
            }
        }
    }

    /// Determine if we care about an event happening at this path. This filters out things like events
    /// starting with `.git/`, hidden files (on linux), unknown extensions, or files not in one of the
    /// three top level folders
    ///
    /// It does not check for files existing, and does not do anything specific to modification types.
    /// The string argument is only to provide better log message output
    fn is_relevant_event_path(&self, modify_path: &Path, modification_type: &'static str) -> bool {
        // We assume that any files that don't have an extension are folders but this function
        // not checking disk means we can't verify that
        if modify_path
            .extension()
            .is_some_and(|extension| extension != "md" && extension != "toml")
        {
            // we write .tmp files and then immediately remove them and other editors can do the same
            // we also don't care about files that other programs generate
            return false;
        }

        if modify_path
            .file_name()
            .is_none_or(|filename| filename.to_string_lossy().starts_with('.'))
        {
            // modified files should have a name, and we don't want to look at hidden files
            return false;
        }

        let relative_path = match modify_path.strip_prefix(self.get_path()) {
            Ok(relative_path) => relative_path,
            Err(err) => {
                log::error!("invalid {modification_type} event path not in project: {err}");
                return false;
            }
        };

        if !(relative_path.starts_with("text")
            || relative_path.starts_with("characters")
            || relative_path.starts_with("worldbuilding"))
        {
            if !relative_path.starts_with(".git") {
                // We expect a bunch of git events, but other events are unexpected, so log it
                log::debug!(
                    "invalid {modification_type} event path not in project folders: {modify_path:?}"
                );
            }
            return false;
        }

        true
    }

    /// process a creation or modify event. These events are basically equivalent because of how
    /// different editors and programs actually write to disk, so we have to process them together.
    /// This could also be a file being moved into the project.
    ///
    /// Returns a directory if it should be rescanned
    fn process_modify_event(&mut self, modify_path: &Path) -> Option<Vec<FileID>> {
        // special case, check for the project info file *first*
        if *modify_path == self.get_project_info_file() {
            if let Err(err) = self.reload_file() {
                log::warn!("Could not reload project info file: {err}")
            }
            // regardless of what happened, we're done
            return None;
        }

        // Filter out events like .git or tmp files
        if !self.is_relevant_event_path(modify_path, "create/modify") {
            return None;
        }

        // Lastly, check if it still exists before trying to read
        if !modify_path.exists() {
            log::debug!(
                "Attempted to process modification of a file that no longer exists: {modify_path:?}"
            );
            return None;
        }

        if let Some(id) = self.find_object_by_path(modify_path) {
            let file_object = self.objects.get(&id).unwrap();

            log::debug!(
                "Processing modify event at path: {modify_path:?}\n\
                Found file object: {}, reloading file",
                file_object.borrow()
            );

            if let Err(err) = file_object.borrow_mut().reload_file() {
                log::warn!("Error loading file {}: {err}", file_object.borrow());
            }
            // This was a modify, not a creation, nothing to do
            None
        } else {
            log::debug!("Processing create/modify event at path: {modify_path:?}");

            let ancestors = modify_path.ancestors();

            for ancestor in ancestors {
                // We need to check if this object can be loaded, which means
                // that its parent is already in the tree
                let parent_path = match ancestor.parent() {
                    Some(parent) => parent,
                    None => {
                        log::error!(
                            "unexpected result while processing event: \
                            parents should exist and the loop should always \
                            finish before it escapes the project tree",
                        );
                        return None;
                    }
                };

                let parent_id = match self.find_object_by_path(parent_path) {
                    Some(id) => id,
                    None => continue,
                };

                let parent_object = self.objects.get(&parent_id).unwrap();

                let new_index = parent_object.borrow_mut().get_base().children.len();

                // We've found a parent, which means that this object should
                // have from_file called on it
                let (new_object, descendents) = match from_file(ancestor, Some(new_index)) {
                    Ok(file_object_creation) => file_object_creation.into_boxed(),
                    Err(err) => {
                        log::warn!(
                            "Could not open file as part of processing modifications: {err}, \
                                    giving up on processing event"
                        );
                        return None;
                    }
                };

                let id = new_object.borrow().id().clone();

                let existing_item = self.objects.get(&id);

                match existing_item {
                    Some(existing_object) => {
                        let old_path = existing_object.borrow().get_path();
                        if old_path == modify_path {
                            panic!(
                                "Found a file object seemingly missed by find_object_by_path. \
                                This should have been processed as a modification which is hard now, \
                                Giving up."
                            );
                        }

                        if !old_path.exists() {
                            let rename_results =
                                self.process_rename_movement(&old_path, &modify_path.to_path_buf());

                            // TODO: copy over descendants/children as well (I think it'll be required)

                            // Reload the file, we have to do another get to avoid making the borrow
                            // checker unhappy (instead of using the existing object)
                            if let Err(err) =
                                self.objects.get(&id).unwrap().borrow_mut().reload_file()
                            {
                                log::error!(
                                    "Error while reloading file during rename movement: {modify_path:?}, \
                                    id: {id:?}, err: {err:?}"
                                );
                            }

                            return rename_results;
                        } else {
                            // We've found duplicates, we could maybe return none and log this as an
                            // error, but for now we panic
                            panic!(
                                "Attempted to process new file at path {modify_path:?}, \
                                but found file_id {id:?}, also currently present at {old_path:?}"
                            );
                        }
                    }
                    None => {
                        // Add to the parent's list of children
                        parent_object
                            .borrow_mut()
                            .get_base_mut()
                            .children
                            .push(id.clone());

                        log::debug!("Loaded new file object: {id}");

                        // Add the parent object to the object list
                        self.objects.insert(id, new_object);

                        // Add all of the descendents to the list
                        for (id_string, object) in descendents {
                            self.objects.insert(id_string, object);
                        }

                        return Some(vec![parent_id]);
                    }
                }
            }
            unreachable!("Ancestors should be found or error before this point")
        }
    }

    /// Processes rename events as best-effort, currently cannot handle complex cases well
    ///
    /// Returns a list of file objects that need to be rescanned for indexing
    fn process_rename_event(
        &mut self,
        event: DebouncedEvent,
        rename_mode: RenameMode,
    ) -> Option<Vec<FileID>> {
        match rename_mode {
            RenameMode::From => {
                let delete_path = event
                    .paths
                    .first()
                    .expect("From rename should have a source");

                self.process_delete(delete_path).map(|fileid| vec![fileid])
            }
            RenameMode::To => {
                let dest_path = event
                    .paths
                    .last()
                    .expect("to event should have a destination");

                self.process_modify_event(dest_path)
            }
            RenameMode::Both => {
                log::debug!("Processing actual rename event: {event:?}");

                let source_path = event
                    .paths
                    .first()
                    .expect("Rename event should have source");

                let dest_path = event
                    .paths
                    .last()
                    .expect("Rename event should have destination");

                self.process_rename_movement(source_path, dest_path)
            }
            _ => {
                // Give up, we don't want to make assumptoins at this stage
                log::warn!(
                    "Encountered rename event: {event:?}, not enough information to continue processing"
                );
                None
            }
        }
    }

    fn process_rename_movement(
        &mut self,
        source_path: &PathBuf,
        dest_path: &PathBuf,
    ) -> Option<Vec<FileID>> {
        if source_path == dest_path {
            log::debug!(
                "Rename event: has the same source and dest ({source_path:?}), nothing to do"
            );
            return None;
        }

        let moving_file_id = match self.find_object_by_path(source_path) {
            Some(fileid) => fileid,
            None => {
                if dest_path.starts_with(self.get_path()) {
                    log::debug!("Processing move as modify event at path: {dest_path:?}");
                    return self.process_modify_event(dest_path);
                } else {
                    log::debug!(
                        "Processed file rename for object with non-object source path: {source_path:?}, \
                        nothing to do."
                    );
                    return None;
                }
            }
        };
        let dest_name = dest_path.file_name().expect("dest should have a file name");

        let source_directory = source_path
            .parent()
            .expect("source should have a directory");
        let dest_directory = dest_path.parent().expect("dest should have a directory");

        let source_parent_file_id = match self.find_object_by_path(source_directory) {
            Some(source_parent_id) => source_parent_id,
            None => {
                log::error!(
                    "Tried to move object but could not find it's parent: {source_directory:?}. \
                    source path: {source_path:?}, dest path: {dest_path:?}"
                );
                return None;
            }
        };

        let moving_object = self.objects.get(&moving_file_id).unwrap();

        // Update the filename (basename) based on what we've gotten (since this needs to happen
        // regardless of path). Indexing will happen during rescan (after all events are processed)
        moving_object.borrow_mut().get_base_mut().file.basename = dest_name.to_owned();
        // propagate that to any children
        for child in moving_object.borrow().children(&self.objects) {
            child
                .borrow_mut()
                .process_path_update(moving_object.borrow().get_path(), &self.objects);
        }

        // Easy case: the file has been renamed within the directory it's in
        if source_directory == dest_directory {
            // Currently, we don't do anything to cleanup the directory or filename in this case.
            // It'll probably happen later, but we don't bother now (this is complicated enough already)
            return Some(vec![source_parent_file_id]);
        }

        // More complicated case: the file has been moved to another part of the tree. We're basically
        // processing a move, but without doing the actual move outselves. This should probably be
        // cleaned up/deduplicated later (#128)
        let dest_file_id = match self.find_object_by_path(dest_directory) {
            Some(dest_file_id) => dest_file_id,
            None => {
                log::debug!(
                    "move from: {source_path:?} to {dest_path:?} moves file object out of project directory, processing as a delete"
                );
                return self.process_delete(dest_path).map(|fileid| vec![fileid]);
            }
        };

        // Remove the moving object from it's current parent
        let source_parent = self
            .objects
            .get(&source_parent_file_id)
            .expect("objects should contain source file id");

        let child_id_position = source_parent
            .borrow()
            .get_base()
            .children
            .iter()
            .position(|val| moving_file_id == *val)
            .unwrap_or_else(|| {
                panic!(
                    "Children should only be removed from their parents. \
                    child id: {moving_file_id}, parent: {source_parent_file_id}"
                )
            });

        let child_id_string = source_parent
            .borrow_mut()
            .get_base_mut()
            .children
            .remove(child_id_position);

        let dest_parent = self.objects.get(&dest_file_id).unwrap();

        // How do I find the proper place here?
        // Move the object into the children of dest (at the proper place)
        dest_parent
            .borrow_mut()
            .get_base_mut()
            .children
            .push(child_id_string);

        moving_object
            .borrow_mut()
            .process_path_update(dest_directory.to_path_buf(), &self.objects);

        Some(vec![source_parent_file_id, dest_file_id])
    }

    fn process_delete(&mut self, delete_path: &Path) -> Option<FileID> {
        if delete_path.exists() {
            log::debug!("Not processing delete event for file that still exists");
            return None;
        }

        let deleting_file_id = self.find_object_by_path(delete_path)?;

        let parent_file_id = match self.find_object_parent(&deleting_file_id) {
            Some(parent_file_id) => parent_file_id,
            None => {
                log::error!(
                    "Could not remove file object: {deleting_file_id}: Could not find parent"
                );
                return None;
            }
        };

        let removed_child = self.objects.remove(&deleting_file_id).unwrap();

        // We're misusing a function here, but it does what we want still. It assumes that the file
        // still exists on disk, while we know it expressly doesn't. A bunch of errors will be generated
        // and we can ignore all of them, since removal happens first
        let _ = removed_child
            .borrow_mut()
            .remove_file_object(&mut self.objects);

        let parent = self.objects.get(&parent_file_id).unwrap();

        // Remove this from the list of children
        let child_index = parent
            .borrow()
            .get_base()
            .children
            .iter()
            .position(|id| *id == deleting_file_id)
            .expect("child_id must be a child of this object");

        parent
            .borrow_mut()
            .get_base_mut()
            .children
            .remove(child_index);

        parent.borrow_mut().fix_indexing(&self.objects);

        Some(parent_file_id)
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
