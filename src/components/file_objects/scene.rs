use crate::components::file_objects::base::{
    BaseFileObject, FileObject, metadata_extract_bool, metadata_extract_string,
};
use regex::Regex;

#[derive(Debug)]
pub struct SceneMetadata {
    pub summary: String,
    pub notes: String,
    pub pov: String, // TODO: create custom object for this
    pub compile_status: bool,
}

impl Default for SceneMetadata {
    fn default() -> Self {
        Self {
            summary: String::new(),
            notes: String::new(),
            pov: String::new(),
            compile_status: true,
        }
    }
}

#[derive(Debug)]
pub struct Scene {
    base: BaseFileObject,
    pub metadata: SceneMetadata,
    pub text: String,
}

impl FileObject for Scene {
    fn load_metadata(&mut self) -> std::io::Result<bool> {
        let mut modified = false;

        match metadata_extract_string(&self.base.toml_header, "summary")? {
            Some(summary) => self.metadata.summary = summary,
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "notes")? {
            Some(notes) => self.metadata.notes = notes,
            None => modified = true,
        }

        match metadata_extract_string(&self.base.toml_header, "pov")? {
            Some(pov) => self.metadata.pov = pov,
            None => modified = true,
        }

        match metadata_extract_bool(&self.base.toml_header, "compile_status")? {
            Some(compile_status) => self.metadata.compile_status = compile_status,
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
        self.text = data.trim().to_string();
    }

    fn get_base(&self) -> &BaseFileObject {
        &self.base
    }

    fn get_base_mut(&mut self) -> &mut BaseFileObject {
        &mut self.base
    }

    fn get_file_type(&self) -> super::FileObjectTypeInterface {
        super::FileObjectTypeInterface::Scene(self)
    }

    fn get_file_type_mut(&mut self) -> super::MutFileObjectTypeInterface {
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

    fn write_metadata(&mut self) {
        self.base.toml_header["summary"] = toml_edit::value(&self.metadata.summary);
        self.base.toml_header["notes"] = toml_edit::value(&self.metadata.notes);
        self.base.toml_header["pov"] = toml_edit::value(&self.metadata.pov);
        self.base.toml_header["compile_status"] = toml_edit::value(self.metadata.compile_status);
    }
}

impl Scene {
    pub fn new(base: BaseFileObject) -> Self {
        let mut scene = Self {
            base,
            metadata: Default::default(),
            text: String::new(),
        };

        match scene.load_metadata() {
            Ok(modified) => {
                if modified {
                    scene.base.file.modified = true;
                }
            }
            Err(err) => {
                // TODO: throw actual error
                log::error!(
                    "Error while loading object-specific metadata for {:?}: {}",
                    scene.get_path(),
                    &err
                );
            }
        }

        scene
    }

    pub fn word_count(&self) -> usize {
        let re = Regex::new(r"\s+").unwrap();
        re.split(&self.text).count()
    }
}
