#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Attachment {
    id: String,
    file_name: Option<String>,
    size: usize,
}

impl Attachment {
    pub fn new(id: String, file_name: Option<String>, size: usize) -> Self {
        Self {
            id,
            file_name,
            size,
        }
    }

    pub fn id(&self) -> &str {
        self.id.as_ref()
    }

    pub fn file_name(&self) -> Option<&String> {
        self.file_name.as_ref()
    }

    pub fn size(&self) -> usize {
        self.size
    }
}
