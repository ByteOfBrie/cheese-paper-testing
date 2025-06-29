use crate::components::file_objects::base::{
    ActualFileObject, BaseFileObject, metadata_extract_string,
};
use toml::Table;

#[derive(Debug)]
struct PlaceMetadata {
    connection: String,
    description: String,
    appearance: String,
    other_senses: String,
    notes: String,
}

impl Default for PlaceMetadata {
    fn default() -> Self {
        Self {
            connection: String::new(),
            description: String::new(),
            appearance: String::new(),
            other_senses: String::new(),
            notes: String::new(),
        }
    }
}

#[derive(Debug)]
pub struct Place {
    base: BaseFileObject,
    metadata: PlaceMetadata,
}

impl Place {
    pub fn new(base: BaseFileObject) -> Self {
        Self {
            base,
            metadata: Default::default(),
        }
    }
}

impl ActualFileObject for Place {
    fn load_metadata(&mut self, table: &mut Table) -> std::io::Result<bool> {
        let mut modified = false;

        match metadata_extract_string(table, "connection")? {
            Some(connection) => self.metadata.connection = connection,
            None => modified = true,
        }

        match metadata_extract_string(table, "description")? {
            Some(description) => self.metadata.description = description,
            None => modified = true,
        }

        match metadata_extract_string(table, "appearance")? {
            Some(appearance) => self.metadata.appearance = appearance,
            None => modified = true,
        }

        match metadata_extract_string(table, "other_senses")? {
            Some(other_senses) => self.metadata.other_senses = other_senses,
            None => modified = true,
        }

        match metadata_extract_string(table, "notes")? {
            Some(notes) => self.metadata.notes = notes,
            None => modified = true,
        }

        Ok(modified)
    }

    fn is_folder(&self) -> bool {
        true
    }

    fn extension(&self) -> &'static str {
        "toml"
    }

    fn empty_string_name(&self) -> &'static str {
        "New Place"
    }

    fn load_body(&mut self, _data: String) {}

    fn get_base(&self) -> &BaseFileObject {
        &self.base
    }

    fn get_base_mut(&mut self) -> &mut BaseFileObject {
        &mut self.base
    }
}
