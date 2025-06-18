use crate::components::file_objects::base::{FileObjectBase, FileObjectType};
use std::path::Path;

// TODO: set defaults
struct SceneMetadata {
    name: String,
    notes: String,
    summary: String,
    compile_status: bool,
    pov: String, // TODO: create custom object for this
}
/*

pub struct Scene {
    base: FileObjectBase,
    metadata: SceneMetadata,
    text: String, // TODO: probably use some better string type like a rope
}

impl Scene {
    pub fn save(&mut self) {}
    pub fn load_from_disk(&mut self) {}
}
*/

pub struct Scene {
    metadata: SceneMetadata,
    text: String, // TODO: better type
}

impl FileObjectType for Scene {
    fn save(&mut self, dest_path: &Path) {}
    fn load_from_disk(&mut self, source_path: &Path) {}
}
