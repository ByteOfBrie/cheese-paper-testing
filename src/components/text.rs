use std::ops::{Deref, DerefMut};

use crate::ui::RenderData;

/// An abstraction for a block of text.
#[derive(Debug, Default)]
pub struct Text {
    pub text: String,
    pub _rdata: RenderData,
}

impl From<String> for Text {
    fn from(s: String) -> Text {
        Text {
            text: s,
            _rdata: RenderData::default(),
        }
    }
}

impl Deref for Text {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.text
    }
}

impl DerefMut for Text {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.text
    }
}
