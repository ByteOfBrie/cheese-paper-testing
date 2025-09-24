use crate::components::file_objects::FileObjectStore;
use crate::components::file_objects::base::{BaseFileObject, FileObject, metadata_extract_string};
use crate::components::file_objects::utils::write_outline_property;
use crate::components::text::Text;
use crate::util::CheeseError;
use std::fs::create_dir;
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
    pub fn new(dirname: PathBuf, index: usize) -> Result<Self, CheeseError> {
        let mut place = Self {
            base: BaseFileObject::new(dirname, Some(index)),
            metadata: PlaceMetadata::default(),
        };

        place.base.file.basename = place.calculate_filename();

        create_dir(place.get_path())?;
        <dyn FileObject>::save(&mut place, &HashMap::new()).unwrap();

        Ok(place)
    }

    pub fn from_base(base: BaseFileObject) -> Result<Self, CheeseError> {
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
    fn load_metadata(&mut self) -> Result<bool, CheeseError> {
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

    fn get_file_type(&self) -> super::FileObjectTypeInterface<'_> {
        super::FileObjectTypeInterface::Place(self)
    }

    fn get_file_type_mut(&mut self) -> super::MutFileObjectTypeInterface<'_> {
        super::MutFileObjectTypeInterface::Place(self)
    }

    fn write_metadata(&mut self, _objects: &FileObjectStore) {
        self.base.toml_header["file_type"] = toml_edit::value("worldbuilding");
        self.base.toml_header["connection"] = toml_edit::value(&*self.metadata.connection);
        self.base.toml_header["description"] = toml_edit::value(&*self.metadata.description);
        self.base.toml_header["appearance"] = toml_edit::value(&*self.metadata.appearance);
        self.base.toml_header["other_senses"] = toml_edit::value(&*self.metadata.other_senses);
        self.base.toml_header["notes"] = toml_edit::value(&*self.metadata.notes);
    }

    fn generate_outline(&self, depth: u64, export_string: &mut String, objects: &FileObjectStore) {
        (self as &dyn FileObject).write_title(depth, export_string);

        write_outline_property("connection", &self.metadata.connection, export_string);
        write_outline_property("description", &self.metadata.description, export_string);
        write_outline_property("appearance", &self.metadata.appearance, export_string);
        write_outline_property("other_senses", &self.metadata.other_senses, export_string);
        write_outline_property("notes", &self.metadata.notes, export_string);

        for child_id in self.get_base().children.iter() {
            objects.get(child_id).unwrap().borrow().generate_outline(
                depth + 1,
                export_string,
                objects,
            );
        }
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
impl Place {
    pub fn save(&mut self, objects: &FileObjectStore) -> Result<(), CheeseError> {
        (self as &mut dyn FileObject).save(objects)
    }
}
