use std::fmt::Display;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "imap")]
use async_imap::types::Mailbox as ImapCounts;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Mailbox {
    stats: Option<MailboxStats>,
    selectable: bool,
    id: String,
    name: String,
}

#[cfg(feature = "imap")]
impl From<&async_imap::types::Name> for Mailbox {
    fn from(mailbox: &async_imap::types::Name) -> Self {
        // Whether the inbox is selectable
        let selectable = !mailbox
            .attributes()
            .contains(&async_imap::types::NameAttribute::NoSelect);

        // Create an owned string if the delimiter is specified
        let delimiter = mailbox.delimiter().map(|del| del.to_string());

        let id = mailbox.name().to_string();

        // Split the id on the delimiter (using the default delimiter if it is not specified) and grab the last item
        // Example: 'INBOX.test.spam' becomes 'spam' if the delimiter is '.'
        let name = match delimiter.as_ref() {
            Some(delimiter) => id.split(delimiter).last().unwrap_or(&id).to_string(),
            None => id.to_string(),
        };

        Self {
            id,
            selectable,
            name,
            stats: None,
        }
    }
}

impl From<MailboxStats> for Mailbox {
    fn from(value: MailboxStats) -> Self {
        let mut default = Mailbox::default();

        default.stats = Some(value);

        default
    }
}

impl PartialEq for Mailbox {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Mailbox {
    pub fn new<S: Into<String>>(
        counts: Option<MailboxStats>,
        selectable: bool,
        id: S,
        name: S,
    ) -> Self {
        Self {
            stats: counts,
            selectable,
            id: id.into(),
            name: name.into(),
        }
    }

    /// A struct containing some info about the message counts in this mailbox.
    pub fn stats(&self) -> Option<&MailboxStats> {
        self.stats.as_ref()
    }

    /// Whether the mailbox contains messages and can be selected.
    pub fn selectable(&self) -> &bool {
        &self.selectable
    }

    /// A unique id for this mailbox.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The mailbox name.
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_stats(&mut self, stats: MailboxStats) {
        self.stats = Some(stats);
    }
}

const DEFAULT_MAILBOX_ID: &str = "default_inbox";
const DEFAULT_MAILBOX_NAME: &str = "Inbox";

impl Default for Mailbox {
    fn default() -> Self {
        Self {
            stats: Some(MailboxStats::default()),
            id: String::from(DEFAULT_MAILBOX_ID),
            name: String::from(DEFAULT_MAILBOX_NAME),
            selectable: true,
        }
    }
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
/// A struct that holds the count for the total amount messages and the total amount of unseen messages in a mailbox
pub struct MailboxStats {
    unseen: usize,
    total: usize,
}

impl Display for MailboxStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Total messages: {} | Unseen messages: {}",
            self.total, self.unseen,
        )
    }
}

impl MailboxStats {
    pub fn new(unseen: usize, total: usize) -> Self {
        MailboxStats { unseen, total }
    }

    /// The total amount of message that have not been read in this mailbox
    pub fn unseen(&self) -> usize {
        self.unseen
    }

    /// The total amount of messages in this mailbox
    pub fn total(&self) -> usize {
        self.total
    }
}

#[cfg(feature = "imap")]
impl From<ImapCounts> for MailboxStats {
    fn from(imap_counts: ImapCounts) -> Self {
        Self::new(
            imap_counts.unseen.unwrap_or(0) as usize,
            imap_counts.exists as usize,
        )
    }
}
