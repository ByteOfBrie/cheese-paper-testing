use crate::components::file_objects::{FileInfo, FileObject, FileObjectMetadata, Folder};
use std::path::Path;
use std::path::PathBuf;

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

impl Project {
    /// Create a new project
    fn new(dirname: PathBuf, project_name: String) -> Self {
        unimplemented!()
    }

    /// Load an existing project from disk
    fn load(path: PathBuf) -> Self {
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
        path.push("project.toml");

        path
    }
}
