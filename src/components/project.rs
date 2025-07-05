use crate::components::file_objects::{
    FileInfo, FileObject, FileObjectMetadata, FileObjectStore, Folder, from_file,
    write_with_temp_file,
};
use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{Error, ErrorKind, Result};
use std::path::Path;
use std::path::PathBuf;
use toml_edit::DocumentMut;

use crate::components::file_objects::utils::process_name_for_filename;

use crate::components::file_objects::base::{
    FileObjectCreation, load_base_metadata, metadata_extract_string,
};

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
    objects: FileObjectStore,
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

fn load_top_level_folder(folder_path: &Path, name: String) -> Result<(Folder, FileObjectStore)> {
    if folder_path.exists() {
        match from_file(&folder_path, None) {
            Some(created_object) => match created_object {
                FileObjectCreation::Folder(folder, contents) => Ok((folder, contents)),
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "could not load text for unknown reason",
                    ));
                }
            },
            None => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "could not load text for unknown reason",
                ));
            }
        }
    } else {
        Ok((
            Folder::new_top_level(folder_path.to_owned(), name)?,
            HashMap::new(),
        ))
    }
}

impl Project {
    /// Create a new project
    pub fn new(dirname: PathBuf, project_name: String) -> Result<Self> {
        // Not truncating here (for now)
        let file_safe_name = process_name_for_filename(&project_name);
        let project_path = dirname.join(&file_safe_name);

        if project_path.exists() {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                format!("attempted to initialize {project_path:?}, which already exists"),
            ));
        } else {
            std::fs::create_dir(&project_path)?;
        }

        Ok(Self {
            base_metadata: FileObjectMetadata {
                name: project_name,
                ..Default::default()
            },
            metadata: ProjectMetadata::default(),
            text: Folder::new_top_level(project_path.clone(), "text".to_owned())?,
            characters: Folder::new_top_level(project_path.clone(), "characters".to_owned())?,
            worldbuilding: Folder::new_top_level(project_path.clone(), "worldbuilding".to_owned())?,
            file: FileInfo {
                dirname,
                basename: OsString::from(file_safe_name),
                modtime: None,
                modified: true, // Newly added files are modified (they don't exist on disk)
            },
            toml_header: DocumentMut::new(),
            objects: HashMap::new(),
        })
    }

    /// Load an existing project from disk
    pub fn load(path: PathBuf) -> Result<Self> {
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
        let metadata = ProjectMetadata::default();

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
        let (text, mut descendents) =
            load_top_level_folder(&Path::join(&path, "text"), "text".to_string())?;

        let (characters, characters_descendents) =
            load_top_level_folder(&Path::join(&path, "characters"), "characters".to_string())?;

        let (worldbuilding, worldbuilding_descendents) = load_top_level_folder(
            &Path::join(&path, "worldbuilding"),
            "worldbuilding".to_string(),
        )?;

        // merge all of the descendents into a single hashmap that owns all of them
        descendents.extend(characters_descendents);
        descendents.extend(worldbuilding_descendents);

        load_base_metadata(&toml_header, &mut base_metadata, &mut file_info)?;

        let mut project = Self {
            metadata,
            base_metadata,
            file: file_info,
            text,
            characters,
            worldbuilding,
            toml_header,
            objects: descendents,
        };

        project.load_metadata()?;

        project.save()?;

        Ok(project)
    }

    pub fn save(&mut self) -> Result<()> {
        if !self.file.modified {
            return Ok(());
        }

        // unlike other file objects, this one doesn't rename automatically. This might be something
        // I want to add later, but it's currently intentional

        self.text.save(&mut self.objects)?;
        self.characters.save(&mut self.objects)?;
        self.worldbuilding.save(&mut self.objects)?;

        self.write_metadata();

        let final_str = self.toml_header.to_string();

        write_with_temp_file(&self.get_project_info_file(), final_str.as_bytes())?;

        let new_modtime = std::fs::metadata(&self.get_project_info_file())
            .expect("attempted to load file that does not exist")
            .modified()
            .expect("Modtime not available");

        // Update modtime based on what we just wrote
        self.file.modtime = Some(new_modtime);

        Ok(())
    }

    fn write_metadata(&mut self) {
        self.toml_header["version"] = toml_edit::value(self.base_metadata.version as i64);
        self.toml_header["name"] = toml_edit::value(&self.base_metadata.name);
        self.toml_header["id"] = toml_edit::value(&self.base_metadata.id);

        self.toml_header["summary"] = toml_edit::value(&self.metadata.summary);
        self.toml_header["notes"] = toml_edit::value(&self.metadata.notes);
        self.toml_header["genre"] = toml_edit::value(&self.metadata.genre);
        self.toml_header["author"] = toml_edit::value(&self.metadata.author);
        self.toml_header["email"] = toml_edit::value(&self.metadata.email);
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
