use crate::{
    components::file_objects::{BaseFileObject, FileObject},
    util::CheeseError,
};

use std::path::Path;

pub use crate::schemas::FileType;

pub trait Schema {
    fn get_schema_name(&self) -> &'static str;

    fn resolve_type(
        &self,
        filename: &Path,
        file_type_identifier: Option<&str>,
    ) -> Result<FileType, CheeseError>;

    fn get_all_file_types(&self) -> &'static [FileType];

    fn get_top_level_folder_type(&self) -> FileType;

    fn init_file_object(
        &self,
        file_type: FileType,
        base: BaseFileObject,
    ) -> Result<Box<dyn FileObject>, CheeseError>;

    fn load_file_object(
        &self,
        file_type: FileType,
        base: BaseFileObject,
        body: Option<String>,
    ) -> Result<Box<dyn FileObject>, CheeseError>;
}

impl std::fmt::Debug for dyn Schema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[FileType: {}]", self.get_schema_name())
    }
}
