use crate::components::file_objects::base::{
    BaseFileObject, FileObject, FileType, metadata_extract_string,
};
use std::io::Result;
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug)]
pub struct PlaceMetadata {
    pub connection: String,
    pub description: String,
    pub appearance: String,
    pub other_senses: String,
    pub notes: String,
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
    pub base: BaseFileObject,
    pub metadata: PlaceMetadata,
}

impl Place {
    pub fn new(dirname: PathBuf, index: u32) -> Result<Self> {
        let mut place = Self {
            base: BaseFileObject::new(FileType::Place, dirname, index),
            metadata: PlaceMetadata::default(),
        };

        place.save(&mut HashMap::new())?;

        Ok(place)
    }

    pub fn from_base(base: BaseFileObject) -> Self {
        let mut place = Self {
            base,
            metadata: Default::default(),
        };

        match place.load_metadata() {
            Ok(modified) => {
                if modified {
                    place.base.file.modified = true;
                }
            }
            Err(err) => {
                log::error!(
                    "Error while loading object-specific metadata for {:?}: {}",
                    place.get_path(),
                    &err
                );
            }
        }

        place
    }
}

impl FileObject for Place {
    fn load_metadata(&mut self) -> std::io::Result<bool> {
        let mut modified = false;

        match metadata_extract_string(&self.base.toml_header, "connection")? {
            Some(connection) => self.metadata.connection = connection,
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "description")? {
            Some(description) => self.metadata.description = description,
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "appearance")? {
            Some(appearance) => self.metadata.appearance = appearance,
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "other_senses")? {
            Some(other_senses) => self.metadata.other_senses = other_senses,
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "notes")? {
            Some(notes) => self.metadata.notes = notes,
            None => modified = true,
        }

        Ok(modified)
    }

    fn is_folder(&self) -> bool {
        true
    }

    fn has_body(&self) -> bool {
        false
    }

    fn extension(&self) -> &'static str {
        "toml"
    }

    fn empty_string_name(&self) -> &'static str {
        "New Place"
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

    fn get_file_type(&self) -> super::FileObjectTypeInterface {
        super::FileObjectTypeInterface::Place(self)
    }

    fn get_file_type_mut(&mut self) -> super::MutFileObjectTypeInterface {
        super::MutFileObjectTypeInterface::Place(self)
    }

    fn write_metadata(&mut self) {
        self.base.toml_header["connection"] = toml_edit::value(&self.metadata.connection);
        self.base.toml_header["description"] = toml_edit::value(&self.metadata.description);
        self.base.toml_header["appearance"] = toml_edit::value(&self.metadata.appearance);
        self.base.toml_header["other_senses"] = toml_edit::value(&self.metadata.other_senses);
        self.base.toml_header["notes"] = toml_edit::value(&self.metadata.notes);
    }
}
