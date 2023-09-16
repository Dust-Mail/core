#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum IncomingClientType {
    #[cfg(feature = "imap")]
    Imap,
    #[cfg(feature = "pop")]
    Pop,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum OutgoingClientType {
    #[cfg(feature = "smtp")]
    Smtp,
}
