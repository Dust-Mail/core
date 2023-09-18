use std::result;

use crate::{
    error::{err, Error, ErrorKind},
    types::{Address, Content, MessageBuilder},
};

use email::{Header, MimeMessage};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SendableMessage {
    from: Address,
    to: Vec<Address>,
    cc: Vec<Address>,
    bcc: Vec<Address>,
    subject: String,
    content: Content,
}

#[cfg(feature = "smtp")]
use async_smtp::SendableEmail;

#[cfg(feature = "smtp")]
impl TryInto<SendableEmail> for SendableMessage {
    type Error = Error;

    fn try_into(self) -> result::Result<SendableEmail, Self::Error> {
        use async_smtp::Envelope;

        let from: async_smtp::EmailAddress = self.from.address().parse().unwrap();

        let to: Vec<async_smtp::EmailAddress> = self
            .to
            .iter()
            .filter_map(|to| to.address().parse().ok())
            .collect();

        let envelope = match Envelope::new(Some(from), to) {
            Ok(envelope) => envelope,
            Err(err) => err!(
                ErrorKind::InvalidMessage,
                "Failed to create message envelope: {}",
                err
            ),
        };

        let message: MimeMessage = self.try_into()?;

        let email = SendableEmail::new(envelope, message.as_string());

        Ok(email)
    }
}

impl TryInto<MimeMessage> for SendableMessage {
    type Error = Error;

    fn try_into(self) -> result::Result<MimeMessage, Self::Error> {
        let mut parts = Vec::new();

        if let Some(html) = self.content.html {
            let mut html_content = MimeMessage::new(html);

            html_content.headers.insert(Header::new_with_value(
                String::from("Content-Type"),
                "text/html",
            )?);

            html_content.update_headers();

            parts.push(html_content)
        }

        if let Some(text) = self.content.text {
            let mut text_content = MimeMessage::new(text);

            text_content.headers.insert(Header::new_with_value(
                String::from("Content-Type"),
                "text/plain",
            )?);

            text_content.update_headers();

            parts.push(text_content)
        }

        let mut message = MimeMessage::new_blank_message();

        for part in parts.into_iter() {
            message.children.push(part);
        }

        if self.to.len() > 0 {
            message.headers.insert(Header::new_with_value(
                String::from("To"),
                self.to
                    .into_iter()
                    .map(|addr| addr.into())
                    .collect::<Vec<email::Address>>(),
            )?);
        }

        if self.cc.len() > 0 {
            message.headers.insert(Header::new_with_value(
                String::from("CC"),
                self.cc
                    .into_iter()
                    .map(|addr| addr.into())
                    .collect::<Vec<email::Address>>(),
            )?);
        }

        if self.bcc.len() > 0 {
            message.headers.insert(Header::new_with_value(
                String::from("BCC"),
                self.bcc
                    .into_iter()
                    .map(|addr| addr.into())
                    .collect::<Vec<email::Address>>(),
            )?);
        }

        message.headers.insert(Header::new_with_value(
            String::from("From"),
            vec![self.from.into()],
        )?);

        message.headers.insert(Header::new_with_value(
            String::from("Subject"),
            self.subject,
        )?);

        message.headers.insert(Header::new_with_value(
            String::from("Content-Type"),
            format!("multipart/alternative; boundary=\"{}\"", message.boundary),
        )?);

        message.update_headers();

        Ok(message)
    }
}

impl TryFrom<MessageBuilder> for SendableMessage {
    type Error = Error;

    fn try_from(mut builder: MessageBuilder) -> result::Result<Self, Self::Error> {
        let from = if builder.from.is_empty() {
            err!(ErrorKind::InvalidMessage, "Missing message sender")
        } else {
            builder.from.swap_remove(0)
        };

        let sendable = Self {
            from,
            to: builder.to,
            bcc: builder.bcc,
            cc: builder.cc,
            content: builder.content,
            subject: builder.subject.unwrap_or(String::new()),
        };

        Ok(sendable)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_to_mime() {
        let builder = MessageBuilder::new()
            .recipients(vec!["Tester <test@example.com>".parse().unwrap()])
            .senders(vec![()])
            .subject("Test email")
            .text("Hello world!")
            .html(
                "<!DOCTYPE html><html><head><title>Example Email</title></head><body><p>Hello Jane,</p><p>Here's the HTML version of the email.</p><p>Best regards,<br>John</p></body></html>",
            );

        let sendable: SendableMessage = builder.build().unwrap();
        let mime_message: MimeMessage = sendable.try_into().unwrap();

        println!("{}", mime_message.as_string())
    }
}
