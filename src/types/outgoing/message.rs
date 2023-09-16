use bytes::Bytes;

use crate::{
    error::{err, Error, ErrorKind},
    types::{Address, MessageBuilder},
};

pub struct SendableMessage {
    from: Address,
    to: Vec<Address>,
    cc: Vec<Address>,
    bcc: Vec<Address>,
    subject: Bytes,
    content: Bytes,
}

#[cfg(feature = "smtp")]
use async_smtp::SendableEmail;

#[cfg(feature = "smtp")]
impl TryInto<SendableEmail> for SendableMessage {
    type Error = Error;

    fn try_into(self) -> Result<SendableEmail, Self::Error> {
        use async_smtp::{EmailAddress, Envelope};

        let from: EmailAddress = self
            .from
            .address()
            .as_ref()
            .map(|from| from.parse().unwrap())
            .unwrap();

        let to: Vec<EmailAddress> = self
            .to
            .iter()
            .filter_map(|to| to.address().as_ref().map(|addr| addr.parse().ok())?)
            .collect();

        let envelope = match Envelope::new(Some(from), to) {
            Ok(envelope) => envelope,
            Err(err) => err!(
                ErrorKind::InvalidMessage,
                "Failed to create message envelope: {}",
                err
            ),
        };

        let email = SendableEmail::new(envelope, self.content);

        Ok(email)
    }
}

impl TryFrom<MessageBuilder> for SendableMessage {
    type Error = Error;

    fn try_from(mut builder: MessageBuilder) -> Result<Self, Self::Error> {
        let from = if builder.from.is_empty() {
            err!(ErrorKind::InvalidMessage, "Missing message sender")
        } else {
            builder.from.swap_remove(0)
        };

        let content = match builder.content {
            Some(content) => {
                if let Some(html) = content.html() {
                    Bytes::from(html.to_string())
                } else if let Some(text) = content.text() {
                    Bytes::from(text.to_string())
                } else {
                    Bytes::new()
                }
            }
            None => Bytes::new(),
        };

        let subject = match builder.subject {
            Some(subject) => Bytes::from(subject),
            None => Bytes::new(),
        };

        let sendable = Self {
            from,
            to: builder.to,
            bcc: builder.bcc,
            cc: builder.cc,
            content,
            subject,
        };

        Ok(sendable)
    }
}
