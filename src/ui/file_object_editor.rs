use egui::Widget;

use crate::ui::{CharacterEditor, FolderEditor, PlaceEditor, SceneEditor};
use std::fmt::Debug;

pub trait FileObjectEditorType<'a>: Debug + Widget {}

impl<'a> FileObjectEditorType<'a> for &mut SceneEditor<'a> {}
impl<'a> FileObjectEditorType<'a> for &mut CharacterEditor<'a> {}
impl<'a> FileObjectEditorType<'a> for &mut FolderEditor<'a> {}
impl<'a> FileObjectEditorType<'a> for &mut PlaceEditor<'a> {}
