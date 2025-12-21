
use crate::components::schema::{FileType, Schema};
use std::path::PathBuf;
use std::ffi::OsString;
use std::fs::create_dir;
use std::collections::HashMap;

use crate::cheese_error;

use crate::util::CheeseError;
use crate::components::file_objects::{FileObject, BaseFileObject};

impl dyn Schema {

    pub fn create_file(
        &self,
        file_type: FileType,
        dirname: PathBuf,
        index: usize,
    ) -> Result<Box<dyn FileObject>, CheeseError> {
        let base = BaseFileObject::new(dirname, Some(index));

        let mut file_object = self.init_file_object(file_type, base)?;

        file_object.get_base_mut().file.basename = file_object.calculate_filename();

        if file_type.is_folder() {
            create_dir(file_object.get_path())?;
        }

        file_object.save(&HashMap::new())?;

        Ok(file_object)
    }

    /// Creates a top level folder (one that doesn't have an index) based on the name. The name will
    /// be used directly in the metadata, but convereted to lowercase for the version on disk
    pub fn create_top_level_folder(
        &self,
        dirname: PathBuf,
        name: &str,
    ) -> Result<Box<dyn FileObject>, CheeseError> {
        let file_type = self.get_top_level_folder_type();
        assert!(file_type.is_folder());

        let mut base = BaseFileObject::new(dirname, None);

        base.metadata.name = name.to_string();
        base.file.basename = OsString::from(name.to_lowercase());

        let mut file_object = self.init_file_object(file_type, base)?;

        create_dir(file_object.get_path())
            .map_err(|err| cheese_error!("Failed to create top-level directory: {}: {err}", name))?;

        file_object.save(&HashMap::new()).map_err(|err| {
            cheese_error!(
                "Failed to save newly created top level directory: {}: {err}",
                name
            )
        })?;

        Ok(file_object)
    }

}
