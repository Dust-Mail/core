use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::types::{ConnectionSecurity, MailBox, MailBoxList, Message, Preview, Result};

#[derive(Deserialize, Serialize)]
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

#[derive(Deserialize, Serialize)]
pub enum Credentials {
    Password { username: String, password: String },
    OAuth { username: String, token: String },
}

pub trait ServerCredentials {
    fn credentials(&self) -> &Credentials;
}

#[cfg(feature = "smtp")]
#[derive(Deserialize, Serialize)]
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
#[derive(Deserialize, Serialize)]
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
#[derive(Deserialize, Serialize)]
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
    async fn get_mailbox_list(&mut self) -> Result<&MailBoxList>;

    async fn get_mailbox(&mut self, mailbox_id: &str) -> Result<&MailBox>;

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

    async fn logout(&mut self) -> Result<()>;
}

#[async_trait]
pub trait OutgoingProtocol {
    async fn send_message(&mut self, message: Message) -> Result<()>;
}

#[derive(Deserialize, Serialize)]
pub enum IncomingEmailProtocol {
    #[cfg(feature = "imap")]
    Imap(ImapCredentials),

    #[cfg(feature = "pop")]
    Pop(PopCredentials),
}

#[derive(Deserialize, Serialize)]
pub enum OutgoingEmailProtocol {
    #[cfg(feature = "smtp")]
    Smtp(SmtpCredentials),
}
