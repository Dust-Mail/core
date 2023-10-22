pub mod types;

#[cfg(feature = "imap")]
pub mod imap;

#[cfg(feature = "pop")]
pub mod pop;

#[cfg(feature = "maildir")]
pub mod maildir;
