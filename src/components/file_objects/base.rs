use egui_ltreeview::DirPosition;
use log::warn;
use std::collections::HashMap;
use uuid::Uuid;

use crate::components::file_objects::utils::{
    add_index_to_name, get_index_from_name, process_name_for_filename, truncate_name,
    write_with_temp_file,
};
use crate::components::file_objects::{Character, Folder, Place, Scene};
use crate::ui::{FileObjectEditor, RenderData};
use std::ffi::OsString;
use std::fmt::Debug;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use toml_edit::DocumentMut;

/// the maximum length of a name before we start trying to truncate it
const FILENAME_MAX_LENGTH: usize = 30;

/// filename of the object within a folder containing its metadata (without extension)
const FOLDER_METADATA_FILE_NAME: &str = "metadata.toml";

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
    pub version: u32,
    /// Name of the object (e.g., title of a scene, character name)
    pub name: String,
    /// ID unique across all objects. The reference implementations use UUIDv4, but any string
    /// is acceptable
    pub id: String,
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
pub struct BaseFileObject {
    pub metadata: FileObjectMetadata,
    /// Index (ordering within parent)
    pub index: Option<usize>,
    pub file: FileInfo,
    pub toml_header: DocumentMut,
    pub children: Vec<String>,
    pub _rdata: RenderData,
}

impl Default for FileObjectMetadata {
    fn default() -> Self {
        Self {
            version: 1u32,
            name: String::new(),
            id: Uuid::new_v4().as_hyphenated().to_string(),
        }
    }
}

// We have to we can safely convert from FileType to str, but not the reverse
// (which has a TryFrom) implementation
#[allow(clippy::from_over_into)]
impl Into<&str> for FileType {
    fn into(self) -> &'static str {
        match self {
            FileType::Scene => "scene",
            FileType::Folder => "folder",
            FileType::Character => "character",
            FileType::Place => "worldbuilding",
        }
    }
}

impl TryFrom<&str> for FileType {
    type Error = &'static str;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "scene" => Ok(FileType::Scene),
            "folder" => Ok(FileType::Folder),
            "character" => Ok(FileType::Character),
            "worldbuilding" => Ok(FileType::Place),
            _ => Err("Unknown file type"),
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

pub type FileObjectStore = HashMap<String, Box<dyn FileObject>>;

#[derive(Debug)]
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

pub fn metadata_extract_u32(table: &DocumentMut, field_name: &str) -> Result<Option<u32>> {
    Ok(match table.get(field_name) {
        Some(value) => Some(
            value
                .as_integer()
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::InvalidData,
                        format!("{field_name} was not an integer"),
                    )
                })?
                .try_into()
                .map_err(|_| {
                    Error::new(
                        ErrorKind::InvalidData,
                        format!("failed to convert {field_name} to u32"),
                    )
                })?,
        ),
        None => None,
    })
}

pub fn metadata_extract_string(table: &DocumentMut, field_name: &str) -> Result<Option<String>> {
    Ok(match table.get(field_name) {
        Some(value) => Some(
            value
                .as_str()
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::InvalidData,
                        format!("{field_name} was not string"),
                    )
                })?
                .to_owned(),
        ),
        None => None,
    })
}

pub fn metadata_extract_bool(table: &DocumentMut, field_name: &str) -> Result<Option<bool>> {
    Ok(match table.get(field_name) {
        Some(value) => Some(value.as_bool().ok_or_else(|| {
            Error::new(ErrorKind::InvalidData, format!("{field_name} was not bool"))
        })?),
        None => None,
    })
}

/// Reads the contents of a file from disk
pub fn read_file_contents(file_to_read: &Path) -> Result<(String, String)> {
    let extension = match file_to_read.extension() {
        Some(val) => val,
        None => return Err(Error::new(ErrorKind::InvalidData, "value was not string")),
    };

    let file_data = std::fs::read_to_string(file_to_read).expect("could not read file");

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
) -> Result<()> {
    match metadata_extract_u32(metadata_table, "version")? {
        Some(version) => metadata_object.version = version,
        None => file_info.modified = true,
    }

    match metadata_extract_string(metadata_table, "name")? {
        Some(name) => metadata_object.name = name,
        None => file_info.modified = true,
    }

    match metadata_extract_string(metadata_table, "id")? {
        Some(id) => metadata_object.id = id,
        None => file_info.modified = true,
    }

    Ok(())
}

pub fn run_with_file_object<T>(
    id_string: &str,
    objects: &mut FileObjectStore,
    func: impl FnOnce(&mut Box<dyn FileObject>, &mut FileObjectStore) -> T,
) -> T {
    let (object_id_string, mut object) = objects
        .remove_entry(id_string)
        .expect("id_string should always be contained within objects");

    let result = func(&mut object, objects);

    objects.insert(object_id_string, object);

    result
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

fn parent_contains(parent_id: &str, checking_id: &str, objects: &mut FileObjectStore) -> bool {
    run_with_file_object(parent_id, objects, |parent, objects| {
        for child_id in parent.get_base().children.iter() {
            // directly check if this is object we're looking for
            if child_id == checking_id {
                return true;
            }

            // check all of the children
            if parent_contains(child_id, checking_id, objects) {
                return true;
            }
        }

        // we didn't find the file object here, return false
        false
    })
}

/// Move a child between two folders, `source_file_id` and `dest_file_id`
///
/// This can't be part of the FileObject trait because ownership is complicated between
/// the
pub fn move_child(
    moving_file_id: &str,
    source_file_id: &str,
    dest_file_id: &str,
    new_index: usize,
    objects: &mut FileObjectStore,
) -> Result<()> {
    // Check for it being a valid move:
    // * can't move to one of your own children
    if parent_contains(moving_file_id, dest_file_id, objects) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("attempted to move {moving_file_id} into itself"),
        ));
    }

    // * can't move something without an index
    let moving = objects
        .get(moving_file_id)
        .expect("objects should contain moving file id");

    let moving_index = match moving.get_base().index {
        Some(index) => index,
        None => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("attempted to move {moving_file_id:} into itself"),
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
        run_with_file_object(source_file_id, objects, |source, objects| {
            source.fix_indexing(objects)
        });
    }

    Ok(())
}

/// Helper function called by move_child for the parts that are not safe to return early (including
/// errors). If something goes wrong, it will panic
fn create_index_and_move_on_disk(
    moving_file_id: &str,
    source_file_id: &str,
    dest_file_id: &str,
    new_index: usize,
    objects: &mut FileObjectStore,
) {
    // Create index "gap" in destination (helpful to do first in case we're moving "up" and this
    // changes the path of the object being moved)
    run_with_file_object(dest_file_id, objects, |dest, objects| {
        dest.create_index_gap(new_index, objects).unwrap();
    });

    // Remove the moving object from it's current parent
    let source = objects
        .get_mut(source_file_id)
        .expect("objects should contain source file id");

    let child_id_position = source
        .get_base()
        .children
        .iter()
        .position(|val| moving_file_id == val)
        .unwrap_or_else(|| {
            panic!(
                "Children should only be removed from their parents.\
                child id: {moving_file_id}, parent: {source_file_id}"
            )
        });

    let child_id_string = source.get_base_mut().children.remove(child_id_position);

    // Object is now removed from it's current parent, although still actually there on disk
    // We should also stop using `source` or the (scary) borrow checker will get mad at us

    // avoid borrowing twice:
    run_with_file_object(dest_file_id, objects, |dest, objects| {
        let insertion_index = std::cmp::min(new_index, dest.get_base().children.len());
        // Move the object into the children of dest (at the proper place)
        dest.get_base_mut()
            .children
            .insert(insertion_index, child_id_string);

        run_with_file_object(moving_file_id, objects, |child, objects| {
            // Move the actual child on disk
            if let Err(err) = child.move_object(insertion_index, dest.get_path(), objects) {
                // We don't pass enough information around to meaninfully recover here
                log::error!("Encountered error while trying to move {moving_file_id}");
                panic!(
                    "Encountered unrecoverable error while trying to move {moving_file_id}: {err}"
                );
            }
        });

        // Fix indexing in the destination (now that it has the child)
        dest.fix_indexing(objects);
    });
}

/// Load an arbitrary file object from a file on disk
pub fn from_file(filename: &Path, index: Option<usize>) -> Result<FileObjectCreation> {
    // Create the file info right at the start
    let mut file_info = FileInfo {
        dirname: match filename.parent() {
            Some(dirname) => dirname,
            None => {
                return Err(Error::new(
                    ErrorKind::InvalidFilename,
                    "filename supplied to from_file should have a dirname component",
                ));
            }
        }
        .to_path_buf(),
        basename: match filename.file_name() {
            Some(basename) => basename,
            None => {
                return Err(Error::new(
                    ErrorKind::InvalidFilename,
                    "filename supplied to from_file should have a basename component",
                ));
            }
        }
        .to_owned(),
        modtime: None,
        modified: false,
    };

    // If the filename is a directory, we need to look for the underlying file, otherwise
    // we already have it
    let underlying_file = match filename.is_dir() {
        true => Path::join(filename, FOLDER_METADATA_FILE_NAME),
        false => filename.to_path_buf(),
    };

    let (metadata_str, file_body) = match read_file_contents(&underlying_file) {
        Ok((metadata_str, file_body)) => (metadata_str, file_body),
        Err(err) => {
            if filename.is_dir() {
                ("".to_string(), "".to_string())
            } else {
                log::error!("Failed to read file {:?}: {:?}", &underlying_file, err);
                return Err(err);
            }
        }
    };

    let mut metadata = FileObjectMetadata::default();

    let toml_header = match metadata_str.parse::<DocumentMut>() {
        Ok(toml_header) => toml_header,
        Err(err) => {
            log::error!("Error parsing {underlying_file:?}: {err}");
            return Err(Error::new(ErrorKind::InvalidData, err));
        }
    };

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

    if let Err(err) = load_base_metadata(&toml_header, &mut metadata, &mut file_info) {
        log::error!("Error while parsing metadata for {:?}: {}", &filename, &err);
        return Err(err);
    }

    let file_type_str = match toml_header.get("file_type") {
        Some(val) => val.as_str().unwrap_or("unknown").to_owned(),
        None => match filename.is_dir() {
            true => "folder".to_string(),
            false => filename.extension().map_or_else(
                || "unknown".to_string(),
                |val| match val.to_str() {
                    Some("md") => "scene".to_string(),
                    Some("toml") => "unknown".to_string(),
                    _ => "unknown".to_string(),
                },
            ),
        },
    };

    let file_type: FileType = match file_type_str.as_str().try_into() {
        Ok(file_type) => file_type,
        Err(err) => {
            // The "correct" string is `worldbuilding`, but allow place anyway
            if file_type_str == "place" {
                FileType::Place
            } else {
                log::error!(
                    "Found unknown file type ({}) while attempt to read {:?}: {}",
                    &file_type_str,
                    &filename,
                    err
                );
                return Err(Error::new(ErrorKind::InvalidData, "unknown file type"));
            }
        }
    };

    let mut base = BaseFileObject {
        metadata,
        index,
        file: file_info,
        toml_header,
        children: Vec::new(),
        _rdata: RenderData::default(),
    };

    // Will eventually return this and all children
    let mut objects: FileObjectStore = HashMap::new();

    // Load children of this file object
    if file_type.is_folder() {
        if filename.is_dir() {
            match std::fs::read_dir(filename) {
                Ok(files) => {
                    let mut indexed_files: Vec<(usize, PathBuf)> = Vec::new();
                    let mut unindexed_files: Vec<PathBuf> = Vec::new();
                    for file in files {
                        match file {
                            Ok(file) => {
                                // We've already read this file, nothing to do
                                if file.path().file_name()
                                    == Some(&OsString::from(FOLDER_METADATA_FILE_NAME))
                                {
                                    continue;
                                }

                                let file_path = file.path();

                                let file_name_str = match file_path.file_name() {
                                    Some(file_name) => match file_name.to_str() {
                                        Some(file_name_str) => file_name_str,
                                        None => {
                                            log::error!(
                                                "Encountered file without valid unicode name: {file:?}"
                                            );
                                            continue;
                                        }
                                    },
                                    None => {
                                        log::error!(
                                            "Encountered file without valid basename name: {file:?}"
                                        );
                                        continue;
                                    }
                                };

                                match get_index_from_name(file_name_str) {
                                    Some(index) => {
                                        indexed_files.push((index, file.path()));
                                    }
                                    None => unindexed_files.push(file.path()),
                                };
                            }
                            Err(err) => {
                                warn!("Could not read file in folder {:?}: {}", &filename, &err)
                            }
                        }
                    }

                    // sort the list of files and grab the first one
                    indexed_files.sort();
                    let max_indexed_file = match indexed_files.last() {
                        Some((final_index, _file)) => *final_index,
                        None => 0,
                    };
                    let unindexed_offset = max_indexed_file + 1;

                    // add the unindexed files to the list, arbitrarily assigning them indexes
                    // (assuming they fall strictly *after* the file path)
                    for (index, file) in unindexed_files.drain(..).enumerate() {
                        indexed_files.push((index + unindexed_offset, file));
                    }

                    // Insert all of the files at their given indexes
                    //
                    // There may still be gaps at this point, but they'll get filled in at the end
                    // by `fix_indexing`
                    for (index, file) in indexed_files.drain(..) {
                        // We process every dir but only some files
                        if !file.is_dir() {
                            // Check for extension
                            if file.extension().unwrap_or_default() != "toml"
                                && file.extension().unwrap_or_default() != "md"
                            {
                                log::debug!("skipping regular {file:?} with unknown extension");
                                continue;
                            }
                        }
                        match from_file(&file, Some(index)) {
                            Ok(created_files) => {
                                let (object, mut descendents): (
                                    Box<dyn FileObject>,
                                    FileObjectStore,
                                ) = match created_files {
                                    FileObjectCreation::Scene(object, descendents) => {
                                        (Box::new(object), descendents)
                                    }
                                    FileObjectCreation::Folder(object, descendents) => {
                                        (Box::new(object), descendents)
                                    }
                                    FileObjectCreation::Character(object, descendents) => {
                                        (Box::new(object), descendents)
                                    }
                                    FileObjectCreation::Place(object, descendents) => {
                                        (Box::new(object), descendents)
                                    }
                                };

                                base.children.push(object.get_base().metadata.id.clone());
                                objects.insert(object.get_base().metadata.id.clone(), object);

                                for (child_file_id, child_file) in descendents.drain() {
                                    objects.insert(child_file_id, child_file);
                                }
                            }
                            Err(err) => {
                                log::warn!(
                                    "found invalid file while attempting to load {:?}, {}",
                                    &file,
                                    err
                                );
                            }
                        }
                    }
                }
                Err(err) => {
                    log::error!(
                        "Error while attempt to read folder {:?}: {}",
                        &filename,
                        &err
                    );
                    return Err(err);
                }
            }
        } else {
            log::error!(
                "attempted to construct a folder-type from a non-folder filename {:?}",
                &filename
            );
            return Err(Error::new(
                ErrorKind::InvalidFilename,
                format!(
                    "{:?} is a folder-type file object, but doesn't have a directory",
                    &filename
                ),
            ));
        }
        // We fix the indexing at the end when returning a folder or place
        // This will ensure that all children have the correct indexing. The only file objects
        // that aren't the children of some folder are the roots, which don't have indexing anyway
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

            folder.fix_indexing(&mut objects);

            Ok(FileObjectCreation::Folder(folder, objects))
        }

        FileType::Place => {
            let mut place = Place::from_base(base)?;

            place.fix_indexing(&mut objects);

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
            _rdata: RenderData::default(),
        }
    }

    fn write_metadata(&mut self) {
        self.toml_header["version"] = toml_edit::value(self.metadata.version as i64);
        self.toml_header["name"] = toml_edit::value(&self.metadata.name);
        self.toml_header["id"] = toml_edit::value(&self.metadata.id);
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

    /// Loads the file-specific metadata from the toml document
    ///
    /// pulls from the file object instead of an argument (otherwise it's slightly tricky to do ownership)
    fn load_metadata(&mut self) -> Result<bool>;

    /// Writes the current type-specific metadata to the BaseFileObjects toml_header
    fn write_metadata(&mut self);

    fn as_editor(&self) -> &dyn FileObjectEditor;

    fn as_editor_mut(&mut self) -> &mut dyn FileObjectEditor;

    /// Sets the index to this file, doing the move if necessary
    fn set_index(&mut self, new_index: usize, objects: &mut FileObjectStore) -> Result<bool> {
        if self.get_base().index == Some(new_index) {
            let name_index = get_index_from_name(&self.get_base().file.basename.to_string_lossy());
            if name_index == self.get_base().index {
                // We have the index in memory and on disk, there's nothing to be done here, return early
                // and avoid writing to disk
                return Ok(false);
            }
        }

        self.get_base_mut().index = Some(new_index);

        self.set_filename(self.calculate_filename(), objects)?;

        Ok(true)
    }

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

    /// Change the filename in the base object and on disk, processing any required updates
    fn set_filename(
        &mut self,
        new_filename: OsString,
        objects: &mut FileObjectStore,
    ) -> Result<()> {
        let old_path = self.get_path();
        let new_path = Path::join(&self.get_base().file.dirname, &new_filename);

        if new_path == old_path {
            // Nothing to do
            log::warn!(
                "tried to move {old_path:?} to itself (set_filename), harmless but shouldn't happen"
            );
            return Ok(());
        }

        self.get_base_mut().file.basename = new_filename;

        if let Err(err) = self.move_on_disk(old_path, new_path, objects) {
            log::error!(
                "failed to set filename of {self:?} to {:?}",
                self.get_base().file.basename
            );
            return Err(err);
        }

        Ok(())
    }

    /// Processes the actual move on disk of this file object. Does *not* handle any logic about
    /// parents or indexes, see `move_child`
    fn move_object(
        &mut self,
        new_index: usize,
        new_path: PathBuf,
        objects: &mut FileObjectStore,
    ) -> Result<()> {
        let old_path = self.get_path();

        self.get_base_mut().index = Some(new_index);
        self.get_base_mut().file.dirname = new_path;
        self.get_base_mut().file.basename = self.calculate_filename();
        let new_path = self.get_path();

        if new_path == old_path {
            // Nothing to do:
            log::warn!("tried to move {old_path:?} to itself, harmless but shouldn't happen");
            return Ok(());
        }

        log::debug!(
            "moving {} from {:#?} to {:?}",
            &self.get_base().metadata.name,
            &old_path,
            &new_path
        );

        self.move_on_disk(old_path, new_path, objects)
    }

    fn move_on_disk(
        &mut self,
        old_path: PathBuf,
        new_path: PathBuf,
        objects: &mut FileObjectStore,
    ) -> Result<()> {
        if new_path == old_path {
            // Nothing to do
            return Err(Error::new(
                ErrorKind::InvalidFilename,
                format!("attempted to rename {old_path:?} to itself"),
            ));
        }

        if new_path.exists() {
            return Err(Error::new(
                ErrorKind::InvalidFilename,
                format!("attempted to rename {old_path:?}, but {new_path:?} already exists"),
            ));
        }

        if old_path.exists() {
            std::fs::rename(old_path, new_path)?;
        }

        for child_id in self.get_base().children.iter() {
            run_with_file_object(child_id.as_str(), objects, |child, objects| {
                child.process_path_update(self.get_path(), objects);
            });
        }

        Ok(())
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

    /// When the parent changes path, updates this dirname and any other children
    fn process_path_update(&mut self, new_directory: PathBuf, objects: &mut FileObjectStore) {
        self.get_base_mut().file.dirname = new_directory;

        // Propogate this to any children
        for child_id in self.get_base().children.iter() {
            run_with_file_object(child_id.as_str(), objects, |child, objects| {
                child.process_path_update(self.get_path(), objects);
            });
        }
    }

    /// Determine if the file should be loaded
    fn should_load(&mut self, file_to_read: &Path) -> Result<bool> {
        let current_modtime = std::fs::metadata(file_to_read)
            .expect("attempted to load file that does not exist")
            .modified()
            .expect("Modtime not available");

        if let Some(old_modtime) = self.get_base().file.modtime {
            if old_modtime == current_modtime {
                // We've already loaded the latest revision, nothing to do
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Reloads the contents of this file object from disk. Assumes that the file has been properly
    /// initialized already
    fn reload_file(&mut self) -> Result<()> {
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

    fn save(&mut self, objects: &mut FileObjectStore) -> Result<()> {
        // First, try to save children, intentionally trying all of them
        let mut errors = vec![];
        for child_id in self.get_base().children.iter() {
            run_with_file_object(child_id.as_str(), objects, |child, objects| {
                if let Err(err) = child.save(objects) {
                    errors.push(err);
                }
            });
        }

        if !self.get_base().file.modified {
            // If we had *any* errors, return one of them
            return match errors.pop() {
                Some(err) => Err(err),
                None => Ok(()),
            };
        }

        // Check if the filename is "correct", updating it if necessary
        let calculated_filename = self.calculate_filename();
        if self.get_base().file.basename != calculated_filename {
            self.set_filename(calculated_filename, objects)?
        }

        // Ensure `toml_header` has the up-to-date metadata
        self.get_base_mut().write_metadata();
        self.write_metadata();

        let mut final_str = self.get_base().toml_header.to_string();

        // Add the scene body and the split (which we want to do even if there isn't any actual body)
        if self.has_body() {
            final_str.push_str(HEADER_SPLIT);
            final_str.push_str("\n\n");
            final_str.push_str(&self.get_body());
        }

        write_with_temp_file(&self.get_file(), final_str.as_bytes())?;

        let new_modtime = std::fs::metadata(self.get_file())
            .expect("attempted to load file that does not exist")
            .modified()
            .expect("Modtime not available");

        // Update modtime based on what we just wrote
        self.get_base_mut().file.modtime = Some(new_modtime);
        self.get_base_mut().file.modified = false;

        // If we had *any* errors, return one of them
        match errors.pop() {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    // Helper function to create a child at the end of a directory, which is much simpler
    #[cfg(test)]
    fn create_child_at_end(&mut self, file_type: FileType) -> Result<Box<dyn FileObject>> {
        assert!(self.is_folder());

        // We know it's at the end, and thus we know that there aren't any children
        self.create_child(file_type, DirPosition::Last, &mut HashMap::new())
    }
    /// Creates a child in this folder, returning it to be added to the list
    fn create_child(
        &mut self,
        file_type: FileType,
        position: DirPosition<String>,
        objects: &mut FileObjectStore,
    ) -> Result<Box<dyn FileObject>> {
        let new_index = match position {
            DirPosition::After(child) => {
                self.get_base()
                    .children
                    .iter()
                    .position(|id| *id == child)
                    .unwrap()
                    + 1
            }
            DirPosition::Before(child) => self
                .get_base()
                .children
                .iter()
                .position(|id| *id == child)
                .unwrap(),
            DirPosition::First => 0,
            DirPosition::Last => self.get_base().children.len(),
        };

        self.create_index_gap(new_index, objects)?;

        // It might not be the best behavior to recover from an error *after* a file is created on
        // disk, but that might not even be possible, and is kinda okay since we should only ever
        // overwrite that file by accident, even in the worst case
        let new_object: Box<dyn FileObject> = match file_type {
            FileType::Scene => Box::new(Scene::new(self.get_path(), new_index)?),
            FileType::Character => Box::new(Character::new(self.get_path(), new_index)?),
            FileType::Folder => Box::new(Folder::new(self.get_path(), new_index)?),
            FileType::Place => Box::new(Place::new(self.get_path(), new_index)?),
        };

        self.get_base_mut()
            .children
            .insert(new_index, new_object.get_base().metadata.id.clone());

        Ok(new_object)
    }

    fn remove_child(&mut self, child_id: &str, objects: &mut FileObjectStore) -> Result<()> {
        let mut errors = Vec::new();
        log::debug!(
            "Removing child {} from {}",
            child_id,
            self.get_base().metadata.id
        );
        let mut child = objects
            .remove(child_id)
            .expect("all children should be in objects");

        let children = child.get_base_mut().children.clone();

        // Go through the list backwards, so calling `fix_indexing` at the end
        // isn't expensive (having to do a bunch of moves)
        for descendant in children.iter().rev() {
            // save any errors for later
            if let Err(err) = child.remove_child(descendant, objects) {
                errors.push(err);
            }
        }

        // Remove this from the list of children
        let child_index = self
            .get_base_mut()
            .children
            .iter()
            .position(|id| id == child_id)
            .expect("child_id must be a child of this object");

        self.get_base_mut().children.remove(child_index);

        // finally, we need to take care of this file
        std::fs::remove_file(child.get_file())?;

        if child.is_folder() {
            std::fs::remove_dir(child.get_path())?;
        }

        self.fix_indexing(objects);

        // If we had any errors earlier, return them
        match errors.pop() {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    /// Creates a gap in the indexes, to be called immediately before a move
    fn create_index_gap(&mut self, index: usize, objects: &mut FileObjectStore) -> Result<()> {
        assert!(self.is_folder());

        let children = &self.get_base().children;

        // Ensure we have to do the work
        if index < children.len() {
            // Go backwards from the end of the list to the place where the gap is being created
            // to ensure that we don't have collisions with names
            for i in (index..children.len()).rev() {
                let child_id = children[i].as_str();

                run_with_file_object(child_id, objects, |child, objects| {
                    // Try to increase the index of the child
                    child.set_index(i + 1, objects)
                })?;
            }

            log::debug!(
                "created indexing gap in {} at {index}",
                &self.get_base().metadata.id
            );
        } else {
            log::debug!(
                "indexing gap requested at the end of {}, nothing to do",
                &self.get_base().metadata.id
            );
        }
        Ok(())
    }

    /// For ease of calling, `objects` can contain arbitrary objects, only values contained
    /// in `children` will actually be sorted.
    fn fix_indexing(&mut self, objects: &mut FileObjectStore) {
        for (count, child_id) in self.get_base().children.iter().enumerate() {
            match run_with_file_object(child_id, objects, |child, objects| {
                child.set_index(count, objects)
            }) {
                Ok(true) => {
                    log::debug!("Updated index of {child_id} to {count}")
                }
                Ok(false) => {}
                Err(err) => {
                    log::error!("Error while trying to fix indexing of child {child_id}: {err}");
                    panic!(
                        "Error during fix_indexing, cannot be sure if we have valid indexes anymore"
                    );
                }
            }
        }
    }
}

impl dyn FileObject {
    pub fn get_title(&self) -> String {
        if self.get_base().metadata.name.is_empty() {
            self.empty_string_name().to_string()
        } else {
            self.get_base().metadata.name.clone()
        }
    }
}
