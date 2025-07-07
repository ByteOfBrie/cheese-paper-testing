use std::fmt::Debug;

pub trait FileObjectEditorType<'a>: Debug {
    fn panels(&mut self, ctx: &egui::Context);
}
/*pub enum FileEditor<'a> {
    Scene(SceneEditor<'a>),
    Character(CharacterEditor<'a>),
    Folder(FolderEditor<'a>),
    Place(PlaceEditor<'a>),
}

pub struct FileObjectEditor<'a> {
    pub editor: FileEditor<'a>,
}

impl<'a> eframe::App for FileObjectEditor<'a> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match &mut self.editor {
            FileEditor::Scene(editor) => editor.panels(ctx),
            FileEditor::Character(editor) => editor.panels(ctx),
            FileEditor::Folder(editor) => editor.panels(ctx),
            FileEditor::Place(editor) => editor.panels(ctx),
        }
    }
}

impl<'a> FileObjectEditorType<'a> {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        file_object: &'a mut Box<dyn FileObject>,
    ) -> Self {
        match file_object.get_file_type_mut() {
            MutFileObjectTypeInterface::Scene(scene) => Self {
                editor: FileEditor::Scene(SceneEditor { scene: scene }),
            },
            MutFileObjectTypeInterface::Folder(folder) => Self {
                editor: FileEditor::Folder(FolderEditor { folder: folder }),
            },
            MutFileObjectTypeInterface::Character(character) => Self {
                editor: FileEditor::Character(CharacterEditor {
                    character: character,
                }),
            },
            MutFileObjectTypeInterface::Place(place) => Self {
                editor: FileEditor::Place(PlaceEditor { place: place }),
            },
        }
    }
}
*/
