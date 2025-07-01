use log::warn;
use std::collections::HashMap;
use std::fs::create_dir;
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
    version: u32,
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
    pub index: u32,
    /// Object ID of the parent
    pub parent: Option<String>,
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
            _ => Err("Unknown error type"),
        }
    }
}

impl FileType {
    fn extension(self) -> &'static str {
        match self {
            FileType::Scene => "md",
            FileType::Folder => "toml",
            FileType::Character => "toml",
            FileType::Place => "toml",
        }
    }

    fn is_folder(self) -> bool {
        match self {
            FileType::Scene => false,
            FileType::Folder => true,
            FileType::Character => false,
            FileType::Place => true,
        }
    }
}

fn empty_string_name(file_type: FileType) -> String {
    format!("new {}", Into::<&str>::into(file_type))
}

#[derive(Debug)]
pub struct FileInfo {
    /// Path of the directory containing this file
    /// `/foo/bar/` -> `/foo`
    dirname: PathBuf,
    /// Path of the file within the dirname
    /// `/foo/bar/` -> `bar`
    basename: OsString,
    /// Modified time if the file exists
    modtime: Option<SystemTime>,
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
fn load_base_metadata(
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

/// For ease of calling, `objects`` can contain arbitrary objects, only values contained
/// in `children` will actually be sorted.
fn fix_indexing(
    children: &mut Vec<String>,
    objects: &mut HashMap<String, Box<dyn FileObject>>,
) -> u32 {
    for (count, child_id) in children.iter().enumerate() {
        let (child_id, mut child) = objects
            .remove_entry(child_id.as_str())
            .expect("fix_indexing needs to borrow a map with the children");

        let child_base = child.get_base();

        if child_base.index
            != count
                .try_into()
                .expect("u32 should be massive overkill for indexes")
        {
            if let Err(err) = child.set_index(
                count.try_into().expect("should be able to convert u32"),
                objects,
            ) {
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

    children
        .len()
        .try_into()
        .expect("should be able to convert to u32")
}

/// Load an arbitrary file object from a file on disk
pub fn from_file(
    filename: &Path,
    index: u32,
    parent: Option<String>,
) -> Option<HashMap<String, Box<dyn FileObject>>> {
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
        Err(_) => {
            if filename.is_dir() {
                ("".to_string(), "".to_string())
            } else {
                log::error!("Failed to read file {:?}", &underlying_file);
                return None;
            }
        }
    };

    let mut metadata = FileObjectMetadata::default();

    let toml_header = metadata_str
        .parse::<DocumentMut>()
        .expect("invalid file metadata header");

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
            log::error!(
                "Found unknown file type ({}) while attempt to read {:?}",
                &file_type_str,
                &filename
            );
            return None;
        }
    };

    let mut base = BaseFileObject {
        metadata,
        index,
        parent,
        file: file_info,
        toml_header,
        children: Vec::new(),
    };

    // Will eventually return this and all children
    // TODO: should maybe convert to <&str, Self> and borrow the metadata.id, instead
    // of cloning it
    let mut objects: HashMap<String, Box<dyn FileObject>> = HashMap::new();

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
                    for file in files {
                        match file {
                            Ok(file) => {
                                println!("{:?}", file.path().file_name());
                                if file.path().file_name()
                                    == Some(&OsString::from(FOLDER_METADATA_FILE_NAME))
                                {
                                    continue;
                                }

                                // fuck, this is going to be even more complicated once indexing
                                // is done properly, it'll require multiple passes
                                let index = get_index_from_name(
                                    file.path().file_name().unwrap().to_str().unwrap(),
                                )
                                .unwrap_or(0);

                                if let Some(files) =
                                    from_file(&file.path(), index, Some(base.metadata.id.clone()))
                                {
                                    for (child_file_id, child_file) in files {
                                        base.children.push(child_file_id.clone());
                                        objects.insert(child_file_id, child_file);
                                    }
                                }
                            }
                            Err(err) => {
                                warn!("Could not read file in folder {:?}: {}", &filename, &err)
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
            )
        }

        // This will ensure that all children have the correct indexing. The only file objects
        // that aren't the children of some folder are the roots, which don't have indexing anyway
        fix_indexing(&mut base.children, &mut objects);
    }

    let mut underlying_obj: Box<dyn FileObject> = match file_type {
        FileType::Scene => Box::new(Scene::new(base)),
        FileType::Character => Box::new(Character::new(base)),
        FileType::Folder => Box::new(Folder::new(base)),
        FileType::Place => Box::new(Place::new(base)),
    };

    underlying_obj.load_body(file_body);

    objects.insert(
        underlying_obj.get_base().metadata.id.clone(),
        underlying_obj,
    );

    Some(objects)
}

impl BaseFileObject {
    /// Create a new file object in a folder
    pub fn new(file_type: FileType, dirname: PathBuf, index: u32, parent: Option<String>) -> Self {
        let name = empty_string_name(file_type);

        let name = truncate_name(&name, FILENAME_MAX_LENGTH);
        let name = process_name_for_filename(name);
        let name = add_index_to_name(&name, index);

        let mut base_path = OsString::from(name);

        if !file_type.is_folder() {
            base_path.push(".");
            base_path.push(file_type.extension());
        }

        Self {
            metadata: FileObjectMetadata::default(),
            index,
            parent,
            file: FileInfo {
                dirname,
                basename: base_path,
                modtime: None,
                modified: true, // Newly added files are modified (since they don't exist on disk)
            },
            toml_header: DocumentMut::new(),
            children: Vec::new(),
        }

        // TODO: when saving is implemented, save on creation (so that it can be used in other things)
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

    fn load_body(&mut self, body: String);

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
    fn set_index(
        &mut self,
        new_index: u32,
        objects: &mut HashMap<String, Box<dyn FileObject>>,
    ) -> Result<()> {
        self.get_base_mut().index = new_index;

        self.set_filename(self.calculate_filename(), objects)
    }

    /// Recalculates the filename from the object property
    ///
    /// Unlike with `set_index`, we expect the underlying values to be borrowed directly,
    /// rather than having a callback with our updated value.
    fn set_filename_from_name(
        &mut self,
        objects: &mut HashMap<String, Box<dyn FileObject>>,
    ) -> Result<()> {
        self.set_filename(self.calculate_filename(), objects)
    }

    /// Calculates the filename for a particular object
    fn calculate_filename(&self) -> OsString {
        let base_name: &str = match self.get_base().metadata.name.is_empty() {
            false => &self.get_base().metadata.name,
            true => self.empty_string_name(),
        };

        let truncated_name = truncate_name(base_name, FILENAME_MAX_LENGTH);
        let file_safe_name = process_name_for_filename(truncated_name);
        let final_name = add_index_to_name(&file_safe_name, self.get_base().index);

        let mut filename = OsString::from(final_name);

        if self.is_folder() {
            filename.push(".");
            filename.push(&self.extension());
        }

        filename
    }

    /// Change the filename in the base object and on disk, processing any required updates
    fn set_filename(
        &mut self,
        new_filename: OsString,
        objects: &mut HashMap<String, Box<dyn FileObject>>,
    ) -> Result<()> {
        let old_path = self.get_path();
        let new_path = Path::join(&self.get_base().file.dirname, &new_filename);

        if new_path != old_path {
            std::fs::rename(old_path, &new_path)?;
            self.get_base_mut().file.basename = new_filename;
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
    fn process_path_update(
        &mut self,
        new_directory: PathBuf,
        objects: &mut HashMap<String, Box<dyn FileObject>>,
    ) {
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

    fn get_file_type(&self) -> FileObjectTypeInterface;
    fn get_file_type_mut(&mut self) -> MutFileObjectTypeInterface;
}
