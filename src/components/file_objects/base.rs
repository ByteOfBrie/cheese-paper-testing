use crate::components::file_objects::utils::{
    calculate_filename_for_object, process_name_for_filename, truncate_name,
};
use std::fs::File;
use std::path::{Path, PathBuf};

/// filename of the object within a folder containing its metadata (without extension)
const FOLDER_METADATA_FILE_NAME: &str = "metadata";

// pub fn get_object_path_from_parent(name: &str, index: u32, parent: Box<dyn FileObject>) -> PathBuf {
// }

// Should use some underlying structure to keep track of when these are changed and any values that
// we don't understand to write back to disk
pub struct FileObjectMetadata {
    /// Version of the object, can eventually be used to detect compatibility changes
    version: u32,
    /// Name of the object (e.g., title of a scene, character name)
    name: String,
    /// ID unique across all objects, probably UUIDv4 (but any string is acceptable)
    id: String,
}

/// List of known file types in this version of the editor. File types that aren't known will not
/// be read in
pub enum FileType {
    Scene,
    Folder,
    Character,
    Place,
}

fn file_type_extension(file_type: &FileType) -> &'static str {
    match file_type {
        FileType::Scene => ".md",
        FileType::Folder => ".toml",
        FileType::Character => ".toml",
        FileType::Place => ".toml",
    }
}

fn file_type_is_folder(file_type: &FileType) -> bool {
    match file_type {
        FileType::Scene => false,
        FileType::Folder => true,
        FileType::Character => false,
        FileType::Place => true,
    }
}

pub struct FileInfo {
    /// Path of the directory containing this file
    /// `/foo/bar/` -> `/foo`
    dirname: PathBuf,
    /// Path of the file within the dirname
    /// `/foo/bar/` -> `bar`
    basename: PathBuf,
    file_type: FileType,
}

pub struct FileObjectBase {
    metadata: FileObjectMetadata,
    index: u32,
    parent: Option<Box<dyn FileObject>>,
    file: FileInfo,
}

impl FileObjectBase {
    /// Change the filename in the base object and on disk, processing any required updates
    fn set_filename(&mut self, new_filename: &Path) -> std::io::Result<()> {
        let old_path = self.get_path();
        let new_path = Path::join(&self.file.dirname, new_filename);

        if new_path != old_path {
            std::fs::rename(old_path, new_path)?;
            self.file.basename = new_filename.to_path_buf();
        }
        Ok(())
    }

    /// Calculates the filename for a particular object
    fn calculate_filename(&self) -> PathBuf {
        PathBuf::from(calculate_filename_for_object(
            &self.metadata.name,
            file_type_extension(&self.file.file_type),
            self.index,
        ))
    }

    /// Sets the index to this file, doing the move if necessary
    pub fn set_index(&mut self, new_index: u32) -> std::io::Result<()> {
        self.index = new_index;

        self.set_filename(&self.calculate_filename())
    }

    /// Recalculates the filename from the object property
    ///
    /// Unlike with `set_index`, we expect the underlying values to be borrowed directly,
    /// rather than having a callback with our updated value.
    pub fn set_filename_from_name(&mut self) -> std::io::Result<()> {
        self.set_filename(&self.calculate_filename())
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
        let path = match file_type_is_folder(&self.file.file_type) {
            true => {
                let extension = file_type_extension(&self.file.file_type);
                let underlying_file_name = format!("{FOLDER_METADATA_FILE_NAME}{extension}");
                Path::join(&base_path, underlying_file_name)
            }
            false => base_path,
        };
        path
    }
}

pub trait FileObject {}

pub trait FileObjectType {
    fn save(&mut self, dest_path: &Path);
    fn load_from_disk(&mut self, source_path: &Path);
}
