use crate::components::file_objects::base::{BaseFileObject, FileObject, metadata_extract_string};
use crate::components::file_objects::utils::write_outline_property;
use crate::components::project::ExportOptions;
use crate::components::text::Text;
use crate::util::CheeseError;
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Default)]
pub struct CharacterMetadata {
    pub summary: Text,
    pub notes: Text,
    pub appearance: Text,
    pub personality: Text,
    pub goal: Text,
    pub conflict: Text,
    pub habits: Text,
}

#[derive(Debug)]
pub struct Character {
    pub base: BaseFileObject,
    pub metadata: CharacterMetadata,
}

impl Character {
    pub fn new(dirname: PathBuf, index: usize) -> Result<Self, CheeseError> {
        let mut character = Self {
            base: BaseFileObject::new(dirname, Some(index)),
            metadata: CharacterMetadata::default(),
        };

        character.base.file.basename = character.calculate_filename();

        <dyn FileObject>::save(&mut character, &HashMap::new()).unwrap();

        Ok(character)
    }

    pub fn from_base(base: BaseFileObject) -> Result<Self, CheeseError> {
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
    fn load_metadata(&mut self) -> Result<bool, CheeseError> {
        let mut modified = false;

        match metadata_extract_string(&self.base.toml_header, "summary")? {
            Some(summary) => self.metadata.summary = summary.into(),
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "notes")? {
            Some(notes) => self.metadata.notes = notes.into(),
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "appearance")? {
            Some(appearance) => self.metadata.appearance = appearance.into(),
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "personality")? {
            Some(personality) => self.metadata.personality = personality.into(),
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "goal")? {
            Some(goal) => self.metadata.goal = goal.into(),
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "conflict")? {
            Some(conflict) => self.metadata.conflict = conflict.into(),
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "habits")? {
            Some(habits) => self.metadata.habits = habits.into(),
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
        self.base.toml_header["summary"] = toml_edit::value(&*self.metadata.summary);
        self.base.toml_header["notes"] = toml_edit::value(&*self.metadata.notes);
        self.base.toml_header["appearance"] = toml_edit::value(&*self.metadata.appearance);
        self.base.toml_header["personality"] = toml_edit::value(&*self.metadata.personality);
        self.base.toml_header["goal"] = toml_edit::value(&*self.metadata.goal);
        self.base.toml_header["conflict"] = toml_edit::value(&*self.metadata.conflict);
        self.base.toml_header["habits"] = toml_edit::value(&*self.metadata.habits);
    }

    fn generate_outline(
        &self,
        depth: u32,
        export_string: &mut String,
        _objects: &super::FileObjectStore,
    ) {
        (self as &dyn FileObject).write_title(depth, export_string);

        write_outline_property("summary", &self.metadata.summary, export_string);
        write_outline_property("appearance", &self.metadata.appearance, export_string);
        write_outline_property("personality", &self.metadata.personality, export_string);
        write_outline_property("goal", &self.metadata.goal, export_string);
        write_outline_property("conflict", &self.metadata.conflict, export_string);
        write_outline_property("habits", &self.metadata.habits, export_string);
        write_outline_property("notes", &self.metadata.notes, export_string);
    }

    /// Characters will not be included in the text export, nothing to do
    fn generate_export(
        &self,
        _depth: u32,
        _export_string: &mut String,
        _objects: &super::FileObjectStore,
        _export_options: &ExportOptions,
    ) {
        // it's fine to call this, we just don't do anything
    }

    fn as_editor(&self) -> &dyn crate::ui::FileObjectEditor {
        self
    }

    fn as_editor_mut(&mut self) -> &mut dyn crate::ui::FileObjectEditor {
        self
    }
}

// shortcuts for not having to cast every time

#[cfg(test)]
impl Character {
    pub fn save(&mut self, objects: &super::FileObjectStore) -> Result<(), CheeseError> {
        (self as &mut dyn FileObject).save(objects)
    }
}
