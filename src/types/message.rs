use chrono::DateTime;
use mailparse::parse_mail;
use serde::Serialize;

use crate::{
    error::Result,
    parse::{self, parse_headers},
};

use super::{Flag, Headers};

use email::FromHeader;

#[derive(Debug, Serialize)]
pub struct Address {
    name: Option<String>,
    address: Option<String>,
}

impl From<email::Mailbox> for Address {
    fn from(mailbox: email::Mailbox) -> Self {
        Address::new(mailbox.name, Some(mailbox.address))
    }
}

impl Address {
    pub fn new(name: Option<String>, address: Option<String>) -> Self {
        Self { name, address }
    }

    pub fn name(&self) -> &Option<String> {
        &self.name
    }

    pub fn address(&self) -> &Option<String> {
        &self.address
    }

    pub fn full(&self) -> Option<String> {
        if self.address.is_some() && self.name.is_some() {
            Some(format!(
                "{} <{}>",
                self.name.as_ref().unwrap(),
                self.address.as_ref().unwrap()
            ))
        } else {
            None
        }
    }

    pub fn from_header<H: Into<String>>(header: H) -> Result<Vec<Self>> {
        let address_list: Vec<email::Address> = Vec::from_header(header.into())?;

        let mut addresses = Vec::new();

        for address in address_list {
            match address {
                email::Address::Group(_name, list) => {
                    for mailbox in list {
                        addresses.push(mailbox.into());
                    }
                }
                email::Address::Mailbox(mailbox) => {
                    addresses.push(mailbox.into());
                }
            }
        }

        Ok(addresses)
    }
}

#[derive(Serialize, Debug)]
pub struct Preview {
    from: Vec<Address>,
    flags: Vec<Flag>,
    id: String,
    sent: Option<i64>,
    subject: Option<String>,
}

impl Preview {
    pub fn new<S: Into<String>>(
        from: Vec<Address>,
        flags: Vec<Flag>,
        id: S,
        sent: Option<i64>,
        subject: Option<String>,
    ) -> Self {
        Self {
            from,
            flags,
            id: id.into(),
            sent,
            subject,
        }
    }

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

    pub fn from_rfc822<B: AsRef<[u8]>, I: AsRef<str>>(
        bytes: B,
        message_id: I,
        flags: Vec<Flag>,
    ) -> Result<Self> {
        let headers = parse_headers(bytes)?;

        let subject = headers.get("Subject").cloned();

        let sent = match headers.get("Date") {
            Some(date) => {
                let datetime = DateTime::parse_from_rfc2822(date.trim())?;

                Some(datetime.timestamp())
            }
            None => None,
        };

        let from = match headers.get("From") {
            Some(from) => Address::from_header(from)?,
            None => Vec::new(),
        };

        let preview = Preview::new(from, flags, message_id.as_ref(), sent, subject);

        Ok(preview)
    }
}

#[derive(Serialize, Debug)]
pub struct Content {
    text: Option<String>,
    html: Option<String>,
}

impl Content {
    pub fn new(text: Option<String>, html: Option<String>) -> Self {
        Self { text, html }
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

    pub fn from_rfc822<B: AsRef<[u8]>>(bytes: B) -> Result<Self> {
        let parsed = parse_mail(bytes.as_ref())?;

        let mut text: Option<String> = None;
        let mut html: Option<String> = None;

        for part in parsed.parts() {
            let headers = part.get_headers();

            for header in headers {
                let key = header.get_key_ref().trim().to_ascii_lowercase();

                if key == "content-type" {
                    let value = header.get_value().trim().to_ascii_lowercase();

                    let body = Some(part.get_body()?);

                    if value.starts_with("text/plain") {
                        text = match body {
                            Some(body_data) => Some(parse::sanitize_text(&body_data)),
                            None => None,
                        }
                    } else if value.starts_with("text/html") {
                        html = match body {
                            Some(body_data) => Some(parse::sanitize_html(&body_data)),
                            None => None,
                        }
                    }
                }
            }
        }

        Ok(Content::new(text, html))
    }
}

#[derive(Serialize, Debug)]
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

impl Message {
    pub fn new<S: Into<String>>(
        from: Vec<Address>,
        to: Vec<Address>,
        cc: Vec<Address>,
        bcc: Vec<Address>,
        headers: Headers,
        flags: Vec<Flag>,
        id: S,
        sent: Option<i64>,
        subject: Option<String>,
        content: Content,
    ) -> Self {
        Self {
            from,
            to,
            cc,
            bcc,
            headers,
            flags,
            id: id.into(),
            sent,
            subject,
            content,
        }
    }

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

    pub fn from_rfc822<B: AsRef<[u8]>, I: Into<String>>(
        bytes: B,
        message_id: I,
        flags: Vec<Flag>,
    ) -> Result<Self> {
        let headers = parse_headers(&bytes)?;
        let content = Content::from_rfc822(&bytes)?;

        let subject = headers.get("Subject").cloned();

        let sent = match headers.get("Date") {
            Some(date) => {
                let datetime = DateTime::parse_from_rfc2822(date.trim())?;

                Some(datetime.timestamp())
            }
            None => None,
        };

        let from = match headers.get("From") {
            Some(from) => Address::from_header(from)?,
            None => Vec::new(),
        };

        let to = match headers.get("To") {
            Some(to) => Address::from_header(to)?,
            None => Vec::new(),
        };

        let bcc = match headers.get("BCC") {
            Some(bcc) => Address::from_header(bcc)?,
            None => Vec::new(),
        };

        let cc = match headers.get("CC") {
            Some(cc) => Address::from_header(cc)?,
            None => Vec::new(),
        };

        let message = Message::new(
            from, to, cc, bcc, headers, flags, message_id, sent, subject, content,
        );

        Ok(message)
    }

    #[cfg(feature = "json")]
    pub fn to_json(&self) -> Result<String> {
        parse::json::to_json(self)
    }
}
