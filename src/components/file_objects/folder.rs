use crate::components::file_objects::base::{
    ActualFileObject, BaseFileObject, metadata_extract_bool, metadata_extract_string,
};

#[derive(Debug)]
struct FolderMetadata {
    summary: String,
    notes: String,
    compile_status: bool,
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
    base: BaseFileObject,
    metadata: FolderMetadata,
}

impl Folder {
    pub fn new(base: BaseFileObject) -> Self {
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

impl ActualFileObject for Folder {
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

    fn extension(&self) -> &'static str {
        "toml"
    }

    fn empty_string_name(&self) -> &'static str {
        "New Folder"
    }

    fn load_body(&mut self, _data: String) {}

    fn get_base(&self) -> &BaseFileObject {
        &self.base
    }

    fn get_base_mut(&mut self) -> &mut BaseFileObject {
        &mut self.base
    }
}
