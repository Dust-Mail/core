use async_smtp::{EmailAddress, Envelope, SendableEmail};

use crate::types::{Error, ErrorKind, Message, Result};

pub fn create_sendable_message(message: Message) -> Result<SendableEmail> {
    let from = message
        .from()
        .first()
        .ok_or(Error::new(ErrorKind::InvalidMessage, "Missing sender"))?;

    let from: EmailAddress = from
        .address()
        .as_ref()
        .map(|from| from.parse().unwrap())
        .unwrap();

    let to: Vec<EmailAddress> = message
        .to()
        .iter()
        .filter_map(|to| to.address().as_ref().map(|addr| addr.parse().ok())?)
        .collect();

    let envelope = Envelope::new(Some(from), to).map_err(|_| {
        Error::new(
            ErrorKind::InvalidMessage,
            "Failed to create message envelope",
        )
    })?;

    let text = message.content().text().ok_or(Error::new(
        ErrorKind::InvalidMessage,
        "Missing message body",
    ))?;

    let email = SendableEmail::new(envelope, text);

    Ok(email)
}
