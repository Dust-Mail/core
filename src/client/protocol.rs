use async_trait::async_trait;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{error::Result, tree::Node};

use super::{
    connection::ConnectionSecurity,
    incoming::types::{
        mailbox::Mailbox,
        message::{Message, Preview},
    },
    outgoing::types::sendable::SendableMessage,
};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RemoteServer {
    server: String,
    port: u16,
    security: ConnectionSecurity,
}

impl RemoteServer {
    pub fn new<Server: Into<String>>(
        server: Server,
        port: u16,
        security: ConnectionSecurity,
    ) -> Self {
        Self {
            server: server.into(),
            port,
            security,
        }
    }

    pub fn security(&self) -> &ConnectionSecurity {
        &self.security
    }

    pub fn domain(&self) -> &str {
        self.server.as_ref()
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Credentials {
    Password { username: String, password: String },
    OAuth { username: String, token: String },
}

impl Credentials {
    pub fn username(&self) -> &str {
        match &self {
            Credentials::OAuth { username, .. } => username,
            Credentials::Password { username, .. } => username,
        }
    }
}

impl Credentials {
    pub fn password<U: Into<String>, P: Into<String>>(username: U, password: P) -> Self {
        Credentials::Password {
            username: username.into(),
            password: password.into(),
        }
    }

    pub fn oauth<U: Into<String>, T: Into<String>>(username: U, token: T) -> Self {
        Credentials::OAuth {
            username: username.into(),
            token: token.into(),
        }
    }
}

pub trait ServerCredentials {
    fn credentials(&self) -> &Credentials;
}

#[cfg(feature = "smtp")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SmtpCredentials {
    server: RemoteServer,
    credentials: Credentials,
}

#[cfg(feature = "smtp")]
impl SmtpCredentials {
    pub fn new(server: RemoteServer, credentials: Credentials) -> Self {
        Self {
            server,
            credentials,
        }
    }

    pub fn server(&self) -> &RemoteServer {
        &self.server
    }
}

#[cfg(feature = "smtp")]
impl ServerCredentials for SmtpCredentials {
    fn credentials(&self) -> &Credentials {
        &self.credentials
    }
}

#[cfg(feature = "imap")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ImapCredentials {
    server: RemoteServer,
    credentials: Credentials,
}

#[cfg(feature = "imap")]
impl ImapCredentials {
    pub fn new(server: RemoteServer, credentials: Credentials) -> Self {
        Self {
            server,
            credentials,
        }
    }

    pub fn server(&self) -> &RemoteServer {
        &self.server
    }
}

#[cfg(feature = "imap")]
impl ServerCredentials for ImapCredentials {
    fn credentials(&self) -> &Credentials {
        &self.credentials
    }
}

#[cfg(feature = "pop")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PopCredentials {
    server: RemoteServer,
    credentials: Credentials,
}

#[cfg(feature = "pop")]
impl PopCredentials {
    pub fn new(server: RemoteServer, credentials: Credentials) -> Self {
        Self {
            server,
            credentials,
        }
    }

    pub fn server(&self) -> &RemoteServer {
        &self.server
    }
}

#[cfg(feature = "pop")]
impl ServerCredentials for PopCredentials {
    fn credentials(&self) -> &Credentials {
        &self.credentials
    }
}

#[async_trait]
pub trait IncomingProtocol {
    async fn send_keep_alive(&mut self) -> Result<()>;

    fn should_keep_alive(&self) -> bool;

    async fn get_mailbox_list(&mut self) -> Result<Node<Mailbox>>;

    async fn get_mailbox(&mut self, mailbox_id: &str) -> Result<Node<Mailbox>>;

    async fn rename_mailbox(&mut self, old_name: &str, new_name: &str) -> Result<()>;

    async fn create_mailbox(&mut self, name: &str) -> Result<()>;

    async fn delete_mailbox(&mut self, box_id: &str) -> Result<()>;

    async fn get_messages(
        &mut self,
        box_id: &str,
        start: usize,
        end: usize,
    ) -> Result<Vec<Preview>>;

    async fn get_message(&mut self, box_id: &str, message_id: &str) -> Result<Message>;

    async fn get_attachment(
        &mut self,
        box_id: &str,
        message_id: &str,
        attachment_id: &str,
    ) -> Result<Vec<u8>>;

    async fn logout(&mut self) -> Result<()>;
}

#[async_trait]
pub trait OutgoingProtocol {
    async fn send_message(&mut self, message: SendableMessage) -> Result<()>;
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum IncomingEmailProtocol {
    #[cfg(feature = "imap")]
    Imap(ImapCredentials),

    #[cfg(feature = "pop")]
    Pop(PopCredentials),

    #[cfg(feature = "maildir")]
    Maildir(std::path::PathBuf),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum OutgoingEmailProtocol {
    #[cfg(feature = "smtp")]
    Smtp(SmtpCredentials),
}

pub struct IncomingConfig {}

impl Default for IncomingConfig {
    fn default() -> Self {
        let config = Self::new();

        config
    }
}

impl IncomingConfig {
    pub fn new() -> Self {
        Self {}
    }
}
