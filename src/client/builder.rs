use std::{collections::HashMap, fmt::Display, result};

use crate::error::{err, Error, ErrorKind, Result};

use super::{address::Address, content::Content, incoming::types::flag::Flag, parser, Headers};

#[derive(Debug)]
pub struct MessageBuilder {
    pub(crate) from: Option<Address>,
    pub(crate) to: Option<Address>,
    pub(crate) cc: Option<Address>,
    pub(crate) bcc: Option<Address>,
    pub(crate) flags: Vec<Flag>,
    pub(crate) id: Option<String>,
    pub(crate) sent: Option<i64>,
    pub(crate) subject: Option<String>,
    pub(crate) headers: Option<Headers>,
    pub(crate) content: Content,
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
            flags: Vec::new(),
            from: None,
            bcc: None,
            cc: None,
            to: None,
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

    pub fn senders<C: Into<Address>>(mut self, sender: C) -> Self {
        self.from = Some(sender.into());

        self
    }

    pub fn recipients<C: Into<Address>>(mut self, recipient: C) -> Self {
        self.to = Some(recipient.into());

        self
    }

    pub fn cc<C: Into<Address>>(mut self, cc: C) -> Self {
        self.cc = Some(cc.into());

        self
    }

    pub fn bcc<C: Into<Address>>(mut self, bcc: C) -> Self {
        self.bcc = Some(bcc.into());

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

    pub fn header<H: Into<String>, V: Display>(mut self, header: H, value: V) -> Self {
        if let None = self.headers {
            self.headers = Some(HashMap::new());
        }

        if let Some(headers) = self.headers.as_mut() {
            headers.insert(header.into(), value.to_string());
        }

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
