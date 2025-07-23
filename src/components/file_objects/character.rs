use crate::components::file_objects::base::{BaseFileObject, FileObject, metadata_extract_string};
use std::io::Result;
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug)]
pub struct CharacterMetadata {
    pub summary: String,
    pub notes: String,
    pub appearance: String,
    pub personality: String,
    pub goal: String,
    pub conflict: String,
    pub habits: String,
}

impl Default for CharacterMetadata {
    fn default() -> Self {
        Self {
            summary: String::new(),
            notes: String::new(),
            appearance: String::new(),
            personality: String::new(),
            goal: String::new(),
            conflict: String::new(),
            habits: String::new(),
        }
    }
}

#[derive(Debug)]
pub struct Character {
    pub base: BaseFileObject,
    pub metadata: CharacterMetadata,
}

impl Character {
    pub fn new(dirname: PathBuf, index: usize) -> Result<Self> {
        let mut character = Self {
            base: BaseFileObject::new(dirname, Some(index)),
            metadata: CharacterMetadata::default(),
        };

        character.base.file.basename = character.calculate_filename();

        character.save(&mut HashMap::new())?;

        Ok(character)
    }

    pub fn from_base(base: BaseFileObject) -> Result<Self> {
        let mut character = Self {
            base,
            metadata: Default::default(),
        };

        match character.load_metadata() {
            Ok(modified) => {
                if modified {
                    character.base.file.modified = true;
                }
            }
            Err(err) => {
                log::error!(
                    "Error while loading object-specific metadata for {:?}: {}",
                    character.get_path(),
                    &err
                );
                return Err(err);
            }
        }

        Ok(character)
    }
}

impl FileObject for Character {
    fn load_metadata(&mut self) -> std::io::Result<bool> {
        let mut modified = false;

        match metadata_extract_string(&self.base.toml_header, "summary")? {
            Some(summary) => self.metadata.summary = summary,
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "notes")? {
            Some(notes) => self.metadata.notes = notes,
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "appearance")? {
            Some(appearance) => self.metadata.appearance = appearance,
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "personality")? {
            Some(personality) => self.metadata.personality = personality,
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "goal")? {
            Some(goal) => self.metadata.goal = goal,
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "conflict")? {
            Some(conflict) => self.metadata.conflict = conflict,
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "habits")? {
            Some(habits) => self.metadata.habits = habits,
            None => modified = true,
        }

        Ok(modified)
    }

    fn is_folder(&self) -> bool {
        false
    }

    fn has_body(&self) -> bool {
        false
    }

    fn extension(&self) -> &'static str {
        "toml"
    }

    fn empty_string_name(&self) -> &'static str {
        "New Character"
    }

    fn load_body(&mut self, _data: String) {}
    fn get_body(&self) -> String {
        String::new()
    }

    fn get_base(&self) -> &BaseFileObject {
        &self.base
    }

    fn get_base_mut(&mut self) -> &mut BaseFileObject {
        &mut self.base
    }

    fn get_file_type(&self) -> super::FileObjectTypeInterface<'_> {
        super::FileObjectTypeInterface::Character(self)
    }

    fn get_file_type_mut(&mut self) -> super::MutFileObjectTypeInterface<'_> {
        super::MutFileObjectTypeInterface::Character(self)
    }

    fn write_metadata(&mut self) {
        self.base.toml_header["file_type"] = toml_edit::value("character");
        self.base.toml_header["summary"] = toml_edit::value(&self.metadata.summary);
        self.base.toml_header["notes"] = toml_edit::value(&self.metadata.notes);
        self.base.toml_header["appearance"] = toml_edit::value(&self.metadata.appearance);
        self.base.toml_header["personality"] = toml_edit::value(&self.metadata.personality);
        self.base.toml_header["goal"] = toml_edit::value(&self.metadata.goal);
        self.base.toml_header["conflict"] = toml_edit::value(&self.metadata.conflict);
        self.base.toml_header["habits"] = toml_edit::value(&self.metadata.habits);
    }
}
