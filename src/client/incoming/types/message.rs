use std::{collections::HashMap, result};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    client::{
        address::Address, attachment::Attachment, builder::MessageBuilder, content::Content,
        Headers,
    },
    error::{err, Error, ErrorKind},
};

use super::flag::Flag;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Preview {
    from: Address,
    flags: Vec<Flag>,
    id: String,
    sent: Option<i64>,
    subject: Option<String>,
}

impl Preview {
    /// The sender(s) of the message.
    pub fn from(&self) -> &Address {
        &self.from
    }

    /// The messages flags that indicate whether the message has been read, deleted, etc.
    pub fn flags(&self) -> &Vec<Flag> {
        &self.flags
    }

    /// A strictly unique id, used to fetch more info about the message.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Date in milliseconds since epoch
    pub fn sent(&self) -> Option<&i64> {
        self.sent.as_ref()
    }

    /// What the message is about.
    pub fn subject(&self) -> Option<&str> {
        match &self.subject {
            Some(subject) => Some(subject),
            None => None,
        }
    }

    #[cfg(feature = "json")]
    pub fn to_json(&self) -> Result<String> {
        parse::json::to_json(self)
    }
}

impl TryFrom<MessageBuilder> for Preview {
    type Error = Error;

    fn try_from(builder: MessageBuilder) -> result::Result<Preview, Self::Error> {
        let id = match builder.id {
            Some(id) => id,
            None => err!(ErrorKind::InvalidMessage, "Message is missing identifier"),
        };

        let from = match builder.from {
            Some(from) => from,
            None => err!(ErrorKind::InvalidMessage, "Message is missing sender"),
        };

        let mut flags = builder.flags;

        if !builder.attachments.is_empty() {
            flags.push(Flag::HasAttachment);
        }

        let preview = Preview {
            flags,
            from,
            id,
            sent: builder.sent,
            subject: builder.subject,
        };

        Ok(preview)
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Message {
    from: Address,
    to: Address,
    cc: Option<Address>,
    bcc: Option<Address>,
    headers: Headers,
    flags: Vec<Flag>,
    id: String,
    sent: Option<i64>,
    subject: Option<String>,
    attachments: Vec<Attachment>,
    content: Content,
}

impl TryFrom<MessageBuilder> for Message {
    type Error = Error;

    fn try_from(builder: MessageBuilder) -> result::Result<Self, Self::Error> {
        let id = match builder.id {
            Some(id) => id,
            None => err!(ErrorKind::InvalidMessage, "Missing message identifier"),
        };

        let from = match builder.from {
            Some(from) => from,
            None => err!(ErrorKind::InvalidMessage, "Missing message sender"),
        };

        let to = match builder.to {
            Some(to) => to,
            None => err!(ErrorKind::InvalidMessage, "Missing message receiver"),
        };

        let message = Message {
            flags: builder.flags,
            to,
            from,
            bcc: builder.bcc,
            cc: builder.cc,
            id,
            sent: builder.sent,
            subject: builder.subject,
            content: builder.content,
            attachments: builder.attachments,
            headers: builder.headers.unwrap_or(HashMap::new()),
        };

        Ok(message)
    }
}

impl Message {
    /// The message's RFC 822 headers.
    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    /// The messages flags that indicate whether the message has been read, deleted, etc.
    pub fn flags(&self) -> &Vec<Flag> {
        &self.flags
    }

    /// A strictly unique id, used to fetch more info about the message.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Date in milliseconds since epoch
    pub fn sent(&self) -> Option<&i64> {
        self.sent.as_ref()
    }

    /// What the message is about.
    pub fn subject(&self) -> Option<&str> {
        match &self.subject {
            Some(subject) => Some(subject),
            None => None,
        }
    }

    /// A struct containing info about the message content
    pub fn content(&self) -> &Content {
        &self.content
    }

    #[cfg(feature = "json")]
    pub fn to_json(&self) -> Result<String> {
        parse::json::to_json(self)
    }

    pub fn from(&self) -> &Address {
        &self.from
    }

    pub fn to(&self) -> &Address {
        &self.to
    }
}
