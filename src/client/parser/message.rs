use std::collections::HashMap;

use chrono::DateTime;

use crate::{
    client::{address::Address, builder::MessageBuilder, content::Content, Headers},
    error::Result,
};

pub fn from_headers<B: AsRef<[u8]>>(response: B) -> Result<Headers> {
    let (parsed, _) = mailparse::parse_headers(response.as_ref())?;

    let mut headers: Headers = HashMap::new();

    for header in parsed.into_iter() {
        match headers.insert(header.get_key(), header.get_value()) {
            _ => {}
        }
    }

    Ok(headers)
}

pub fn from_body<B: AsRef<[u8]>>(body: B) -> Result<Content> {
    let parsed = mailparse::parse_mail(body.as_ref())?;

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

    Ok(Content::new(text, html))
}

pub fn from_rfc822<B: AsRef<[u8]>>(bytes: B) -> Result<MessageBuilder> {
    let headers = from_headers(&bytes)?;

    let content = from_body(&bytes)?;

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

    let mut message_builder = MessageBuilder::new()
        .headers(headers)
        .senders(from)
        .recipients(to)
        .cc(cc)
        .bcc(bcc);

    if let Some(html) = content.html {
        message_builder = message_builder.html(html)
    }

    if let Some(text) = content.text {
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
