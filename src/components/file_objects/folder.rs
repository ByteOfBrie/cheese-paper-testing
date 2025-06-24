use crate::components::file_objects::base::{
    FileObjectType, metadata_extract_bool, metadata_extract_string,
};
use toml::Table;

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
    metadata: FolderMetadata,
}

impl Default for Folder {
    fn default() -> Self {
        Self {
            metadata: Default::default(),
        }
    }
}

impl FileObjectType for Folder {
    fn load_metadata(&mut self, table: &mut Table) -> std::io::Result<bool> {
        let mut modified = false;

        match metadata_extract_string(table, "summary")? {
            Some(value) => self.metadata.summary = value,
            None => modified = true,
        }

        match metadata_extract_string(table, "notes")? {
            Some(notes) => self.metadata.notes = notes,
            None => modified = true,
        }

        match metadata_extract_bool(table, "compile_status")? {
            Some(compile_status) => self.metadata.compile_status = compile_status,
            None => modified = true,
        }

        Ok(modified)
    }

    fn load_extra_data(&mut self, _data: String) {}
}
