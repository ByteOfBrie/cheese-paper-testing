use uuid::Uuid;

use crate::components::file_objects::utils::{
    add_index_to_name, process_name_for_filename, truncate_name,
};
use crate::components::file_objects::{Character, Folder, Place, Scene};
use std::ffi::OsString;
use std::fmt::Debug;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use toml::Table;

/// the maximum length of a name before we start trying to truncate it
const FILENAME_MAX_LENGTH: usize = 30;

/// filename of the object within a folder containing its metadata (without extension)
const FOLDER_METADATA_FILE_NAME: &str = "metadata";

/// Value that splits the header of any file that contains non-metadata content
const HEADER_SPLIT: &str = "++++++++";

// pub fn get_object_path_from_parent(name: &str, index: u32, parent: Box<dyn FileObject>) -> PathBuf {
// }

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
    version: u32,
    /// Name of the object (e.g., title of a scene, character name)
    name: String,
    /// ID unique across all objects. The reference implementations use UUIDv4, but any string
    /// is acceptable
    id: String,
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

/// List of known file types in this version of the editor. File types that aren't known will not
/// be read in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Scene,
    Folder,
    Character,
    Place,
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
    modified: bool,
}

pub fn metadata_extract_u32(table: &mut Table, field_name: &str) -> Result<Option<u32>> {
    Ok(match table.remove(field_name) {
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

pub fn metadata_extract_string(table: &mut Table, field_name: &str) -> Result<Option<String>> {
    Ok(match table.remove(field_name) {
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

pub fn metadata_extract_bool(table: &mut Table, field_name: &str) -> Result<Option<bool>> {
    Ok(match table.remove(field_name) {
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

    Ok((metadata_str.to_owned(), file_content.to_owned()))
}

/// Given a freshly read metadata dictionary, read it into the file objects, setting modified as
/// appropriate
fn load_metadata(
    metadata_table: &mut toml::map::Map<String, toml::Value>,
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

#[derive(Debug)]
pub struct FileObject {
    file_type: FileType,
    metadata: FileObjectMetadata,
    /// Index (ordering within parent)
    index: u32,
    /// Object ID of the parent
    parent: Option<String>,
    file: FileInfo,
    child: Box<dyn FileObjectType>,
    extra_metadata: Table,
}

impl FileObject {
    // TODO: figure out what this actually does (creation vs loading)
    /// create new file object at path
    pub fn new(
        file_type: FileType,
        dirname: PathBuf,
        basename: OsString,
        index: u32,
        parent: Option<String>,
    ) -> Self {
        Self {
            file_type,
            metadata: FileObjectMetadata::default(),
            index,
            parent,
            file: FileInfo {
                dirname,
                basename,
                modtime: None,
                modified: false,
            },
            child: match file_type {
                FileType::Scene => Box::new(Scene::default()),
                FileType::Character => Box::new(Character::default()),
                FileType::Folder => Box::new(Folder::default()),
                FileType::Place => Box::new(Place::default()),
            },
            extra_metadata: Table::new(),
        }
    }

    /// Load an arbitrary file object from a file on disk
    pub fn from_file(mut filename: PathBuf, index: u32, parent: Option<String>) -> Option<Self> {
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
        if filename.is_dir() {
            filename.push(FOLDER_METADATA_FILE_NAME);
            filename.set_extension(FileType::Folder.extension());
        }

        let (metadata_str, file_body) = match read_file_contents(&filename) {
            Ok((metadata_str, file_body)) => (metadata_str, file_body),
            Err(_) => {
                log::error!("Failed to read file {:?}", &filename);
                return None;
            }
        };

        let mut metadata = FileObjectMetadata::default();

        let mut file_metadata_contents = metadata_str.parse::<Table>().unwrap();

        if let Err(err) = load_metadata(&mut file_metadata_contents, &mut metadata, &mut file_info)
        {
            log::error!("Error while parsing metadata for {:?}: {}", &filename, &err);
            return None;
        }

        let file_type_str = match file_metadata_contents.remove("file_type") {
            Some(val) => val.as_str().unwrap_or("unknown").to_owned(),
            None => "unknown".to_string(), // TODO: actually write logic here
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

        let mut child: Box<dyn FileObjectType> = match file_type {
            FileType::Scene => Box::new(Scene::default()),
            FileType::Character => Box::new(Character::default()),
            FileType::Folder => Box::new(Folder::default()),
            FileType::Place => Box::new(Place::default()),
        };

        if let Err(err) = child.load_metadata(&mut file_metadata_contents) {
            log::error!(
                "Error while loading object-specific metadata for {:?}: {}",
                &filename,
                &err
            );
            return None;
        }
        child.load_extra_data(file_body);

        Some(Self {
            file_type,
            metadata,
            index,
            parent,
            file: file_info,
            child,
            extra_metadata: file_metadata_contents,
        })
    }

    /// Change the filename in the base object and on disk, processing any required updates
    fn set_filename(&mut self, new_filename: OsString) -> Result<()> {
        let old_path = self.get_path();
        let new_path = Path::join(&self.file.dirname, &new_filename);

        if new_path != old_path {
            std::fs::rename(old_path, new_path)?;
            self.file.basename = new_filename;
        }
        Ok(())
    }

    /// Calculates the filename for a particular object
    fn calculate_filename(&self) -> OsString {
        let name: &str = match self.metadata.name.is_empty() {
            false => &self.metadata.name,
            true => &empty_string_name(self.file_type),
        };

        let name = truncate_name(name, FILENAME_MAX_LENGTH);
        let name = process_name_for_filename(name);
        let name = add_index_to_name(&name, self.index);

        let mut base_path = OsString::from(name);

        if !self.file_type.is_folder() {
            base_path.push(".");
            base_path.push(self.file_type.extension());
        }

        base_path
    }

    /// Sets the index to this file, doing the move if necessary
    pub fn set_index(&mut self, new_index: u32) -> Result<()> {
        self.index = new_index;

        self.set_filename(self.calculate_filename())
    }

    /// Recalculates the filename from the object property
    ///
    /// Unlike with `set_index`, we expect the underlying values to be borrowed directly,
    /// rather than having a callback with our updated value.
    pub fn set_filename_from_name(&mut self) -> Result<()> {
        self.set_filename(self.calculate_filename())
    }

    /// Calculates the object's current path. For objects in a single file, this is their path
    /// (including the extension), for folder-based objects (i.e., Folder, Place), this is the
    /// path to the folder.
    ///
    /// Also see `get_file`
    pub fn get_path(&self) -> PathBuf {
        Path::join(&self.file.dirname, &self.file.basename)
    }

    /// The path to an object's underlying file, the equivalent of `get_path` when doing file
    /// operations on this object
    fn get_file(&self) -> PathBuf {
        let base_path = self.get_path();
        let path = match &self.file_type.is_folder() {
            true => {
                let extension = self.file_type.extension();
                let underlying_file_name = format!("{FOLDER_METADATA_FILE_NAME}{extension}");
                Path::join(&base_path, underlying_file_name)
            }
            false => base_path,
        };
        path
    }

    /// Reloads the contents of this file object from disk. Assumes that the file has been properly
    /// initialized already
    pub fn reload_file(&mut self) -> Result<()> {
        let file_to_read = self.get_file();

        if !self.should_load(&file_to_read)? {
            return Ok(());
        }

        let (metadata_str, file_body) = read_file_contents(&file_to_read)?;

        let mut file_metadata_contents = metadata_str.parse::<Table>().unwrap();

        load_metadata(
            &mut file_metadata_contents,
            &mut self.metadata,
            &mut self.file,
        )?;

        self.child.load_metadata(&mut file_metadata_contents)?;
        self.child.load_extra_data(file_body);

        self.extra_metadata = file_metadata_contents;

        Ok(())
    }

    /// Determine if the file should be loaded
    fn should_load(&mut self, file_to_read: &Path) -> Result<bool> {
        let current_modtime = std::fs::metadata(file_to_read)
            .expect("attempted to load file that does not exist")
            .modified()
            .expect("Modtime not available");

        if let Some(old_modtime) = self.file.modtime {
            if old_modtime == current_modtime {
                // We've already loaded the latest revision, nothing to do
                return Ok(false);
            }
        }

        Ok(true)
    }
}

pub trait FileObjectType: Debug {
    fn load_metadata(&mut self, table: &mut Table) -> Result<bool>;
    fn load_extra_data(&mut self, data: String);
}
