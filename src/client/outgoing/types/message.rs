use std::result;

use crate::{
    client::{address::Address, builder::MessageBuilder, content::Content},
    error::{err, Error, ErrorKind},
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SendableMessage {
    from: Address,
    to: Address,
    cc: Option<Address>,
    bcc: Option<Address>,
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

        let from: Option<async_smtp::EmailAddress> =
            self.from.first().map(|addr| addr.email().parse().unwrap());

        let to: Vec<async_smtp::EmailAddress> = self
            .to
            .as_list()
            .iter()
            .filter_map(|to| to.email().parse().ok())
            .collect();

        let envelope = match Envelope::new(from, to) {
            Ok(envelope) => envelope,
            Err(err) => err!(
                ErrorKind::InvalidMessage,
                "Failed to create message envelope: {}",
                err
            ),
        };

        let message: String = self.try_into()?;

        let email = SendableEmail::new(envelope, message);

        Ok(email)
    }
}

impl TryInto<String> for SendableMessage {
    type Error = Error;

    fn try_into(self) -> result::Result<String, Self::Error> {
        let mut builder = mail_builder::MessageBuilder::new()
            .from(self.from)
            .to(self.to)
            .subject(self.subject);

        if let Some(cc) = self.cc {
            builder = builder.cc(cc);
        }

        if let Some(bcc) = self.bcc {
            builder = builder.bcc(bcc);
        }

        if let Some(text) = self.content.text {
            builder = builder.text_body(text);
        }

        if let Some(html) = self.content.html {
            builder = builder.html_body(html);
        }

        Ok(builder.write_to_string()?)
    }
}

impl TryFrom<MessageBuilder> for SendableMessage {
    type Error = Error;

    fn try_from(builder: MessageBuilder) -> result::Result<Self, Self::Error> {
        let from = match builder.from {
            Some(from) => from,
            None => {
                err!(ErrorKind::InvalidMessage, "Missing message sender");
            }
        };

        let to = match builder.to {
            Some(to) => to,
            None => {
                err!(ErrorKind::InvalidMessage, "Missing message receiver");
            }
        };

        let sendable = Self {
            from,
            to,
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
            .recipients(("Tester", "test@example.com"))
            .senders(("User", "user@example.com"))
            .subject("Test email")
            .text("Hello world!")
            .html(
                "<!DOCTYPE html><html><head><title>Example Email</title></head><body><p>Hello Jane,</p><p>Here's the HTML version of the email.</p><p>Best regards,<br>John</p></body></html>",
            );

        let sendable: SendableMessage = builder.build().unwrap();
        let message_str: String = sendable.try_into().unwrap();

        println!("{}", message_str)
    }
}
