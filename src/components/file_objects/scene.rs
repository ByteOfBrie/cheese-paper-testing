use crate::components::file_objects::base::{
    BaseFileObject, CompileStatus, FileObject, metadata_extract_string, metadata_extract_u64,
};
use crate::components::text::Text;
use regex::Regex;
use std::io::Result;
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Default)]
pub struct SceneMetadata {
    pub summary: Text,
    pub notes: Text,
    pub pov: Text, // TODO: create custom object for this
    pub compile_status: CompileStatus,
}

#[derive(Debug)]
pub struct Scene {
    base: BaseFileObject,
    pub metadata: SceneMetadata,
    pub text: Text,
}

impl FileObject for Scene {
    fn load_metadata(&mut self) -> std::io::Result<bool> {
        let mut modified = false;

        match metadata_extract_string(&self.base.toml_header, "summary")? {
            Some(summary) => self.metadata.summary = summary.into(),
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "notes")? {
            Some(notes) => self.metadata.notes = notes.into(),
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "pov")? {
            Some(pov) => self.metadata.pov = pov.into(),
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

    fn get_body(&self) -> String {
        let mut full_text = String::new();

        for line in self.text.split('\n') {
            full_text.push_str(line.trim());
            full_text.push('\n');
        }

        full_text
    }

    fn write_metadata(&mut self) {
        self.base.toml_header["file_type"] = toml_edit::value("scene");
        self.base.toml_header["summary"] = toml_edit::value(&*self.metadata.summary);
        self.base.toml_header["notes"] = toml_edit::value(&*self.metadata.notes);
        self.base.toml_header["pov"] = toml_edit::value(&*self.metadata.pov);
        self.base.toml_header["compile_status"] =
            toml_edit::value(self.metadata.compile_status.bits() as i64);
    }

    fn as_editor(&self) -> &dyn crate::ui::FileObjectEditor {
        self
    }

    fn as_editor_mut(&mut self) -> &mut dyn crate::ui::FileObjectEditor {
        self
    }
}

impl Scene {
    pub fn new(dirname: PathBuf, index: usize) -> Result<Self> {
        let mut scene = Self {
            base: BaseFileObject::new(dirname, Some(index)),
            metadata: SceneMetadata::default(),
            text: Text::default(),
        };

        scene.base.file.basename = scene.calculate_filename();

        <dyn FileObject>::save(&mut scene, &HashMap::new()).unwrap();

        Ok(scene)
    }

    pub fn from_file_object(base: BaseFileObject) -> Result<Self> {
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

    pub fn word_count(&self) -> usize {
        let re = Regex::new(r"\s+").unwrap();
        re.split(&self.text).count()
    }
}

// shortcuts for not having to cast every time

#[cfg(test)]
impl Scene {
    pub fn save(&mut self, objects: &super::FileObjectStore) -> Result<()> {
        (self as &mut dyn FileObject).save(objects)
    }
}
