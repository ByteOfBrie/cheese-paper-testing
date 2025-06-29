use crate::components::file_objects::base::{
    FileObjectType, metadata_extract_bool, metadata_extract_string,
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
    pub metadata: SceneMetadata,
    pub text: String,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            metadata: Default::default(),
            text: String::new(),
        }
    }
}

impl FileObjectType for Scene {
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
}

impl Scene {
    pub fn load_extra_data(&mut self, data: String) {
        self.text = data.trim().to_string();
    }

    pub fn get_body(&mut self) -> &mut String {
        &mut self.text
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
