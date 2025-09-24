use crate::cheese_error;
use crate::components::file_objects::FileObjectStore;
use crate::components::file_objects::base::{
    BaseFileObject, CompileStatus, FileObject, IncludeOptions, metadata_extract_string,
    metadata_extract_u64,
};
use crate::components::file_objects::utils::write_outline_property;
use crate::components::project::ExportOptions;
use crate::components::text::Text;
use crate::util::CheeseError;
use std::ffi::OsString;
use std::fs::create_dir;
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Default)]
pub struct FolderMetadata {
    pub summary: Text,
    pub notes: Text,
    pub compile_status: CompileStatus,
}

#[derive(Debug)]
pub struct Folder {
    pub base: BaseFileObject,
    pub metadata: FolderMetadata,
}

impl Folder {
    pub fn new(dirname: PathBuf, index: usize) -> Result<Self, CheeseError> {
        let mut folder = Self {
            base: BaseFileObject::new(dirname, Some(index)),
            metadata: FolderMetadata::default(),
        };

        folder.base.file.basename = folder.calculate_filename();

        create_dir(folder.get_path())?;
        <dyn FileObject>::save(&mut folder, &HashMap::new()).unwrap();

        Ok(folder)
    }

    /// Creates a top level folder (one that doesn't have an index) based on the name. The name will
    /// be used directly in the metadata, but convereted to lowercase for the version on disk
    pub fn new_top_level(dirname: PathBuf, name: &str) -> Result<Self, CheeseError> {
        let mut folder = Self {
            base: BaseFileObject::new(dirname, None),
            metadata: FolderMetadata::default(),
        };

        folder.get_base_mut().metadata.name = name.to_string();
        folder.get_base_mut().file.basename = OsString::from(name.to_lowercase());

        if let Err(err) = create_dir(folder.get_path()) {
            return Err(cheese_error!(
                "Failed to create top level directory: {:?}: {err}",
                folder.get_path()
            ));
        }

        if let Err(err) = <dyn FileObject>::save(&mut folder, &HashMap::new()) {
            return Err(cheese_error!(
                "Failed to save newly created top level directory: {}: {err}",
                &folder.get_base().metadata.name
            ));
        }

        Ok(folder)
    }

    pub fn from_base(base: BaseFileObject) -> Result<Self, CheeseError> {
        let mut folder = Self {
            base,
            metadata: Default::default(),
        };

        let modified = folder.load_metadata().map_err(|err| {
            cheese_error!(
                "Error while loading object-specific metadata for {:?}:\n{}",
                folder.get_path(),
                err
            )
        })?;

        if modified {
            folder.base.file.modified = true;
        }

        Ok(folder)
    }
}

impl FileObject for Folder {
    fn load_metadata(&mut self) -> Result<bool, CheeseError> {
        let mut modified = false;

        match metadata_extract_string(&self.base.toml_header, "summary")? {
            Some(value) => self.metadata.summary = value.into(),
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "notes")? {
            Some(notes) => self.metadata.notes = notes.into(),
            None => modified = true,
        }

        match metadata_extract_u64(&self.base.toml_header, "compile_status", true)? {
            Some(compile_status) => {
                self.metadata.compile_status = CompileStatus::from_bits_retain(compile_status)
            }
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

    fn get_file_type(&self) -> super::FileObjectTypeInterface<'_> {
        super::FileObjectTypeInterface::Folder(self)
    }

    fn get_file_type_mut(&mut self) -> super::MutFileObjectTypeInterface<'_> {
        super::MutFileObjectTypeInterface::Folder(self)
    }

    fn write_metadata(&mut self, _objects: &FileObjectStore) {
        self.base.toml_header["file_type"] = toml_edit::value("folder");
        self.base.toml_header["summary"] = toml_edit::value(&*self.metadata.summary);
        self.base.toml_header["notes"] = toml_edit::value(&*self.metadata.notes);
        self.base.toml_header["compile_status"] =
            toml_edit::value(self.metadata.compile_status.bits() as i64);
    }

    fn generate_outline(&self, depth: u64, export_string: &mut String, objects: &FileObjectStore) {
        (self as &dyn FileObject).write_title(depth, export_string);

        write_outline_property("summary", &self.metadata.summary, export_string);
        write_outline_property("notes", &self.metadata.notes, export_string);

        for child_id in self.get_base().children.iter() {
            objects.get(child_id).unwrap().borrow().generate_outline(
                depth + 1,
                export_string,
                objects,
            );
        }
    }

    fn generate_export(
        &self,
        depth: u64,
        export_string: &mut String,
        objects: &FileObjectStore,
        export_options: &ExportOptions,
        include_break: bool,
    ) -> bool {
        if self
            .metadata
            .compile_status
            .contains(CompileStatus::INCLUDE)
        {
            let display_title = match self.metadata.compile_status.include_title() {
                IncludeOptions::Always => true,
                IncludeOptions::Default => export_options.folder_title_depth.should_display(depth),
                IncludeOptions::Never => false,
            };

            // Keep track of whether the next scene will start with a break, which only ever gets
            // rendered in scenes
            let mut include_break_next = include_break;

            if display_title {
                (self as &dyn FileObject).write_title(depth, export_string);
                // We've written a title, so the requested break has been taken care of
                include_break_next = false;
            }

            // We don't actually have enough information here to decide to include a break, even
            // though it seems like we should. For example, we might have `include_break` set here
            // and no title displayed, but the next scene could actually start with a title, in which
            // case we shouldn't include the break here. Since we don't have any information about
            // what comes next, we just have to wait for the title to be drawn

            for child_id in self.get_base().children.iter() {
                // Keep passing the include_break status forwards along with any updates to it
                include_break_next = objects.get(child_id).unwrap().borrow().generate_export(
                    depth + 1,
                    export_string,
                    objects,
                    export_options,
                    include_break_next,
                );
            }

            let folder_include_break_next = match self.metadata.compile_status.break_at_end() {
                IncludeOptions::Always => true,
                IncludeOptions::Default => export_options.insert_breaks,
                IncludeOptions::Never => false,
            };

            // Request a break if either the final child or this folder should have a break
            include_break_next || folder_include_break_next
        } else {
            include_break
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
impl Folder {
    pub fn save(&mut self, objects: &FileObjectStore) -> Result<(), CheeseError> {
        (self as &mut dyn FileObject).save(objects)
    }
}
