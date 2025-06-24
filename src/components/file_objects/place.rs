use crate::components::file_objects::base::{FileObjectType, metadata_extract_string};
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
    metadata: PlaceMetadata,
}

impl Default for Place {
    fn default() -> Self {
        Self {
            metadata: Default::default(),
        }
    }
}

impl FileObjectType for Place {
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

    fn load_extra_data(&mut self, _data: String) {}
}
