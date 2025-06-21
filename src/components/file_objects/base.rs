use uuid::Uuid;

use crate::components::file_objects::utils::{
    add_index_to_name, process_name_for_filename, truncate_name,
};
use std::ffi::OsString;
use std::fs;
use std::io::{Error, ErrorKind};
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

impl FileType {
    fn extension(self) -> &'static str {
        match self {
            FileType::Scene => ".md",
            FileType::Folder => ".toml",
            FileType::Character => ".toml",
            FileType::Place => ".toml",
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
    modtime: Option<SystemTime>,
    modified: bool,
}

fn metadata_extract_u32(table: &mut Table, field_name: &str) -> std::io::Result<Option<u32>> {
    Ok(match table.remove(field_name) {
        Some(value) => Some(
            value
                .as_integer()
                .ok_or_else(|| Error::new(ErrorKind::InvalidData, "value was non-integer"))?
                .try_into()
                .map_err(|_| Error::new(ErrorKind::InvalidData, "failed to convert to u32"))?,
        ),
        None => None,
    })
}

fn metadata_extract_string(table: &mut Table, field_name: &str) -> std::io::Result<Option<String>> {
    Ok(match table.remove(field_name) {
        Some(value) => Some(
            value
                .as_str()
                .ok_or_else(|| Error::new(ErrorKind::InvalidData, "value was not string"))?
                .to_owned(),
        ),
        None => None,
    })
}

fn metadata_extract_bool(table: &mut Table, field_name: &str) -> std::io::Result<Option<bool>> {
    Ok(match table.remove(field_name) {
        Some(value) => Some(
            value
                .as_bool()
                .ok_or_else(|| Error::new(ErrorKind::InvalidData, "value was not string"))?,
        ),
        None => None,
    })
}

#[derive(Debug)]
pub struct FileObject {
    file_type: FileType,
    metadata: FileObjectMetadata,
    /// Index (ordering within parent)
    index: u32,
    /// Object ID of the parent
    parent_id: Option<String>,
    file: FileInfo,
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
            parent_id: parent,
            file: FileInfo {
                dirname,
                basename,
                modtime: None,
                modified: false,
            },
        }
    }

    /// Change the filename in the base object and on disk, processing any required updates
    fn set_filename(&mut self, new_filename: OsString) -> std::io::Result<()> {
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
            base_path.push(self.file_type.extension());
        }

        base_path
    }

    /// Sets the index to this file, doing the move if necessary
    pub fn set_index(&mut self, new_index: u32) -> std::io::Result<()> {
        self.index = new_index;

        self.set_filename(self.calculate_filename())
    }

    /// Recalculates the filename from the object property
    ///
    /// Unlike with `set_index`, we expect the underlying values to be borrowed directly,
    /// rather than having a callback with our updated value.
    pub fn set_filename_from_name(&mut self) -> std::io::Result<()> {
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
                let extension = &self.file_type.extension();
                let underlying_file_name = format!("{FOLDER_METADATA_FILE_NAME}{extension}");
                Path::join(&base_path, underlying_file_name)
            }
            false => base_path,
        };
        path
    }

    /// Load a file from disk. Will eventually become private, but useful for testing now
    pub fn load_file(&mut self) -> std::io::Result<()> {
        let file_to_read = self.get_file();

        // Determine if we want to read this file
        let current_modtime = fs::metadata(&file_to_read)
            .expect("attempted to load file that does not exist")
            .modified()
            .expect("Modtime not available");

        if self.file.modtime.is_some() {
            let old_modtime = self.file.modtime.unwrap();
            if old_modtime == current_modtime {
                // We've already loaded the latest revision, nothing to do
                return Ok(());
            }
        }

        // We want to read the file
        let file_data = fs::read_to_string(&file_to_read).expect("could not read file");

        let (metadata_str, file_content): (&str, &str) = match self.file_type.extension() == ".md" {
            false => (&file_data, ""),
            true => match file_data.split_once(HEADER_SPLIT) {
                None => ("", &file_data),
                Some((start, end)) => (start, end),
            },
        };

        println!("metadata: {metadata_str}");
        println!("contents: {file_content}");

        // TODO: Parse metadata
        // TODO: Store file_content if necessary

        Ok(())
    }
}

pub trait FileObjectType {
    fn save(&mut self, dest_path: &Path);
    fn load_from_disk(&mut self, source_path: &Path);
}
