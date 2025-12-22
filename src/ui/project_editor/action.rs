use super::ProjectEditor;

pub struct Action(Box<dyn FnOnce(&mut ProjectEditor)>);

impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "Project Editor Action")
    }
}

impl Action {
    pub fn new(f: impl FnOnce(&mut ProjectEditor) + 'static) -> Self {
        Self(Box::new(f))
    }

    pub fn perform(self, editor: &mut ProjectEditor) {
        self.0(editor)
    }
}
