mod client;
mod connection;

mod parser;

pub mod incoming;

use std::{collections::HashMap, fmt::Display, result};

pub use client::*;
pub use connection::ConnectionSecurity;

use self::{
    incoming::{
        flags::Flag,
        message::{Address, Content},
    },
    parser::message::from_rfc822,
};
use crate::error::{err, Error, ErrorKind, Result};

pub type Headers = HashMap<String, String>;

pub struct MessageBuilder {
    from: Vec<Address>,
    to: Vec<Address>,
    cc: Vec<Address>,
    bcc: Vec<Address>,
    flags: Vec<Flag>,
    id: Option<String>,
    sent: Option<i64>,
    subject: Option<String>,
    headers: Option<Headers>,
    content: Option<Content>,
}

impl TryFrom<&[u8]> for MessageBuilder {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> result::Result<Self, Self::Error> {
        from_rfc822(bytes)
    }
}

impl MessageBuilder {
    pub fn new() -> Self {
        Self {
            from: Vec::new(),
            flags: Vec::new(),
            bcc: Vec::new(),
            cc: Vec::new(),
            to: Vec::new(),
            id: None,
            sent: None,
            subject: None,
            content: None,
            headers: None,
        }
    }

    pub fn add_flags<F: IntoIterator<Item = Flag>>(mut self, flags: F) -> Self {
        let mut iter = flags.into_iter();

        while let Some(flag) = iter.next() {
            self.flags.push(flag)
        }

        self
    }

    pub fn add_senders<C: IntoIterator<Item = Address>>(mut self, sender: C) -> Self {
        let mut iter = sender.into_iter();

        while let Some(address) = iter.next() {
            self.from.push(address)
        }

        self
    }
    pub fn add_recipients<C: IntoIterator<Item = Address>>(mut self, recipient: C) -> Self {
        let mut iter = recipient.into_iter();

        while let Some(address) = iter.next() {
            self.to.push(address)
        }

        self
    }

    pub fn add_cc<C: IntoIterator<Item = Address>>(mut self, cc: C) -> Self {
        let mut iter = cc.into_iter();

        while let Some(address) = iter.next() {
            self.cc.push(address)
        }

        self
    }

    pub fn add_bcc<C: IntoIterator<Item = Address>>(mut self, bcc: C) -> Self {
        let mut iter = bcc.into_iter();

        while let Some(address) = iter.next() {
            self.bcc.push(address)
        }

        self
    }

    pub fn set_id<I: Display>(mut self, id: I) -> Self {
        self.id = Some(id.to_string());

        self
    }

    pub fn set_sent(mut self, sent: i64) -> Self {
        self.sent = Some(sent);

        self
    }

    pub fn set_subject<S: Display>(mut self, subject: S) -> Self {
        self.subject = Some(subject.to_string());

        self
    }

    pub fn set_headers(mut self, headers: Headers) -> Self {
        self.headers = Some(headers);

        self
    }

    pub fn set_content(mut self, content: Content) -> Self {
        self.content = Some(content);

        self
    }

    pub fn build<T: TryFrom<Self>>(self) -> Result<T> {
        match self.try_into() {
            Ok(message) => Ok(message),
            Err(_err) => err!(ErrorKind::InvalidMessage, "Could not build a valid message"),
        }
    }
}
