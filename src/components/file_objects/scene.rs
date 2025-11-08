use regex::Regex;

use crate::components::file_objects::FileObjectStore;
use crate::components::file_objects::FileType;
use crate::components::file_objects::base::{
    BaseFileObject, CompileStatus, FileObject, IncludeOptions, metadata_extract_string,
    metadata_extract_u64,
};
use crate::components::file_objects::reference::ObjectReference;
use crate::components::file_objects::utils::write_outline_property;
use crate::components::project::ExportOptions;
use crate::components::text::Text;
use crate::util::CheeseError;
use std::cell::RefCell;
use std::rc::Rc;
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Default)]
pub struct SceneMetadata {
    pub summary: Text,
    pub notes: Text,
    pub pov: Rc<RefCell<ObjectReference>>,
    pub compile_status: CompileStatus,
}

#[derive(Debug)]
pub struct Scene {
    base: BaseFileObject,
    pub metadata: SceneMetadata,
    pub text: Text,
}

impl FileObject for Scene {
    fn load_metadata(&mut self) -> Result<bool, CheeseError> {
        let mut modified = false;

        match metadata_extract_string(self.base.toml_header.as_table(), "summary")? {
            Some(summary) => self.metadata.summary = summary.into(),
            None => modified = true,
        }

        match metadata_extract_string(self.base.toml_header.as_table(), "notes")? {
            Some(notes) => self.metadata.notes = notes.into(),
            None => modified = true,
        }

        match metadata_extract_string(self.base.toml_header.as_table(), "pov")? {
            Some(pov) => {
                self.metadata.pov = Rc::new(RefCell::new(ObjectReference::new(
                    pov,
                    Some(FileType::Character),
                )))
            }
            None => modified = true,
        }

        match metadata_extract_u64(self.base.toml_header.as_table(), "compile_status", true)? {
            Some(compile_status) => {
                self.metadata.compile_status = CompileStatus::from_bits_retain(compile_status)
            }
            None => modified = true,
        }

        Ok(modified)
    }

    fn is_folder(&self) -> bool {
        false
    }

    fn has_body(&self) -> bool {
        true
    }

    fn extension(&self) -> &'static str {
        "md"
    }

    fn empty_string_name(&self) -> &'static str {
        "New Scene"
    }

    fn load_body(&mut self, data: String) {
        self.text = data.trim().to_string().into();
    }

    fn get_base(&self) -> &BaseFileObject {
        &self.base
    }

    fn get_base_mut(&mut self) -> &mut BaseFileObject {
        &mut self.base
    }

    fn get_file_type(&self) -> super::FileObjectTypeInterface<'_> {
        super::FileObjectTypeInterface::Scene(self)
    }

    fn get_file_type_mut(&mut self) -> super::MutFileObjectTypeInterface<'_> {
        super::MutFileObjectTypeInterface::Scene(self)
    }

    fn get_body(&self) -> String {
        let mut full_text = String::new();

        for line in self.text.split('\n') {
            full_text.push_str(line.trim());
            full_text.push('\n');
        }

        full_text
    }

    fn write_metadata(&mut self, objects: &FileObjectStore) {
        self.base.toml_header["file_type"] = toml_edit::value("scene");
        self.base.toml_header["summary"] = toml_edit::value(&*self.metadata.summary);
        self.base.toml_header["notes"] = toml_edit::value(&*self.metadata.notes);
        self.base.toml_header["pov"] =
            toml_edit::value(self.metadata.pov.borrow().to_string(objects));
        self.base.toml_header["compile_status"] =
            toml_edit::value(self.metadata.compile_status.bits() as i64);
    }

    fn generate_outline(&self, depth: u64, export_string: &mut String, objects: &FileObjectStore) {
        (self as &dyn FileObject).write_title(depth, export_string);

        write_outline_property("summary", &self.metadata.summary, export_string);
        write_outline_property(
            "pov",
            &self.metadata.pov.borrow().to_string(objects),
            export_string,
        );
        write_outline_property("notes", &self.metadata.notes, export_string);
    }

    fn generate_export(
        &self,
        depth: u64,
        export_string: &mut String,
        _objects: &FileObjectStore,
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
                IncludeOptions::Default => export_options.scene_title_depth.should_display(depth),
                IncludeOptions::Never => false,
            };

            if display_title {
                (self as &dyn FileObject).write_title(depth, export_string);
            } else if include_break {
                // We only include a break if the previous scene/document requested it *and* we
                // didn't already include a heading (title)
                export_string.push_str("----\n\n");
            }

            let body_text_unprocessed = &self.get_body();

            // add in smart quotes, other platforms will insert some and it's easier to be consistent here
            // regexes from https://webapps.stackexchange.com/questions/166314/how-to-replace-dumb-quotes-with-smart-quotes-in-google-docs/169065#169065
            // quotes preceded by whitespace or at the start of a block are beginning quotes
            let opening_double_quote = Regex::new(r#"((^|\s)\*{0,3})""#).unwrap();
            let closing_double_quote = Regex::new("\"").unwrap();

            // same thing for opening quotes
            let opening_single_quote = Regex::new(r#"((^|\s)\*{0,3})'"#).unwrap();
            let closing_single_quote = Regex::new("'").unwrap();

            let body_text = opening_double_quote.replace_all(body_text_unprocessed, "$1“");
            let body_text = closing_double_quote.replace_all(&body_text, "”");

            let body_text = opening_single_quote.replace_all(&body_text, "$1‘");
            let body_text = closing_single_quote.replace_all(&body_text, "’");

            // This should probably eventually be split into a `get_body_export` and `get_body_save`
            // function once those are different (probably for in-text-notes)
            export_string.push_str(&body_text);

            while !export_string.ends_with("\n\n") {
                export_string.push('\n');
            }

            // Determine if there should be a break after this scene and return it
            match self.metadata.compile_status.break_at_end() {
                IncludeOptions::Always => true,
                IncludeOptions::Default => export_options.insert_breaks,
                IncludeOptions::Never => false,
            }
        } else {
            // We didn't do anything, keep the same state
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

impl Scene {
    pub fn new(dirname: PathBuf, index: usize) -> Result<Self, CheeseError> {
        let mut scene = Self {
            base: BaseFileObject::new(dirname, Some(index)),
            metadata: SceneMetadata::default(),
            text: Text::default(),
        };

        scene.base.file.basename = scene.calculate_filename();

        <dyn FileObject>::save(&mut scene, &HashMap::new()).unwrap();

        Ok(scene)
    }

    pub fn from_file_object(base: BaseFileObject) -> Result<Self, CheeseError> {
        let mut scene = Self {
            base,
            metadata: Default::default(),
            text: Text::default(),
        };

        match scene.load_metadata() {
            Ok(modified) => {
                if modified {
                    scene.base.file.modified = true;
                }
            }
            Err(err) => {
                log::error!(
                    "Error while loading object-specific metadata for {:?}: {}",
                    scene.get_path(),
                    &err
                );
                return Err(err);
            }
        }

        Ok(scene)
    }
}

// shortcuts for not having to cast every time

#[cfg(test)]
impl Scene {
    pub fn save(&mut self, objects: &FileObjectStore) -> Result<(), CheeseError> {
        (self as &mut dyn FileObject).save(objects)
    }
}
