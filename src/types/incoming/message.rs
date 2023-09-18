use std::{collections::HashMap, result};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    error::{err, Error, ErrorKind},
    types::{Address, Content, Headers, MessageBuilder},
};

use super::flags::Flag;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Preview {
    from: Vec<Address>,
    flags: Vec<Flag>,
    id: String,
    sent: Option<i64>,
    subject: Option<String>,
}

impl Preview {
    /// The sender(s) of the message.
    pub fn from(&self) -> &Vec<Address> {
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

        let preview = Preview {
            flags: builder.flags,
            from: builder.from,
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
    from: Vec<Address>,
    to: Vec<Address>,
    cc: Vec<Address>,
    bcc: Vec<Address>,
    headers: Headers,
    flags: Vec<Flag>,
    id: String,
    sent: Option<i64>,
    subject: Option<String>,
    content: Content,
}

impl TryFrom<MessageBuilder> for Message {
    type Error = Error;

    fn try_from(builder: MessageBuilder) -> result::Result<Self, Self::Error> {
        let id = match builder.id {
            Some(id) => id,
            None => err!(ErrorKind::InvalidMessage, "Message is missing identifier"),
        };

        let message = Message {
            flags: builder.flags,
            to: builder.to,
            from: builder.from,
            bcc: builder.bcc,
            cc: builder.cc,
            id,
            sent: builder.sent,
            subject: builder.subject,
            content: builder.content,
            headers: builder.headers.unwrap_or(HashMap::new()),
        };

        Ok(message)
    }
}

impl Message {
    pub fn from(&self) -> &Vec<Address> {
        &self.from
    }

    pub fn to(&self) -> &Vec<Address> {
        &self.to
    }

    pub fn cc(&self) -> &Vec<Address> {
        &self.cc
    }

    pub fn bcc(&self) -> &Vec<Address> {
        &self.bcc
    }

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
}
