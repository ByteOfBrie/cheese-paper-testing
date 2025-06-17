use regex::Regex;
use serde::Deserialize;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
// in order to have defaults, there should be some null/empty value that can be set,
// which then means that default values will have to be implemented when using the
// config values in other places, not in the config itself. it's not super clean,
// but I don't see how I can realistically get default values that can be unset
// in the UI without implementing config reading by hand (or at least more manually
//
// maybe I should have a "config_file" and "config" concepts at different layers?
// that avoids potentially having a default defined in multiple places, but feels
// really ugly
//
// the hard problem here is that I have to write the values back, but I want to
// retain the ability to unset them

/// the maximum length of a name before we start trying to truncate it
const FILENAME_MAX_LENGTH: usize = 30;
/// filename of the object within a folder containing its metadata (without extension)
const FOLDER_METADATA_FILE_NAME: &str = "metadata";

// make sure the name fits within a specified length, trying to split on word boundaries
pub fn truncate_name(name: &str, max_length: usize) -> &str {
    let name = name.trim();
    // if the name is already short enough, we're done
    if name.len() <= max_length {
        return &name;
    }

    // split by word, go with increasingly fewer words
    let split_name: Vec<&str> = name.split(" ").collect();
    for number_of_words in (1..split_name.len()).rev() {
        // construct the split text into a temporary string
        let shortened = &split_name[..number_of_words].join(" ");
        if shortened.len() < max_length && shortened != "" {
            // return a slice of the actual name so it retains the original lifetime
            return &name[..shortened.len()];
        }
    }

    // we the first word is longer than `max_length`, give up on being smart
    &name[..max_length]
}

#[test]
fn test_truncate_name() {
    assert_eq!(truncate_name("Hello World", 30), "Hello World");
    assert_eq!(truncate_name("Hello World", 9), "Hello");
    assert_eq!(truncate_name("Hello World", 11), "Hello World");
    assert_eq!(truncate_name("Hello World", 5), "Hello");
    assert_eq!(truncate_name("Hello World", 4), "Hell");
    assert_eq!(truncate_name(" Hello World", 2), "He");
    assert_eq!(truncate_name("Hello World   ", 30), "Hello World");
}

/// Translates a name into something we can put on disk
pub fn process_name_for_filename(name: &str) -> String {
    // get rid of spaces in names for editing convenience
    let name = name.replace(" ", "_");
    let name = name.replace("'", "");

    // Characters that might be annoying to escape/handle sometimes, avoid including them at all
    let dangerous_character_filter = Regex::new(r#"[/\?%*:|"<>\x7F\x00-\x1F]"#).unwrap();
    dangerous_character_filter
        .replace_all(&name, "-")
        .into_owned()
}

#[test]
fn test_process_name_for_filename() {
    assert_eq!(process_name_for_filename(r"hello world"), "hello_world");
    assert_eq!(process_name_for_filename(r"possessive's"), "possessives");
    assert_eq!(process_name_for_filename(r"asdf?'?s"), "asdf--s");
}

/// Just adds an index to a name, no real logic
pub fn add_index_to_name(name: &str, index: u32) -> String {
    format!("{index:03}-{name}")
}

/// Default way to get the path of a file object
/// name: name that the file object has within the editor
pub fn calculate_filename_for_object(name: &str, extension: &str, index: u32) -> String {
    let name = truncate_name(name, FILENAME_MAX_LENGTH);
    let name = process_name_for_filename(name);
    let name = add_index_to_name(&name, index);
    format!("{name}{extension}")
}

#[test]
fn test_calculate_filename_for_object() {
    assert_eq!(
        calculate_filename_for_object("New Scene", ".md", 0),
        "000-New_Scene.md"
    );
    assert_eq!(
        calculate_filename_for_object("New Scene", ".md", 10),
        "010-New_Scene.md"
    );
}

// pub fn get_object_path_from_parent(name: &str, index: u32, parent: Box<dyn FileObject>) -> PathBuf {
// }

// Should use some underlying structure to keep track of when these are changed and any values that
// we don't understand to write back to disk
pub struct FileObjectMetadata {
    version: u32,
    name: String,
    id: String,
}

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
    dirname: PathBuf,
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
    fn calculate_filename(&self) -> String {
        calculate_filename_for_object(
            &self.metadata.name,
            file_type_extension(&self.file.file_type),
            self.index,
        )
    }

    pub fn set_index(&mut self, new_index: u32) -> std::io::Result<()> {
        self.index = new_index;

        let new_filename: PathBuf = PathBuf::from(self.calculate_filename());
        self.set_filename(&new_filename)
    }

    pub fn get_path(&self) -> PathBuf {
        Path::join(&self.file.dirname, &self.file.basename)
    }

    pub fn set_filename(&mut self, new_filename: &Path) -> std::io::Result<()> {
        let old_path = self.get_path();
        let new_path = Path::join(&self.file.dirname, new_filename);
        std::fs::rename(old_path, new_path)?;
        self.file.basename = new_filename.to_path_buf();
        Ok(())
    }

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
