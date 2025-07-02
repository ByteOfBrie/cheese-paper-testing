use crate::components::file_objects::{
    FileInfo, FileObjectMetadata, FileObjectTypeInterface, Folder, from_file,
};
use std::io::{Error, ErrorKind, Result};
use std::path::Path;
use std::path::PathBuf;
use toml_edit::DocumentMut;

use crate::components::file_objects::base::{load_base_metadata, metadata_extract_string};

/// An entire project. This is somewhat file_object like, but we don't implement everything,
/// so it's separate (for now)
#[derive(Debug)]
pub struct Project {
    pub metadata: ProjectMetadata,
    pub base_metadata: FileObjectMetadata,
    pub file: FileInfo,
    text: Folder,
    characters: Folder,
    worldbuilding: Folder,
    /// Whether the Project's files themselves have been modified, not related to children
    modified: bool,
    toml_header: DocumentMut,
}

#[derive(Debug)]
pub struct ProjectMetadata {
    summary: String,
    notes: String,
    genre: String,
    author: String,
    email: String,
}

impl Default for ProjectMetadata {
    fn default() -> Self {
        Self {
            summary: String::new(),
            notes: String::new(),
            genre: String::new(),
            author: String::new(),
            email: String::new(),
        }
    }
}

const PROJECT_INFO_NAME: &str = "project.toml";

impl Project {
    /// Create a new project
    fn new(dirname: PathBuf, project_name: String) -> Self {
        unimplemented!()
    }

    /// Load an existing project from disk
    fn load(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            return Err(Error::new(
                ErrorKind::NotADirectory,
                format!("attempted to load {path:?}, was not a directory"),
            ));
        }

        let mut file_info = FileInfo {
            dirname: match path.parent() {
                Some(dirname) => dirname,
                None => {
                    return Err(Error::new(
                        ErrorKind::InvalidFilename,
                        format!("no directory component in {path:?}"),
                    ));
                }
            }
            .to_path_buf(),
            basename: match path.file_name() {
                Some(basename) => basename,
                None => {
                    return Err(Error::new(
                        ErrorKind::InvalidFilename,
                        format!("no filename component in {path:?}"),
                    ));
                }
            }
            .to_owned(),
            modtime: None,
            modified: false,
        };

        let mut base_metadata = FileObjectMetadata::default();
        let mut metadata = ProjectMetadata::default();

        // Load project metadata
        let project_info_path = Path::join(&path, PROJECT_INFO_NAME);

        let toml_header = if project_info_path.exists() {
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
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "attempted to load {path:?}, did not \
                         contain {PROJECT_INFO_NAME} or text folder"
                    ),
                ));
            }
            DocumentMut::new()
        };

        // Load or create folders
        let text_path = Path::join(&path, "text");
        // index should maybe be an option here to rely more strongly on the type system
        let text = match from_file(&text_path, 0) {
            Some(created_object) => {
                match created_object.object.get_file_type() {
                    FileObjectTypeInterface::Folder(folder) => folder,
                    _ => {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            "could not load text for unknown reason",
                        ));
                    }
                };
                created_object
            }
            None => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "could not load text for unknown reason",
                ));
            }
        };

        load_base_metadata(&toml_header, &mut base_metadata, &mut file_info)?;

        unimplemented!()
    }

    fn save(&mut self) {
        unimplemented!()
        // self.text.save()
    }

    fn get_path(&self) -> PathBuf {
        Path::join(&self.file.dirname, &self.file.basename)
    }

    fn get_project_info_file(&self) -> PathBuf {
        let mut path = self.get_path();
        path.push(PROJECT_INFO_NAME);

        path
    }

    fn load_metadata(&mut self) -> std::io::Result<bool> {
        let mut modified = false;

        match metadata_extract_string(&self.toml_header, "summary")? {
            Some(summary) => self.metadata.summary = summary,
            None => modified = true,
        }

        match metadata_extract_string(&self.toml_header, "notes")? {
            Some(notes) => self.metadata.notes = notes,
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

        Ok(modified)
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

    fn reload_file(&mut self) -> Result<()> {
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
}
