/// Text editor view for an entire scene object, will be embeded in other file objects
use crate::components::file_objects::FileObject;

struct SceneTextEditor<'a> {
    scene: &'a mut FileObject,
}
