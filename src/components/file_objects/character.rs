use crate::components::file_objects::base::{
    ActualFileObject, BaseFileObject, metadata_extract_string,
};

#[derive(Debug)]
struct CharacterMetadata {
    summary: String,
    notes: String,
    appearance: String,
    personality: String,
    goal: String,
    conflict: String,
    habits: String,
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
    base: BaseFileObject,
    metadata: CharacterMetadata,
}

impl Character {
    pub fn new(base: BaseFileObject) -> Self {
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
            }
        }

        character
    }
}

impl ActualFileObject for Character {
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

    fn extension(&self) -> &'static str {
        "toml"
    }

    fn empty_string_name(&self) -> &'static str {
        "New Character"
    }

    fn load_body(&mut self, _data: String) {}

    fn get_base(&self) -> &BaseFileObject {
        &self.base
    }

    fn get_base_mut(&mut self) -> &mut BaseFileObject {
        &mut self.base
    }
}
