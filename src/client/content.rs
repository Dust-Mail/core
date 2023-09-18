#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Content {
    pub(crate) text: Option<String>,
    pub(crate) html: Option<String>,
}

impl<T: Into<String>> From<T> for Content {
    fn from(text: T) -> Self {
        Self::from_text(text)
    }
}

impl Default for Content {
    fn default() -> Self {
        Self {
            html: None,
            text: None,
        }
    }
}

impl Content {
    pub fn new(text: Option<String>, html: Option<String>) -> Self {
        Self { text, html }
    }

    pub fn from_text<T: Into<String>>(text: T) -> Self {
        Self::new(Some(text.into()), None)
    }

    pub fn set_text<T: Into<String>>(&mut self, text: T) {
        self.text = Some(text.into())
    }

    pub fn set_html<H: Into<String>>(&mut self, html: H) {
        self.html = Some(html.into())
    }

    /// The message in pure text form.
    pub fn text(&self) -> Option<&str> {
        match &self.text {
            Some(text) => Some(text),
            None => None,
        }
    }

    /// The message as a html page.
    pub fn html(&self) -> Option<&str> {
        match &self.html {
            Some(html) => Some(html),
            None => None,
        }
    }
}
