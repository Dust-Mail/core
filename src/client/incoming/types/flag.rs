#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Flag {
    HasAttachment,
    Read,
    Deleted,
    Answered,
    Flagged,
    Draft,
    Custom(Option<String>),
}

#[cfg(feature = "imap")]
use async_imap::types as imap;

impl Flag {
    #[cfg(feature = "imap")]
    pub fn from_imap(flag: &imap::Flag<'_>) -> Option<Self> {
        match flag {
            imap::Flag::Seen => Some(Self::Read),
            imap::Flag::Answered => Some(Self::Answered),
            imap::Flag::Draft => Some(Self::Draft),
            imap::Flag::Flagged => Some(Self::Flagged),
            imap::Flag::Deleted => Some(Self::Deleted),
            imap::Flag::Custom(value) => Some(Self::Custom(Some(value.to_string()))),
            _ => None,
        }
    }
}
