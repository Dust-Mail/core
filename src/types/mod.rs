mod client;
mod connection;

mod parser;

pub mod incoming;
pub mod outgoing;

use std::{collections::HashMap, fmt::Display, result, str::FromStr};

pub use client::*;
pub use connection::ConnectionSecurity;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use self::incoming::flags::Flag;
use crate::error::{err, Error, ErrorKind, Result};

pub type Headers = HashMap<String, String>;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Content {
    text: Option<String>,
    html: Option<String>,
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

    fn set_text<T: Into<String>>(&mut self, text: T) {
        self.text = Some(text.into())
    }

    fn set_html<H: Into<String>>(&mut self, html: H) {
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

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Address {
    name: Option<String>,
    address: String,
}

impl<N: Into<String>, A: Into<String>> From<(N, A)> for Address {
    fn from((name, address): (N, A)) -> Self {
        Self::new(Some(name.into()), address.into())
    }
}

impl FromStr for Address {
    type Err = Error;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        Ok(email::Mailbox::from_str(s)?.into())
    }
}

impl Into<email::Address> for Address {
    fn into(self) -> email::Address {
        match self.name {
            Some(name) => email::Address::new_mailbox_with_name(name, self.address),
            None => email::Address::new_mailbox(self.address),
        }
    }
}

impl Into<email::Mailbox> for Address {
    fn into(self) -> email::Mailbox {
        match self.name {
            Some(name) => email::Mailbox::new_with_name(name, self.address),
            None => email::Mailbox::new(self.address),
        }
    }
}

impl From<email::Mailbox> for Address {
    fn from(mailbox: email::Mailbox) -> Self {
        Address::new(mailbox.name, mailbox.address)
    }
}

impl Address {
    pub fn new(name: Option<String>, address: String) -> Self {
        Self { name, address }
    }

    pub fn name(&self) -> &Option<String> {
        &self.name
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn full(&self) -> String {
        match self.name.as_ref() {
            Some(name) => format!("{} <{}>", name, self.address),
            None => self.address.to_string(),
        }
    }

    pub fn from_header<H: Into<String>>(header: H) -> Result<Vec<Self>> {
        parser::address::address_list(header)
    }
}

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
    content: Content,
}

impl TryFrom<&[u8]> for MessageBuilder {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> result::Result<Self, Self::Error> {
        parser::message::from_rfc822(bytes)
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
            content: Content::default(),
            headers: None,
        }
    }

    pub fn flags<F: IntoIterator<Item = Flag>>(mut self, flags: F) -> Self {
        let mut iter = flags.into_iter();

        while let Some(flag) = iter.next() {
            self.flags.push(flag)
        }

        self
    }

    pub fn senders<C: IntoIterator<Item = Address>>(mut self, sender: C) -> Self {
        let mut iter = sender.into_iter();

        while let Some(address) = iter.next() {
            self.from.push(address)
        }

        self
    }

    pub fn recipients<C: IntoIterator<Item = Address>>(mut self, recipient: C) -> Self {
        let mut iter = recipient.into_iter();

        while let Some(address) = iter.next() {
            self.to.push(address)
        }

        self
    }

    pub fn cc<C: IntoIterator<Item = Address>>(mut self, cc: C) -> Self {
        let mut iter = cc.into_iter();

        while let Some(address) = iter.next() {
            self.cc.push(address)
        }

        self
    }

    pub fn bcc<C: IntoIterator<Item = Address>>(mut self, bcc: C) -> Self {
        let mut iter = bcc.into_iter();

        while let Some(address) = iter.next() {
            self.bcc.push(address)
        }

        self
    }

    pub fn id<I: Display>(mut self, id: I) -> Self {
        self.id = Some(id.to_string());

        self
    }

    pub fn sent(mut self, sent: i64) -> Self {
        self.sent = Some(sent);

        self
    }

    pub fn subject<S: Display>(mut self, subject: S) -> Self {
        self.subject = Some(subject.to_string());

        self
    }

    pub fn headers(mut self, headers: Headers) -> Self {
        self.headers = Some(headers);

        self
    }

    pub fn html<H: Into<String>>(mut self, html: H) -> Self {
        self.content.set_html(html);

        self
    }

    pub fn text<H: Into<String>>(mut self, text: H) -> Self {
        self.content.set_text(text);

        self
    }

    pub fn build<T: TryFrom<Self>>(self) -> Result<T> {
        match self.try_into() {
            Ok(message) => Ok(message),
            Err(_err) => err!(ErrorKind::InvalidMessage, "Could not build a valid message"),
        }
    }
}
