mod implementation;
pub use implementation::*;

use bitflags::bitflags;
use std::collections::HashMap;
use uuid::Uuid;

use crate::cheese_error;
use crate::components::file_objects::utils::{
    add_index_to_name, get_index_from_name, process_name_for_filename, truncate_name,
};
use crate::components::file_objects::{Character, Folder, Place, Scene};
use crate::components::project::ExportOptions;
use crate::ui::FileObjectEditor;
use crate::util::CheeseError;
use std::cell::RefCell;
use std::ffi::OsString;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;
use toml_edit::DocumentMut;

/// the maximum length of a name before we start trying to truncate it
const FILENAME_MAX_LENGTH: usize = 30;

/// filename of the object within a folder containing its metadata (without extension)
pub const FOLDER_METADATA_FILE_NAME: &str = "metadata.toml";

/// Value that splits the header of any file that contains non-metadata content
const HEADER_SPLIT: &str = "++++++++";

/// Loading a file:
/// 1. Parse filename as a name -> metadata.name
/// 2. Load file, storing the metadata in some intermediate place
/// 3. Store the rest of the file into the metadata automatically (as present)
/// 4. Check for a meaningful name in the metadata (present and not the default), write if meaningful
///
/// Baseline metadata for all file objects
#[derive(Debug)]
pub struct FileObjectMetadata {
    /// Version of the object, can eventually be used to detect compatibility changes
    pub version: u64,
    /// Name of the object (e.g., title of a scene, character name)
    pub name: String,
    /// ID unique across all objects. The reference implementations use UUIDv4, but any string
    /// is acceptable
    pub id: Rc<String>,
}

/// List of known file types in this version of the editor. File types that aren't known will not
/// be read in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Scene,
    Folder,
    Character,
    Place,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum FileObjectTypeInterface<'a> {
    Scene(&'a Scene),
    Folder(&'a Folder),
    Character(&'a Character),
    Place(&'a Place),
}

impl From<FileObjectTypeInterface<'_>> for FileType {
    fn from(value: FileObjectTypeInterface) -> Self {
        match value {
            FileObjectTypeInterface::Scene(_) => FileType::Scene,
            FileObjectTypeInterface::Folder(_) => FileType::Folder,
            FileObjectTypeInterface::Character(_) => FileType::Character,
            FileObjectTypeInterface::Place(_) => FileType::Place,
        }
    }
}

pub enum MutFileObjectTypeInterface<'a> {
    Scene(&'a mut Scene),
    Folder(&'a mut Folder),
    Character(&'a mut Character),
    Place(&'a mut Place),
}

#[derive(Debug)]
pub struct BaseFileObject {
    pub metadata: FileObjectMetadata,
    /// Index (ordering within parent)
    pub index: Option<usize>,
    pub file: FileInfo,
    pub toml_header: DocumentMut,
    pub children: Vec<FileID>,
}

impl Default for FileObjectMetadata {
    fn default() -> Self {
        Self {
            version: 1u64,
            name: String::new(),
            id: Rc::new(Uuid::new_v4().as_hyphenated().to_string()),
        }
    }
}

impl From<FileType> for &str {
    fn from(val: FileType) -> Self {
        match val {
            FileType::Scene => "scene",
            FileType::Folder => "folder",
            FileType::Character => "character",
            FileType::Place => "worldbuilding",
        }
    }
}

impl TryFrom<&str> for FileType {
    type Error = CheeseError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "scene" => Ok(FileType::Scene),
            "folder" => Ok(FileType::Folder),
            "character" => Ok(FileType::Character),
            "worldbuilding" => Ok(FileType::Place),
            // "worldbuilding" is the proper string, but also accept "place"
            "place" => Ok(FileType::Place),
            _ => Err(cheese_error!("Unknown file type: {value}")),
        }
    }
}

impl FileType {
    fn is_folder(self) -> bool {
        match self {
            FileType::Scene => false,
            FileType::Folder => true,
            FileType::Character => false,
            FileType::Place => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    /// Path of the directory containing this file
    /// `/foo/bar/` -> `/foo`
    pub dirname: PathBuf,
    /// Path of the file within the dirname
    /// `/foo/bar/` -> `bar`
    pub basename: OsString,
    /// Modified time if the file exists
    pub modtime: Option<SystemTime>,
    pub modified: bool,
}

bitflags! {

    /// The presence of this particular object in the export.
    ///
    /// The `INCLUDE` will (likely) represent an "include in story" checkbox,
    /// if it's not set, none of the other options have any meaningful value.
    /// We (currently) accept bools as well, since that's most of the normal
    /// configuration.
    ///
    /// At a data level, it would probably make more sense to have (at least)
    /// three separate variables here (include, include_title, and break_at_end),
    /// where the latter two will have an enum with (default, always, and never),
    /// which is how it'll likely be presented in the UI, but doing it this way is
    /// more compact in the file.
    #[derive(Debug)]
    pub struct CompileStatus: u64 {
        const INCLUDE                = 0b0000_0000_0000_0001;
        const OVERRIDE_INCLUDE_TITLE = 0b0000_0000_0000_0010;
        const INCLUDE_TITLE          = 0b0000_0000_0000_0100;
        const OVERRIDE_BREAK_AT_END  = 0b0000_0000_0000_1000;
        const BREAK_AT_END           = 0b0000_0000_0001_0000;

        // allow for any bits, in case a future version of cheese-paper sets more
        const _ = !0;
    }
}

impl Default for CompileStatus {
    fn default() -> Self {
        CompileStatus::INCLUDE
    }
}

impl CompileStatus {
    pub fn include_title(&self) -> IncludeOptions {
        if self.contains(CompileStatus::INCLUDE_TITLE | CompileStatus::OVERRIDE_INCLUDE_TITLE) {
            IncludeOptions::Always
        } else if self.contains(CompileStatus::OVERRIDE_INCLUDE_TITLE) {
            IncludeOptions::Never
        } else {
            IncludeOptions::Default
        }
    }

    pub fn set_include_title(&mut self, options: IncludeOptions) {
        match options {
            IncludeOptions::Default => self.set(CompileStatus::OVERRIDE_INCLUDE_TITLE, false),
            IncludeOptions::Always => {
                self.set(CompileStatus::OVERRIDE_INCLUDE_TITLE, true);
                self.set(CompileStatus::INCLUDE_TITLE, true);
            }
            IncludeOptions::Never => {
                self.set(CompileStatus::OVERRIDE_INCLUDE_TITLE, true);
                self.set(CompileStatus::INCLUDE_TITLE, false);
            }
        }
    }

    pub fn break_at_end(&self) -> IncludeOptions {
        if self.contains(CompileStatus::BREAK_AT_END | CompileStatus::OVERRIDE_BREAK_AT_END) {
            IncludeOptions::Always
        } else if self.contains(CompileStatus::OVERRIDE_BREAK_AT_END) {
            IncludeOptions::Never
        } else {
            IncludeOptions::Default
        }
    }

    pub fn set_break_at_end(&mut self, options: IncludeOptions) {
        match options {
            IncludeOptions::Default => self.set(CompileStatus::OVERRIDE_BREAK_AT_END, false),
            IncludeOptions::Always => {
                self.set(CompileStatus::OVERRIDE_BREAK_AT_END, true);
                self.set(CompileStatus::BREAK_AT_END, true);
            }
            IncludeOptions::Never => {
                self.set(CompileStatus::OVERRIDE_BREAK_AT_END, true);
                self.set(CompileStatus::BREAK_AT_END, false);
            }
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum IncludeOptions {
    #[default]
    Default,
    Always,
    Never,
}

pub fn metadata_extract_u64(
    table: &DocumentMut,
    field_name: &str,
    allow_bool: bool,
) -> Result<Option<u64>, CheeseError> {
    match table.get(field_name) {
        Some(value) => {
            if let Some(value) = value.as_integer() {
                Ok(Some(value as u64))
            } else if allow_bool && let Some(value) = value.as_bool() {
                Ok(Some(value as u64))
            } else {
                Err(cheese_error!("{field_name} was not an integer"))
            }
        }
        None => Ok(None),
    }
}

pub fn metadata_extract_string(
    table: &DocumentMut,
    field_name: &str,
) -> Result<Option<String>, CheeseError> {
    Ok(match table.get(field_name) {
        Some(value) => Some(
            value
                .as_str()
                .ok_or_else(|| cheese_error!("{field_name} was not string"))?
                .to_owned(),
        ),
        None => None,
    })
}

pub fn metadata_extract_bool(
    table: &DocumentMut,
    field_name: &str,
) -> Result<Option<bool>, CheeseError> {
    Ok(match table.get(field_name) {
        Some(value) => Some(
            value
                .as_bool()
                .ok_or_else(|| cheese_error!("{field_name} was not bool"))?,
        ),
        None => None,
    })
}

/// Reads the contents of a file from disk
pub fn read_file_contents(file_to_read: &Path) -> Result<(String, String), CheeseError> {
    let extension = match file_to_read.extension() {
        Some(val) => val,
        None => return Err(cheese_error!("value was not string")),
    };

    let file_data = std::fs::read_to_string(file_to_read)?;

    let (metadata_str, file_content): (&str, &str) = match extension == "md" {
        false => (&file_data, ""),
        true => match file_data.split_once(HEADER_SPLIT) {
            None => ("", &file_data),
            Some((start, end)) => (start, end),
        },
    };

    Ok((metadata_str.to_owned(), file_content.trim().to_owned()))
}

/// Given a freshly read metadata dictionary, read it into the file objects, setting modified as
/// appropriate
pub fn load_base_metadata(
    metadata_table: &DocumentMut,
    metadata_object: &mut FileObjectMetadata,
    file_info: &mut FileInfo,
) -> Result<(), CheeseError> {
    match metadata_extract_u64(metadata_table, "file_format_version", false)? {
        Some(version) => metadata_object.version = version,
        None => file_info.modified = true,
    }

    match metadata_extract_string(metadata_table, "name")? {
        Some(name) => metadata_object.name = name,
        None => file_info.modified = true,
    }

    match metadata_extract_string(metadata_table, "id")? {
        Some(id) => metadata_object.id = Rc::new(id),
        None => file_info.modified = true,
    }

    Ok(())
}

/// The object that was requested,
/// All of the descendents of that file object (including children) in a hashmap that owns them
#[derive(Debug)]
pub enum FileObjectCreation {
    Scene(Scene, FileObjectStore),
    Folder(Folder, FileObjectStore),
    Character(Character, FileObjectStore),
    Place(Place, FileObjectStore),
}

impl FileObjectCreation {
    pub fn into_boxed(self) -> (Box<RefCell<dyn FileObject>>, FileObjectStore) {
        match self {
            FileObjectCreation::Scene(parent, children) => {
                (Box::new(RefCell::new(parent)), children)
            }
            FileObjectCreation::Character(parent, children) => {
                (Box::new(RefCell::new(parent)), children)
            }
            FileObjectCreation::Folder(parent, children) => {
                (Box::new(RefCell::new(parent)), children)
            }
            FileObjectCreation::Place(parent, children) => {
                (Box::new(RefCell::new(parent)), children)
            }
        }
    }
}

fn parent_contains(parent_id: &FileID, checking_id: &FileID, objects: &FileObjectStore) -> bool {
    let parent = objects.get(parent_id).unwrap();

    for child_id in &parent.borrow().get_base().children {
        // directly check if this is object we're looking for
        if child_id == checking_id {
            return true;
        }

        // check all of the children
        if parent_contains(child_id, checking_id, objects) {
            return true;
        }
    }

    false
}

/// Move a child between two folders, `source_file_id` and `dest_file_id`
///
/// This can't be part of the FileObject trait because ownership is complicated between
/// the
pub fn move_child(
    moving_file_id: &FileID,
    source_file_id: &FileID,
    dest_file_id: &FileID,
    new_index: usize,
    objects: &FileObjectStore,
) -> Result<(), CheeseError> {
    // Check for it being a valid move:
    // * can't move to one of your own children
    if parent_contains(moving_file_id, dest_file_id, objects) {
        return Err(cheese_error!(
            "attempted to move {moving_file_id} into itself"
        ));
    }

    // * can't move something without an index
    let moving = objects
        .get(moving_file_id)
        .expect("objects should contain moving file id");

    let moving_index = match moving.borrow().get_base().index {
        Some(index) => index,
        None => {
            return Err(cheese_error!(
                "attempted to move {moving_file_id:} into itself"
            ));
        }
    };
    // * shouldn't move something where it already is
    if source_file_id == dest_file_id && moving_index == new_index {
        log::debug!("attempted to move {moving_file_id} to itself, skipping");
        return Ok(());
    }

    // We know it's a valid move (or at least think we do), go ahead with the move

    // From this point until the call to fix indexing, we have state that we can't safely recover
    // from with an error, so we should always panic instead
    create_index_and_move_on_disk(
        moving_file_id,
        source_file_id,
        dest_file_id,
        new_index,
        objects,
    );

    // if we're moving within an object, we already fixed indexing a few lines above
    if source_file_id != dest_file_id {
        // We just need to clean up and re-index the source to fill in the gap we left
        objects
            .get(source_file_id)
            .unwrap()
            .borrow_mut()
            .fix_indexing(objects);
    }

    Ok(())
}

/// Helper function called by move_child for the parts that are not safe to return early (including
/// errors). If something goes wrong, it will panic
fn create_index_and_move_on_disk(
    moving_file_id: &FileID,
    source_file_id: &FileID,
    dest_file_id: &FileID,
    new_index: usize,
    objects: &FileObjectStore,
) {
    // Create index "gap" in destination (helpful to do first in case we're moving "up" and this
    // changes the path of the object being moved)
    objects
        .get(dest_file_id)
        .unwrap()
        .borrow_mut()
        .create_index_gap(new_index, objects)
        .unwrap();

    // Remove the moving object from it's current parent
    let source = objects
        .get(source_file_id)
        .expect("objects should contain source file id");

    let child_id_position = source
        .borrow()
        .get_base()
        .children
        .iter()
        .position(|val| moving_file_id == val)
        .unwrap_or_else(|| {
            panic!(
                "Children should only be removed from their parents. \
                child id: {moving_file_id}, parent: {source_file_id}"
            )
        });

    let child_id_string = source
        .borrow_mut()
        .get_base_mut()
        .children
        .remove(child_id_position);

    // Object is now removed from it's current parent, although still actually there on disk
    // We should also stop using `source` or the (scary) borrow checker will get mad at us
    // update: we have added some refcells everywhere and the borrow checker is nicer now

    let dest = objects.get(dest_file_id).unwrap();

    let insertion_index = std::cmp::min(new_index, dest.borrow().get_base().children.len());
    // Move the object into the children of dest (at the proper place)
    dest.borrow_mut()
        .get_base_mut()
        .children
        .insert(insertion_index, child_id_string);

    let child = objects.get(moving_file_id).unwrap();

    // Move the actual child on disk
    if let Err(err) =
        child
            .borrow_mut()
            .move_object(insertion_index, dest.borrow().get_path(), objects)
    {
        // We don't pass enough information around to meaninfully recover here
        log::error!("Encountered error while trying to move {moving_file_id}");
        panic!("Encountered unrecoverable error while trying to move {moving_file_id}: {err}");
    }

    // Fix indexing in the destination (now that it has the child)
    dest.borrow_mut().fix_indexing(objects);
}

/// Load an arbitrary file object from a file on disk
pub fn from_file(filename: &Path) -> Result<FileObjectCreation, CheeseError> {
    if !filename.exists() {
        return Err(cheese_error!(
            "from_file cannot load file that does not exist: {filename:?}"
        ));
    }

    // We process every dir, but only `.toml` or `.md` files
    if !filename.is_dir()
        && filename
            .extension()
            .is_none_or(|extension| extension != "toml" && extension != "md")
    {
        return Err(cheese_error!(
            "from_file cannot load file {filename:?} with unknown extension"
        ));
    }

    // Create the file info right at the start
    let mut file_info = FileInfo {
        dirname: filename
            .parent()
            .ok_or(cheese_error!(
                "filename supplied to from_file should have a dirname component",
            ))?
            .to_path_buf(),
        basename: filename
            .file_name()
            .ok_or(cheese_error!(
                "filename supplied to from_file should have a basename component",
            ))?
            .to_owned(),
        modtime: None,
        modified: false,
    };

    // If the filename is a directory, we need to look for the underlying file
    let underlying_file = match filename.is_dir() {
        true => filename.join(FOLDER_METADATA_FILE_NAME),
        false => filename.to_path_buf(),
    };

    let (metadata_str, file_body) = read_file_contents(&underlying_file).or_else(|err| {
        if filename.is_dir() {
            Ok(("".to_string(), "".to_string()))
        } else {
            Err(cheese_error!(
                "Failed to read file {underlying_file:?}: {err}"
            ))
        }
    })?;

    let mut metadata = FileObjectMetadata::default();

    let toml_header = metadata_str
        .parse::<DocumentMut>()
        .map_err(|err| cheese_error!("Error parsing {underlying_file:?}: {err}"))?;

    if !toml_header.contains_key("name") {
        let file_name = PathBuf::from(&file_info.basename)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let name_to_parse = if let Some((prefix, suffix)) = file_name.split_once('-') {
            match prefix.parse::<i64>() {
                Ok(_) => suffix,
                Err(_) => file_name.as_str(),
            }
        } else {
            file_name.as_str()
        };

        metadata.name = name_to_parse.replace("_", " ").trim().to_string();
        if !metadata.name.is_empty() {
            file_info.modified = true;
        }
    }

    load_base_metadata(&toml_header, &mut metadata, &mut file_info)
        .map_err(|err| cheese_error!("Error while parsing metadata for {filename:?}: {err}"))?;

    let file_type: FileType = match toml_header.get("file_type") {
        Some(file_type_toml_item) => match file_type_toml_item.as_str() {
            Some(file_type_str) => file_type_str.try_into().map_err(|err| {
                cheese_error!("could not get file_type for file {filename:?}: {err}")
            })?,
            None => {
                return Err(cheese_error!(
                    "file header contained non-string value for file_type: {filename:?}"
                ));
            }
        },
        None => match filename.is_dir() {
            true => FileType::Folder,
            false => match filename.extension().and_then(|ext| ext.to_str()) {
                Some("md") => FileType::Scene,
                _ => {
                    return Err(cheese_error!(
                        "Unspecified file type file type while attempting to read {filename:?}"
                    ));
                }
            },
        },
    };

    let mut base = BaseFileObject {
        metadata,
        index: get_index_from_name(&file_info.basename.to_string_lossy()),
        file: file_info,
        toml_header,
        children: Vec::new(),
    };

    // Will eventually return this and all children
    let mut objects: FileObjectStore = HashMap::new();

    // Load children of this file object
    if file_type.is_folder() {
        if !filename.is_dir() {
            return Err(cheese_error!(
                "{filename:?} has a folder-based file_type, but isn't actually a directory",
            ));
        }

        // We rescan and fix the indexing at the end when returning a folder or place, so we can read
        // the files in any order here. The only files that won't ever be affected by this are the
        // roots, which don't have indexing anyway
        for file in std::fs::read_dir(filename).map_err(|err| {
            cheese_error!("Error while attempt to read folder {filename:?}: {err}")
        })? {
            match file {
                Ok(file) => {
                    // We've already read this file, nothing to do
                    if file.file_name() == FOLDER_METADATA_FILE_NAME {
                        continue;
                    }

                    let file_path = file.path();

                    // Just read the children in any order, we'll clean it up later
                    match from_file(&file_path) {
                        Ok(created_files) => {
                            let (object, mut descendents) = created_files.into_boxed();

                            let id = object.borrow().id().clone();
                            base.children.push(id.clone());
                            objects.insert(id, object);

                            for (child_file_id, child_file) in descendents.drain() {
                                objects.insert(child_file_id, child_file);
                            }
                        }
                        Err(err) => {
                            log::debug!("Could not load child {file:?}: {err}")
                        }
                    }
                }
                Err(err) => log::warn!("Could not read file {filename:?}: {err}"),
            }
        }
    }

    match file_type {
        FileType::Scene => {
            let mut scene = Scene::from_file_object(base)?;
            scene.load_body(file_body);
            Ok(FileObjectCreation::Scene(scene, objects))
        }
        FileType::Character => Ok(FileObjectCreation::Character(
            Character::from_base(base)?,
            objects,
        )),
        FileType::Folder => {
            let mut folder = Folder::from_base(base)?;

            <dyn FileObject>::rescan_indexing(&mut folder, &objects);

            Ok(FileObjectCreation::Folder(folder, objects))
        }

        FileType::Place => {
            let mut place = Place::from_base(base)?;

            <dyn FileObject>::rescan_indexing(&mut place, &objects);

            Ok(FileObjectCreation::Place(place, objects))
        }
    }
}

impl BaseFileObject {
    /// Create a new file object in a folder
    pub fn new(dirname: PathBuf, index: Option<usize>) -> Self {
        Self {
            metadata: FileObjectMetadata::default(),
            index,
            file: FileInfo {
                dirname,
                basename: OsString::new(),
                modtime: None,
                modified: true, // Newly added files are modified (they don't exist on disk)
            },
            toml_header: DocumentMut::new(),
            children: Vec::new(),
        }
    }

    fn write_metadata(&mut self) {
        self.toml_header["file_format_version"] = toml_edit::value(self.metadata.version as i64);
        self.toml_header["name"] = toml_edit::value(&self.metadata.name);
        self.toml_header["id"] = toml_edit::value(&*self.metadata.id);
    }
}

pub trait FileObject: Debug {
    fn get_base(&self) -> &BaseFileObject;
    fn get_base_mut(&mut self) -> &mut BaseFileObject;

    /// If this has a body, currently only true for `Scene`
    fn has_body(&self) -> bool;
    /// Load the body when loading this file object
    fn load_body(&mut self, body: String);
    /// Gets the contents of the body to be written when saving
    fn get_body(&self) -> String;

    fn empty_string_name(&self) -> &'static str;
    fn is_folder(&self) -> bool;
    fn extension(&self) -> &'static str;

    /// Allow for downcasting this as a reference, useful for some UI components
    #[allow(dead_code)]
    fn get_file_type<'a>(&'a self) -> FileObjectTypeInterface<'a>;
    /// Allow for downcasting this as a mutable reference, useful for some UI components
    fn get_file_type_mut<'a>(&'a mut self) -> MutFileObjectTypeInterface<'a>;

    /// Display the outline, writing all relevant non-prose information we have to a single
    /// markdown file that can be scanned/shared easily. We don't (currently) have any selections
    /// on export, everything gets included
    fn generate_outline(&self, depth: u64, export_string: &mut String, objects: &FileObjectStore);

    /// Generate an export of story text, will be overridden by objects that actually generate
    /// (folder and scene)
    ///
    /// `include_break` adds a break at the beginning if appropriate, and this function returns
    /// `true` if the next function should include a break
    fn generate_export(
        &self,
        _current_depth: u64,
        _export_string: &mut String,
        _objects: &FileObjectStore,
        _export_options: &ExportOptions,
        include_break: bool,
    ) -> bool {
        // we don't do anything by default, but we want to pass on the include
        include_break
    }

    /// Loads the file-specific metadata from the toml document
    ///
    /// pulls from the file object instead of an argument (otherwise it's slightly tricky to do ownership)
    fn load_metadata(&mut self) -> Result<bool, CheeseError>;

    /// Writes the current type-specific metadata to the BaseFileObjects toml_header
    fn write_metadata(&mut self, objects: &FileObjectStore);

    fn as_editor(&self) -> &dyn FileObjectEditor;

    fn as_editor_mut(&mut self) -> &mut dyn FileObjectEditor;

    /// Calculates the filename for a particular object
    fn calculate_filename(&self) -> OsString {
        let base_name: &str = match self.get_base().metadata.name.is_empty() {
            false => &self.get_base().metadata.name,
            true => self.empty_string_name(),
        };

        let mut basename = match self.get_base().index {
            Some(index) => {
                let truncated_name = truncate_name(base_name, FILENAME_MAX_LENGTH);
                let file_safe_name = process_name_for_filename(truncated_name);
                let final_name = add_index_to_name(&file_safe_name, index);

                OsString::from(final_name)
            }
            None => OsString::from(process_name_for_filename(base_name)),
        };

        if !self.is_folder() {
            basename.push(".");
            basename.push(self.extension());
        }

        basename
    }

    /// Calculates the object's current path. For objects in a single file, this is their path
    /// (including the extension), for folder-based objects (i.e., Folder, Place), this is the
    /// path to the folder.
    ///
    /// Also see `get_file`
    fn get_path(&self) -> PathBuf {
        Path::join(
            &self.get_base().file.dirname,
            &self.get_base().file.basename,
        )
    }

    /// The path to an object's underlying file, the equivalent of `get_path` when doing file
    /// operations on this object
    fn get_file(&self) -> PathBuf {
        let base_path = self.get_path();
        if self.is_folder() {
            Path::join(&base_path, FOLDER_METADATA_FILE_NAME)
        } else {
            base_path
        }
    }

    /// Determine if the file should be loaded
    fn should_load(&mut self, file_to_read: &Path) -> Result<bool, CheeseError> {
        let current_modtime = match std::fs::metadata(file_to_read) {
            Ok(file_metadata) => file_metadata.modified()?,
            Err(err) => {
                log::warn!(
                    "attempted to load file that does not exist: {:?}",
                    file_to_read
                );
                return Err(err.into());
            }
        };

        if let Some(old_modtime) = self.get_base().file.modtime
            && old_modtime == current_modtime
        {
            // We've already loaded the latest revision, nothing to do
            return Ok(false);
        }

        Ok(true)
    }

    /// Reloads the contents of this file object from disk. Assumes that the file has been properly
    /// initialized already
    fn reload_file(&mut self) -> Result<(), CheeseError> {
        let file_to_read = self.get_file();

        if !self.should_load(&file_to_read)? {
            return Ok(());
        }

        let (metadata_str, file_body) = read_file_contents(&file_to_read)?;

        let new_toml_header = metadata_str
            .parse::<DocumentMut>()
            .expect("invalid file metadata header");

        let base_file_object = self.get_base_mut();

        load_base_metadata(
            &new_toml_header,
            &mut base_file_object.metadata,
            &mut base_file_object.file,
        )?;

        base_file_object.toml_header = new_toml_header;

        self.load_metadata()?;

        self.load_body(file_body);

        Ok(())
    }
}

impl std::fmt::Display for dyn FileObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[File Object | name=\"{}\" | id={}]",
            self.get_title(),
            self.id()
        )
    }
}
