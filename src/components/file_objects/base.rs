use log::warn;
use std::collections::HashMap;
use uuid::Uuid;

use crate::components::file_objects::utils::{
    add_index_to_name, get_index_from_name, process_name_for_filename, truncate_name,
    write_with_temp_file,
};
use crate::components::file_objects::{Character, Folder, Place, Scene};
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

#[derive(Debug)]
#[allow(dead_code)]
pub enum FileObjectTypeInterface<'a> {
    Scene(&'a Scene),
    Folder(&'a Folder),
    Character(&'a Character),
    Place(&'a Place),
}

pub enum MutFileObjectTypeInterface<'a> {
    Scene(&'a mut Scene),
    Folder(&'a mut Folder),
    Character(&'a mut Character),
    Place(&'a mut Place),
}
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
fn read_file_contents(file_to_read: &Path) -> Result<(String, String)> {
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

/// For ease of calling, `objects` can contain arbitrary objects, only values contained
/// in `children` will actually be sorted.
fn fix_indexing(children: &mut Vec<String>, objects: &mut FileObjectStore) -> usize {
    for (count, child_id) in children.iter().enumerate() {
        let (child_id, mut child) = objects
            .remove_entry(child_id.as_str())
            .expect("fix_indexing needs to borrow a map with the children");

        let child_base = child.get_base();

        if child_base
            .index
            .expect("Children should always have indexes")
            != count
        {
            if let Err(err) = child.set_index(count, objects) {
                log::error!(
                    "Error while trying to fix indexing of child {:?}: {}",
                    child,
                    err
                );

                // break out of the loop, returning early
                objects.insert(child_id, child);
                break;
            }
        }

        objects.insert(child_id, child);
    }

    children.len()
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
    let (parent_id_string, parent) = objects
        .remove_entry(parent_id)
        .expect("objects should contain parent id");

    let mut found = false;

    for child_id in parent.get_base().children.iter() {
        // directly check if this is object we're looking for
        if child_id == checking_id {
            found = true;
            break;
        }

        // check all of the children
        if parent_contains(&child_id, checking_id, objects) {
            found = true;
            break;
        }
    }

    objects.insert(parent_id_string, parent);
    return found;
}

/// Creates a gap in the indexes, to be called immediately before a move
fn create_index_gap(parent_id: &str, index: usize, objects: &mut FileObjectStore) -> Result<()> {
    let (parent_id_string, parent) = objects
        .remove_entry(parent_id)
        .expect("objects should contain parent id");

    let children = &parent.get_base().children;

    // Ensure we have to do the work
    if index < children.len() {
        // Go backwards from the end of the list to the place where the gap is being created
        // to ensure that we don't have collisions with names
        for i in (index..children.len()).rev() {
            let child_id = children[i].as_str();

            let (child_id_string, mut child) = objects
                .remove_entry(child_id)
                .expect("create_index_gap needs to borrow a map with the children");

            // Try to increase the index of the child
            if let Err(err) = child.set_index(i + 1, objects) {
                objects.insert(child_id_string, child);
                objects.insert(parent_id_string, parent);

                return Err(err);
            }

            objects.insert(child_id_string, child);
        }
    }

    objects.insert(parent_id_string, parent);

    Ok(())
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
        log::warn!("attempted to move {moving_file_id} to itself, skipping");
        return Ok(());
    }

    // We know it's a valid move (or at least think we do), go ahead with the move

    // Create index "gap" in destination (helpful to do first in case we're moving "up" and this
    // changes the path of the object being moved)
    create_index_gap(dest_file_id, new_index, objects)?;

    // Remove the moving object from it's current parent
    let source = objects
        .get_mut(source_file_id)
        .expect("objects should contain source file id");

    let child_id_position = match source
        .get_base()
        .children
        .iter()
        .position(|val| moving_file_id == val)
    {
        Some(child_starting_index) => child_starting_index,
        None => {
            // This should be impossible but we check anyway
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Attempted to remove a child from an element that doesn't contain it: \
                        child id: {moving_file_id}, parent: {source_file_id}",
                ),
            ));
        }
    };

    let child_id_string = source.get_base_mut().children.remove(child_id_position);

    // Object is now removed from it's current parent, although still actually there on disk
    // We should also stop using `source` or the (scary) borrow checker will get mad at us

    // Remove dest from the object list (to avoid borrowing twice)
    let (dest_id_string, mut dest) = objects
        .remove_entry(dest_file_id)
        .expect("dest must be in the object map when calling move");

    let insertion_index = std::cmp::min(new_index, dest.get_base().children.len());
    // Move the object into the children of dest (at the proper place)
    dest.get_base_mut()
        .children
        .insert(insertion_index, child_id_string);

    let (child_id_string, mut child) = objects
        .remove_entry(moving_file_id)
        .expect("the moved object needs to be in the object map");

    // Move the actual child on disk
    if let Err(err) = child.move_object(insertion_index, &dest.get_path(), objects) {
        // be as graceful as possible, put the children back, the current state is still likely
        // sorta broken :/
        objects.insert(child_id_string, child);
        objects.insert(dest_id_string, dest);
        log::error!("Encountered error while trying to move {moving_file_id}");
        return Err(err);
    }

    // We no longer have ownership of the child
    objects.insert(child_id_string, child);

    // Fix indexing in the destination (now that it has the child)
    fix_indexing(&mut dest.get_base_mut().children, objects);

    // Put the destination back in in the map
    objects.insert(dest_id_string, dest);

    // if we're moving within an object, we already fixed indexing a few lines above
    if source_file_id != dest_file_id {
        // We just need to clean up and re-index the source to fill in the gap we left
        let (source_id_string, mut source) = objects
            .remove_entry(dest_file_id)
            .expect("source must be in the object map when calling move");

        fix_indexing(&mut source.get_base_mut().children, objects);

        objects.insert(source_id_string, source);
    }

    Ok(())
}

// TODO: this function probably doesn't make sense as an option (instead of result) given the other code I'm writing
/// Load an arbitrary file object from a file on disk
pub fn from_file(filename: &Path, index: Option<usize>) -> Option<FileObjectCreation> {
    // Create the file info right at the start
    let mut file_info = FileInfo {
        dirname: match filename.parent() {
            Some(dirname) => dirname,
            None => return None,
        }
        .to_path_buf(),
        basename: match filename.file_name() {
            Some(basename) => basename,
            None => return None,
        }
        .to_owned(),
        modtime: None,
        modified: false,
    };

    // If the filename is a directory, we need to look for the underlying file, otherwise
    // we already have it
    let underlying_file = match filename.is_dir() {
        true => Path::join(&filename, FOLDER_METADATA_FILE_NAME),
        false => filename.to_path_buf(),
    };

    let (metadata_str, file_body) = match read_file_contents(&underlying_file) {
        Ok((metadata_str, file_body)) => (metadata_str, file_body),
        Err(err) => {
            if filename.is_dir() {
                ("".to_string(), "".to_string())
            } else {
                log::error!("Failed to read file {:?}: {:?}", &underlying_file, err);
                return None;
            }
        }
    };

    let mut metadata = FileObjectMetadata::default();

    let toml_header = metadata_str
        .parse::<DocumentMut>()
        .expect("invalid file metadata header");

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
        return None;
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
        Err(_) => {
            // The "correct" string is `worldbuilding`, but allow place anyway
            if file_type_str == "place" {
                FileType::Place
            } else {
                log::error!(
                    "Found unknown file type ({}) while attempt to read {:?}",
                    &file_type_str,
                    &filename
                );
                return None;
            }
        }
    };

    let mut base = BaseFileObject {
        metadata,
        index,
        file: file_info,
        toml_header,
        children: Vec::new(),
    };

    // Will eventually return this and all children
    let mut objects: FileObjectStore = HashMap::new();

    // Load children of this file object
    if file_type.is_folder() {
        if filename.is_dir() {
            match std::fs::read_dir(&filename) {
                Ok(files) => {
                    // TODO: Proper logic:
                    // 1. Read all of the files (that we can) into a list, it's fine to skip errors
                    // 2. Store all indexes that exist
                    // 3. Find the max of those indexes, assign (increasing) indexes to all remaining files
                    // 4. Call fix_indexing (which might end up being a non-member function because of ownership stuff)
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
                                            return None;
                                        }
                                    },
                                    None => {
                                        log::error!(
                                            "Encountered file without valid unicode name: {file:?}"
                                        );
                                        return None;
                                    }
                                };

                                // fuck, this is going to be even more complicated once indexing
                                // is done properly, it'll require multiple passes
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
                        if let Some(created_files) = from_file(&file, Some(index)) {
                            let (object, mut descendents): (Box<dyn FileObject>, FileObjectStore) =
                                match created_files {
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
                    }
                }
                Err(err) => {
                    log::error!(
                        "Error while attempt to read folder {:?}: {}",
                        &filename,
                        &err
                    )
                }
            }
        } else {
            log::error!(
                "attempted to construct a folder-type from a non-folder filename {:?}",
                &filename
            );
            return None;
        }

        // This will ensure that all children have the correct indexing. The only file objects
        // that aren't the children of some folder are the roots, which don't have indexing anyway
        fix_indexing(&mut base.children, &mut objects);
    }

    Some(match file_type {
        FileType::Scene => {
            let mut scene = Scene::from_file_object(base);
            scene.load_body(file_body);
            FileObjectCreation::Scene(scene, objects)
        }
        FileType::Character => FileObjectCreation::Character(Character::from_base(base), objects),
        FileType::Folder => FileObjectCreation::Folder(Folder::from_base(base), objects),
        FileType::Place => FileObjectCreation::Place(Place::from_base(base), objects),
    })
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

    /// Sets the index to this file, doing the move if necessary
    fn set_index(&mut self, new_index: usize, objects: &mut FileObjectStore) -> Result<()> {
        self.get_base_mut().index = Some(new_index);

        self.set_filename(self.calculate_filename(), objects)
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
            basename.push(&self.extension());
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
        new_path: &Path,
        objects: &mut FileObjectStore,
    ) -> Result<()> {
        let old_path = self.get_path();

        self.get_base_mut().index = Some(new_index);
        let new_path = Path::join(new_path, self.calculate_filename());

        if new_path == old_path {
            // Nothing to do:
            log::warn!("tried to move {old_path:?} to itself, harmless but shouldn't happen");
            return Ok(());
        }

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
            let (child_id, mut child) = objects
                .remove_entry(child_id.as_str())
                .expect("set_filename needs to borrow a map with the children");

            child.process_path_update(self.get_path(), objects);

            objects.insert(child_id, child);
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
        let path = match self.is_folder() {
            true => Path::join(&base_path, FOLDER_METADATA_FILE_NAME),
            false => base_path,
        };
        path
    }

    /// When the parent changes path, updates this dirname and any other children
    fn process_path_update(&mut self, new_directory: PathBuf, objects: &mut FileObjectStore) {
        self.get_base_mut().file.dirname = new_directory;

        // Propogate this to any children
        for child_id in self.get_base().children.iter() {
            let (child_id, mut child) = objects
                .remove_entry(child_id.as_str())
                .expect("process_path_update needs to borrow a map with the children");

            child.process_path_update(self.get_path(), objects);

            objects.insert(child_id, child);
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
            let (child_id_removed, mut child) = objects
                .remove_entry(child_id.as_str())
                .expect("process_path_update needs to borrow a map with the children");

            if let Err(err) = child.save(objects) {
                errors.push(err);
            }

            objects.insert(child_id_removed, child);
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

        let new_modtime = std::fs::metadata(&self.get_file())
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

    /// Creates a child in this folder, returning it to be added to the list
    fn create_child(&mut self, file_type: FileType) -> Result<Box<dyn FileObject>> {
        assert!(self.is_folder());

        // TODO: add a check to ensure that we don't have any indexing gaps when running this
        // maybe a bool somewhere that gets set in `create_indexing_gap` and removed in `fix_indexing`
        let new_index = self.get_base().children.len();

        let new_object: Box<dyn FileObject> = match file_type {
            FileType::Scene => Box::new(Scene::new(self.get_path(), new_index)?),
            FileType::Character => Box::new(Character::new(self.get_path(), new_index)?),
            FileType::Folder => Box::new(Folder::new(self.get_path(), new_index)?),
            FileType::Place => Box::new(Place::new(self.get_path(), new_index)?),
        };

        self.get_base_mut()
            .children
            .push(new_object.get_base().metadata.id.clone());

        Ok(new_object)
    }

    /// Allow for downcasting this as a reference, useful for creating the editors
    #[allow(dead_code)]
    fn get_file_type(&self) -> FileObjectTypeInterface;
    /// Allow for downcasting this as a mutable reference, useful for creating the editors
    fn get_file_type_mut(&mut self) -> MutFileObjectTypeInterface;
}
