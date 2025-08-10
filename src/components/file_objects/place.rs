use crate::components::file_objects::base::{BaseFileObject, FileObject, metadata_extract_string};
use crate::components::text::Text;
use std::fs::create_dir;
use std::io::Result;
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Default)]
pub struct PlaceMetadata {
    pub connection: Text,
    pub description: Text,
    pub appearance: Text,
    pub other_senses: Text,
    pub notes: Text,
}

#[derive(Debug)]
pub struct Place {
    pub base: BaseFileObject,
    pub metadata: PlaceMetadata,
}

impl Place {
    pub fn new(dirname: PathBuf, index: usize) -> Result<Self> {
        let mut place = Self {
            base: BaseFileObject::new(dirname, Some(index)),
            metadata: PlaceMetadata::default(),
        };

        place.base.file.basename = place.calculate_filename();

        create_dir(place.get_path())?;
        place.save(&mut HashMap::new())?;

        Ok(place)
    }

    pub fn from_base(base: BaseFileObject) -> Result<Self> {
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
                return Err(err);
            }
        }

        Ok(place)
    }
}

impl FileObject for Place {
    fn load_metadata(&mut self) -> std::io::Result<bool> {
        let mut modified = false;

        match metadata_extract_string(&self.base.toml_header, "connection")? {
            Some(connection) => self.metadata.connection = connection.into(),
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "description")? {
            Some(description) => self.metadata.description = description.into(),
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "appearance")? {
            Some(appearance) => self.metadata.appearance = appearance.into(),
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "other_senses")? {
            Some(other_senses) => self.metadata.other_senses = other_senses.into(),
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "notes")? {
            Some(notes) => self.metadata.notes = notes.into(),
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

    fn write_metadata(&mut self) {
        self.base.toml_header["file_type"] = toml_edit::value("worldbuilding");
        self.base.toml_header["connection"] = toml_edit::value(&*self.metadata.connection);
        self.base.toml_header["description"] = toml_edit::value(&*self.metadata.description);
        self.base.toml_header["appearance"] = toml_edit::value(&*self.metadata.appearance);
        self.base.toml_header["other_senses"] = toml_edit::value(&*self.metadata.other_senses);
        self.base.toml_header["notes"] = toml_edit::value(&*self.metadata.notes);
    }

    fn as_editor(&self) -> &dyn crate::ui::FileObjectEditor {
        self
    }

    fn as_editor_mut(&mut self) -> &mut dyn crate::ui::FileObjectEditor {
        self
    }
}
