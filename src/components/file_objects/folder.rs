use crate::components::file_objects::base::{
    BaseFileObject, FileObject, metadata_extract_bool, metadata_extract_string,
};
use std::ffi::OsString;
use std::io::Result;
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug)]
pub struct FolderMetadata {
    pub summary: String,
    pub notes: String,
    pub compile_status: bool,
}

impl Default for FolderMetadata {
    fn default() -> Self {
        Self {
            summary: String::new(),
            notes: String::new(),
            compile_status: true,
        }
    }
}

#[derive(Debug)]
pub struct Folder {
    pub base: BaseFileObject,
    pub metadata: FolderMetadata,
}

impl Folder {
    pub fn new(dirname: PathBuf, index: usize) -> Result<Self> {
        let mut folder = Self {
            base: BaseFileObject::new(dirname, Some(index)),
            metadata: FolderMetadata::default(),
        };

        folder.save(&mut HashMap::new())?;

        Ok(folder)
    }

    pub fn new_top_level(dirname: PathBuf, name: String) -> Result<Self> {
        let mut folder = Self {
            base: BaseFileObject::new(dirname, None),
            metadata: FolderMetadata::default(),
        };

        folder.get_base_mut().metadata.name = name.clone();
        folder.get_base_mut().file.basename = OsString::from(name);

        folder.save(&mut HashMap::new())?;

        Ok(folder)
    }

    pub fn from_base(base: BaseFileObject) -> Self {
        let mut folder = Self {
            base,
            metadata: Default::default(),
        };

        match folder.load_metadata() {
            Ok(modified) => {
                if modified {
                    folder.base.file.modified = true;
                }
            }
            Err(err) => {
                log::error!(
                    "Error while loading object-specific metadata for {:?}: {}",
                    folder.get_path(),
                    &err
                );
            }
        }

        folder
    }
}

impl FileObject for Folder {
    fn load_metadata(&mut self) -> std::io::Result<bool> {
        let mut modified = false;

        match metadata_extract_string(&self.base.toml_header, "summary")? {
            Some(value) => self.metadata.summary = value,
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "notes")? {
            Some(notes) => self.metadata.notes = notes,
            None => modified = true,
        }

        match metadata_extract_bool(&self.base.toml_header, "compile_status")? {
            Some(compile_status) => self.metadata.compile_status = compile_status,
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
        "New Folder"
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
        super::FileObjectTypeInterface::Folder(self)
    }

    fn get_file_type_mut(&mut self) -> super::MutFileObjectTypeInterface {
        super::MutFileObjectTypeInterface::Folder(self)
    }

    fn write_metadata(&mut self) {
        self.base.toml_header["summary"] = toml_edit::value(&self.metadata.summary);
        self.base.toml_header["notes"] = toml_edit::value(&self.metadata.notes);
        self.base.toml_header["compile_status"] = toml_edit::value(self.metadata.compile_status);
    }
}
