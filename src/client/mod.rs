use std::{collections::HashMap, fmt::Display, sync::Arc};

use crate::{
    error::{Error, ErrorKind},
    runtime::thread::RwLock,
    tree::Node,
};

#[cfg(feature = "imap")]
use self::incoming::imap;

#[cfg(feature = "pop")]
use self::incoming::pop;

#[cfg(feature = "maildir")]
use self::incoming::maildir;

#[cfg(all(feature = "smtp", feature = "runtime-tokio"))]
use self::outgoing::smtp;

use self::{
    incoming::types::{
        mailbox::Mailbox,
        message::{Message, Preview},
    },
    outgoing::types::sendable::SendableMessage,
    protocol::{IncomingProtocol, OutgoingProtocol},
};

pub use self::{
    keep_alive::KeepAlive,
    protocol::{Credentials, IncomingEmailProtocol, OutgoingEmailProtocol, ServerCredentials},
};

use crate::error::Result;

mod incoming;
mod outgoing;

pub use incoming::types::*;
pub use outgoing::types::*;

pub mod address;
pub mod attachment;
pub mod builder;
pub mod connection;
pub mod content;

mod parser;

mod protocol;

mod keep_alive;

pub type Headers = HashMap<String, String>;

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

    pub fn should_keep_alive(&self) -> bool {
        self.incoming.should_keep_alive()
    }

    pub async fn get_mailbox_list(&mut self) -> Result<Node<Mailbox>> {
        self.incoming.get_mailbox_list().await
    }

    pub async fn get_mailbox<BoxId: AsRef<str>>(
        &mut self,
        mailbox_id: BoxId,
    ) -> Result<Node<Mailbox>> {
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
        let start = start.into();
        let end = end.into();

        if start >= end {
            return Ok(Vec::new());
        }

        self.incoming
            .get_messages(box_id.as_ref(), start, end)
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

    pub async fn get_attachment<
        BoxId: AsRef<str>,
        MessageId: AsRef<str>,
        AttachmentId: AsRef<str>,
    >(
        &mut self,
        box_id: BoxId,
        message_id: MessageId,
        attachment_id: AttachmentId,
    ) -> Result<Vec<u8>> {
        self.incoming
            .get_attachment(box_id.as_ref(), message_id.as_ref(), attachment_id.as_ref())
            .await
    }

    pub async fn send_message<M: TryInto<SendableMessage, Error = impl Display>>(
        &mut self,
        message: M,
    ) -> Result<()> {
        let sendable = message.try_into().map_err(|err| {
            Error::new(
                ErrorKind::InvalidMessage,
                format!("Failed to create sendable message: {}", err),
            )
        })?;

        self.outgoing.send_message(sendable).await
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
        IncomingEmailProtocol::Imap(credentials) => {
            imap::create(&credentials, Default::default()).await?
        }

        #[cfg(feature = "pop")]
        IncomingEmailProtocol::Pop(credentials) => pop::create(&credentials).await?,

        #[cfg(feature = "maildir")]
        IncomingEmailProtocol::Maildir(path) => maildir::create(path)?,

        #[cfg(not(any(feature = "imap", feature = "pop")))]
        _ => {
            use crate::error::{err, ErrorKind};

            err!(
                ErrorKind::NoClientAvailable,
                "There are no incoming mail clients supported",
            );
        }
    };

    let outgoing_protocol = match outgoing {
        #[cfg(all(feature = "smtp", feature = "runtime-tokio"))]
        OutgoingEmailProtocol::Smtp(credentials) => smtp::create(credentials)?,
        #[cfg(not(any(all(feature = "smtp", feature = "runtime-tokio"))))]
        _ => {
            use crate::error::{err, ErrorKind};

            err!(
                ErrorKind::NoClientAvailable,
                "There are no outgoing mail clients supported",
            );
        }
    };

    let client = EmailClient::new(incoming_protocol, outgoing_protocol);

    Ok(client)
}

/// An email client suitable for multithreading applications.
pub struct ThreadableEmailClient {
    client: Arc<RwLock<EmailClient>>,
    keep_alive: KeepAlive,
}

impl AsRef<Arc<RwLock<EmailClient>>> for ThreadableEmailClient {
    fn as_ref(&self) -> &Arc<RwLock<EmailClient>> {
        &self.client
    }
}

impl ThreadableEmailClient {
    pub fn new(client: Arc<RwLock<EmailClient>>, mut keep_alive: KeepAlive) -> Self {
        keep_alive.start();

        Self { client, keep_alive }
    }

    pub fn keep_alive(&self) -> &KeepAlive {
        &self.keep_alive
    }
}

impl From<EmailClient> for ThreadableEmailClient {
    fn from(client: EmailClient) -> Self {
        let client = Arc::new(RwLock::new(client));

        let keep_alive: KeepAlive = Arc::clone(&client).into();

        Self::new(client, keep_alive)
    }
}
