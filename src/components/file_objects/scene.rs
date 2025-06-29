use crate::components::file_objects::base::{
    ActualFileObject, BaseFileObject, metadata_extract_bool, metadata_extract_string,
};
use regex::Regex;
use toml::Table;

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

impl ActualFileObject for Scene {
    fn load_metadata(&mut self, table: &mut Table) -> std::io::Result<bool> {
        let mut modified = false;

        match metadata_extract_string(table, "summary")? {
            Some(summary) => self.metadata.summary = summary,
            None => modified = true,
        }

        match metadata_extract_string(table, "notes")? {
            Some(notes) => self.metadata.notes = notes,
            None => modified = true,
        }

        match metadata_extract_string(table, "pov")? {
            Some(pov) => self.metadata.pov = pov,
            None => modified = true,
        }

        match metadata_extract_bool(table, "compile_status")? {
            Some(compile_status) => self.metadata.compile_status = compile_status,
            None => modified = true,
        }

        Ok(modified)
    }

    fn is_folder(&self) -> bool {
        false
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
}

impl Scene {
    pub fn new(base: BaseFileObject) -> Self {
        Self {
            base,
            metadata: Default::default(),
            text: String::new(),
        }
    }

    pub fn word_count(&self) -> usize {
        let re = Regex::new(r"\s+").unwrap();
        re.split(&self.text).count()
    }

    pub fn assemble_save_text(&self) -> String {
        let mut full_text = String::new();

        for line in self.text.split('\n') {
            full_text.push_str(line.trim());
            full_text.push('\n');
        }

        full_text
    }
}
