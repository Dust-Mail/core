use std::collections::HashMap;

use chrono::DateTime;
use mailparse::ParsedMail;

use crate::{
    client::{address::Address, builder::MessageBuilder},
    error::Result,
};

pub fn from_parsed_mail<'a>(parsed_mail: ParsedMail<'a>) -> Result<MessageBuilder> {
    let mut headers = HashMap::new();

    for header in parsed_mail.get_headers().into_iter() {
        match headers.insert(header.get_key(), header.get_value()) {
            _ => {}
        }
    }

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

    let mut message_builder = MessageBuilder::new().headers(headers);

    if from.len() > 0 {
        message_builder = message_builder.senders(from);
    }

    if to.len() > 0 {
        message_builder = message_builder.recipients(to);
    }

    if cc.len() > 0 {
        message_builder = message_builder.cc(cc);
    }

    if bcc.len() > 0 {
        message_builder = message_builder.bcc(bcc);
    }

    let mut text: Option<String> = None;
    let mut html: Option<String> = None;

    for part in parsed_mail.parts() {
        let headers = part.get_headers();

        for header in headers {
            let key = header.get_key_ref().trim().to_ascii_lowercase();

            if key == "content-type" {
                let value = header.get_value().trim().to_ascii_lowercase();

                let body = Some(part.get_body()?);

                if value.starts_with("text/plain") {
                    text = match body {
                        Some(body_data) => Some(super::sanitize_text(&body_data)),
                        None => None,
                    }
                } else if value.starts_with("text/html") {
                    html = match body {
                        Some(body_data) => Some(super::sanitize_html(&body_data)),
                        None => None,
                    }
                }
            }
        }
    }

    if let Some(html) = html {
        message_builder = message_builder.html(html)
    }

    if let Some(text) = text {
        message_builder = message_builder.text(text)
    }

    if let Some(subject) = subject {
        message_builder = message_builder.subject(subject);
    }

    if let Some(sent) = sent {
        message_builder = message_builder.sent(sent);
    }

    Ok(message_builder)
}

pub fn from_rfc822<B: AsRef<[u8]>>(bytes: B) -> Result<MessageBuilder> {
    let parsed = mailparse::parse_mail(bytes.as_ref())?;

    Ok(from_parsed_mail(parsed)?)
}
