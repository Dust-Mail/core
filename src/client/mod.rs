use tokio::io::{AsyncRead, AsyncWrite};

trait Stream: AsyncRead + AsyncWrite {}

#[cfg(feature = "imap")]
use self::incoming::imap;

#[cfg(feature = "pop")]
use self::incoming::pop;

#[cfg(feature = "smtp")]
use self::outgoing::smtp;

use self::protocol::{IncomingProtocol, OutgoingProtocol};

pub use self::protocol::{Credentials, IncomingEmailProtocol, OutgoingEmailProtocol};

use crate::types::{Error, ErrorKind, MailBoxList, Message, Result};

mod incoming;
mod outgoing;

mod protocol;

pub struct EmailClient {
    incoming: Box<dyn IncomingProtocol>,
    outgoing: Box<dyn OutgoingProtocol>,
}

impl EmailClient {
    pub fn new(incoming: Box<dyn IncomingProtocol>, outgoing: Box<dyn OutgoingProtocol>) -> Self {
        Self { incoming, outgoing }
    }

    pub async fn get_mailbox_list(&mut self) -> Result<&MailBoxList> {
        self.incoming.get_mailbox_list().await
    }

    pub async fn rename_mailbox<OldName: AsRef<str>, NewName: AsRef<str>>(
        &mut self,
        old_name: OldName,
        new_name: NewName,
    ) -> Result<()> {
        self.incoming
            .rename_mailbox(old_name.as_ref(), new_name.as_ref())
            .await
    }

    pub async fn delete_mailbox<BoxId: AsRef<str>>(&mut self, box_id: BoxId) -> Result<()> {
        self.incoming.delete_mailbox(box_id.as_ref()).await
    }

    pub async fn create_mailbox<BoxName: AsRef<str>>(&mut self, box_id: BoxName) -> Result<()> {
        self.incoming.create_mailbox(box_id.as_ref()).await
    }

    pub async fn send_message(&mut self, message: Message) -> Result<()> {
        self.outgoing.send_message(message).await
    }
}

pub async fn create(
    incoming: IncomingEmailProtocol,
    outgoing: OutgoingEmailProtocol,
) -> Result<EmailClient> {
    let incoming_protocol: Box<dyn IncomingProtocol> = match incoming {
        #[cfg(feature = "imap")]
        IncomingEmailProtocol::Imap(credentials) => imap::create(&credentials).await?,

        #[cfg(feature = "pop")]
        IncomingEmailProtocol::Pop(credentials) => pop::create(&credentials).await?,
        _ => {
            return Err(Error::new(
                ErrorKind::NoClientAvailable,
                "There are no incoming mail clients supported",
            ));
        }
    };

    let outgoing_protocol = match outgoing {
        #[cfg(feature = "smtp")]
        OutgoingEmailProtocol::Smtp(credentials) => smtp::create(credentials)?,
        _ => {
            return Err(Error::new(
                ErrorKind::NoClientAvailable,
                "There are no outgoing mail clients supported",
            ));
        }
    };

    let client = EmailClient {
        incoming: incoming_protocol,
        outgoing: outgoing_protocol,
    };

    Ok(client)
}
