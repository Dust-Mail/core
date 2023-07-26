use tokio::io::{AsyncRead, AsyncWrite};

trait Stream: AsyncRead + AsyncWrite {}

#[cfg(feature = "imap")]
use self::incoming::imap;

#[cfg(feature = "pop")]
use self::incoming::pop;

#[cfg(feature = "smtp")]
use self::outgoing::smtp;

use self::protocol::{IncomingProtocol, OutgoingProtocol};

pub use self::protocol::{
    Credentials, IncomingEmailProtocol, OutgoingEmailProtocol, ServerCredentials,
};

use crate::types::{Error, ErrorKind, MailBox, MailBoxList, Message, Preview, Result};

mod incoming;
mod outgoing;

mod protocol;

pub struct EmailClient {
    incoming: Box<dyn IncomingProtocol + Sync + Send>,
    outgoing: Box<dyn OutgoingProtocol + Sync + Send>,
}

impl EmailClient {
    pub fn new(
        incoming: Box<dyn IncomingProtocol + Sync + Send>,
        outgoing: Box<dyn OutgoingProtocol + Sync + Send>,
    ) -> Self {
        Self { incoming, outgoing }
    }

    pub async fn send_keep_alive(&mut self) -> Result<()> {
        self.incoming.send_keep_alive().await
    }

    pub fn should_keep_alive(&mut self) -> bool {
        self.incoming.should_keep_alive()
    }

    pub async fn get_mailbox_list(&mut self) -> Result<&MailBoxList> {
        self.incoming.get_mailbox_list().await
    }

    pub async fn get_mailbox<BoxId: AsRef<str>>(&mut self, mailbox_id: BoxId) -> Result<&MailBox> {
        self.incoming.get_mailbox(mailbox_id.as_ref()).await
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

    pub async fn get_messages<BoxId: AsRef<str>, S: Into<usize>, E: Into<usize>>(
        &mut self,
        box_id: BoxId,
        start: S,
        end: E,
    ) -> Result<Vec<Preview>> {
        self.incoming
            .get_messages(box_id.as_ref(), start.into(), end.into())
            .await
    }

    pub async fn get_message<BoxId: AsRef<str>, MessageId: AsRef<str>>(
        &mut self,
        box_id: BoxId,
        message_id: MessageId,
    ) -> Result<Message> {
        self.incoming
            .get_message(box_id.as_ref(), message_id.as_ref())
            .await
    }

    pub async fn send_message(&mut self, message: Message) -> Result<()> {
        self.outgoing.send_message(message).await
    }

    pub async fn logout(&mut self) -> Result<()> {
        self.incoming.logout().await
    }
}

pub async fn create(
    incoming: IncomingEmailProtocol,
    outgoing: OutgoingEmailProtocol,
) -> Result<EmailClient> {
    let incoming_protocol = match incoming {
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
