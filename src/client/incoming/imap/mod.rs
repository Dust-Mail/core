mod oauth;
mod query;

// use std::collections::HashMap;
use std::fmt::Debug;

use crate::{
    client::{
        builder::MessageBuilder,
        connection::ConnectionSecurity,
        protocol::{ImapCredentials, IncomingProtocol},
        Credentials, ServerCredentials,
    },
    error::{err, Error, ErrorKind, Result},
    runtime::{
        io::{Read, Write},
        net::TcpStream,
        time::{Duration, Instant},
    },
};

use async_native_tls::{TlsConnector, TlsStream};
use async_trait::async_trait;
use futures::StreamExt;
use log::{debug, info};

use self::oauth::OAuthCredentials;
use self::query::QueryBuilder;

use super::types::{
    flag::Flag,
    mailbox::{Mailbox, MailboxList, MailboxStats},
    message::{Message, Preview},
};

const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(29 * 60);

pub struct ImapClient<S: Read + Write + Unpin + Debug + Send> {
    client: async_imap::Client<S>,
}

pub struct ImapSession<S: Write + Read + Unpin + Debug + Send + Sync> {
    session: async_imap::Session<S>,
    /// The currently selected box' id and its stats.
    selected_box: Option<(String, MailboxStats)>,
    last_keep_alive: Option<Instant>,
}

pub async fn connect<S: AsRef<str>, P: Into<u16>>(
    server: S,
    port: P,
) -> Result<ImapClient<TlsStream<TcpStream>>> {
    let tls = TlsConnector::new();

    let tcp_stream = TcpStream::connect((server.as_ref(), port.into())).await?;

    let tls_stream = tls.connect(server.as_ref(), tcp_stream).await?;

    let client = async_imap::Client::new(tls_stream);

    let imap_client = ImapClient { client };

    Ok(imap_client)
}

pub async fn connect_plain<S: AsRef<str>, P: Into<u16>>(
    server: S,
    port: P,
) -> Result<ImapClient<TcpStream>> {
    let stream = TcpStream::connect((server.as_ref(), port.into())).await?;

    let client = async_imap::Client::new(stream);

    Ok(ImapClient { client })
}

async fn create_session<S: Read + Write + Unpin + Debug + Send + Sync>(
    imap_client: ImapClient<S>,
    credentials: &Credentials,
) -> Result<ImapSession<S>> {
    info!("Creating new imap session");

    let imap_session = match credentials {
        Credentials::OAuth { username, token } => imap_client.oauth2_login(username, token).await?,
        Credentials::Password { username, password } => {
            imap_client.login(username, password).await?
        }
    };

    Ok(imap_session)
}

/// Creates a new imap client from a given set of credentials
pub async fn create(
    credentials: &ImapCredentials,
) -> Result<Box<dyn IncomingProtocol + Sync + Send>> {
    match credentials.server().security() {
        ConnectionSecurity::Tls => {
            let imap_client =
                connect(credentials.server().domain(), credentials.server().port()).await?;

            let session = create_session(imap_client, &credentials.credentials()).await?;

            Ok(Box::new(session))
        }
        _ => {
            let imap_client =
                connect_plain(credentials.server().domain(), credentials.server().port()).await?;

            let session = create_session(imap_client, &credentials.credentials()).await?;

            Ok(Box::new(session))
        }
    }
}

impl<S: Read + Write + Unpin + Debug + Send + Sync> ImapClient<S> {
    fn new_imap_session(session: async_imap::Session<S>) -> ImapSession<S> {
        ImapSession {
            session,
            selected_box: None,
            last_keep_alive: None,
        }
    }

    pub async fn login<U: AsRef<str>, P: AsRef<str>>(
        self,
        username: U,
        password: P,
    ) -> Result<ImapSession<S>> {
        let session = self
            .client
            .login(username, password)
            .await
            .map_err(|(error, _)| Error::from(error))?;

        let imap_session = Self::new_imap_session(session);

        Ok(imap_session)
    }

    pub async fn oauth2_login<U: AsRef<str>, T: AsRef<str>>(
        self,
        user: U,
        token: T,
    ) -> Result<ImapSession<S>> {
        let auth = OAuthCredentials::new(user.as_ref(), token.as_ref());

        let session = self
            .client
            .authenticate("XOAUTH2", auth)
            .await
            .map_err(|(error, _)| Error::from(error))?;

        let imap_session = Self::new_imap_session(session);

        Ok(imap_session)
    }
}

impl<S: Read + Write + Unpin + Debug + Send + Sync> ImapSession<S> {
    pub fn inner_mut(&mut self) -> &mut async_imap::Session<S> {
        &mut self.session
    }

    /// This function will check if a box with a given box id is actually selectable, throwing an error if it is not.
    async fn check_selectable(&mut self, box_id: &str) -> Result<()> {
        let box_list = self.get_mailbox_list().await?;

        let requested_box = box_list.get_box(box_id);

        match requested_box {
            Some(mailbox) => {
                if !mailbox.selectable() {
                    err!(
                        ErrorKind::MailServer,
                        "The mailbox with id '{}' is not selectable",
                        box_id,
                    );
                }
            }
            None => {}
        }

        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        if let Some(_) = self.selected_box {
            self.session.close().await?;

            self.selected_box = None;
        }

        Ok(())
    }

    /// Select a given box if it hasn't already been selected, otherwise return the already selected box.
    async fn select<I: Into<String>>(&mut self, box_id: I) -> Result<&MailboxStats> {
        let box_id = box_id.into();

        // If there is no box selected yet or the box we have selected is not the box we want to select, we have to request the server.
        if !self.selected_box.is_some() || self.selected_box.as_ref().unwrap().0 != box_id {
            debug!("Selecting box: {}", box_id);

            // If there is already a box selected we must close it first
            self.close().await?;

            let imap_stats = self.session.select(&box_id).await?;

            self.selected_box = Some((box_id, imap_stats.into()));
        };

        if let Some((_id, stats)) = self.selected_box.as_ref() {
            return Ok(stats);
        }

        err!(
            ErrorKind::MailBoxNotFound,
            "Could not find a mailbox with that id",
        )
    }
}

#[async_trait]
impl<S: Read + Write + Unpin + Debug + Send + Sync> IncomingProtocol for ImapSession<S> {
    async fn send_keep_alive(&mut self) -> Result<()> {
        self.last_keep_alive = Some(Instant::now());

        self.session.noop().await?;

        Ok(())
    }

    fn should_keep_alive(&self) -> bool {
        if let Some(last_keep_alive) = self.last_keep_alive {
            Instant::now().duration_since(last_keep_alive) >= KEEP_ALIVE_INTERVAL
        } else {
            true
        }
    }

    async fn get_mailbox_list(&mut self) -> Result<MailboxList> {
        let mut mailboxes: Vec<Mailbox> = Vec::new();

        self.close().await?;

        {
            let mut mailbox_stream = self.session.list(None, Some("*")).await?;

            while let Some(mailbox) = mailbox_stream.next().await {
                mailboxes.push(mailbox?.into());
            }
        }

        Ok(mailboxes.into())
    }

    async fn get_mailbox(&mut self, mailbox_id: &str) -> Result<Mailbox> {
        let stats = self.select(mailbox_id).await?.clone();

        let mut mailboxes: Vec<Mailbox> = Vec::new();

        {
            let mut mailbox_stream = self.session.list(Some(mailbox_id), Some("*")).await?;

            while let Some(mailbox) = mailbox_stream.next().await {
                mailboxes.push(mailbox?.into());
            }
        }

        let tree: MailboxList = mailboxes.into();

        match tree
            .to_vec()
            .into_iter()
            .find(|mailbox| mailbox.id() == mailbox_id)
        {
            Some(mut mailbox) => {
                mailbox.add_stats(stats);

                Ok(mailbox)
            }
            None => {
                err!(
                    ErrorKind::MailBoxNotFound,
                    "Could not find a mailbox with that id",
                )
            }
        }
    }

    async fn logout(&mut self) -> Result<()> {
        self.session.logout().await?;

        Ok(())
    }

    async fn delete_mailbox(&mut self, box_id: &str) -> Result<()> {
        let session = self.inner_mut();

        session.delete(box_id).await?;

        Ok(())
    }

    async fn rename_mailbox(&mut self, box_id: &str, new_name: &str) -> Result<()> {
        let mailbox = self.get_mailbox(box_id).await?;

        let new_name = match mailbox.delimiter() {
            Some(delimiter) => {
                let item_count = box_id.matches(delimiter).count();

                if item_count >= 2 {
                    let split = box_id.split(delimiter);

                    let mut prefix = split
                        .take(item_count)
                        .collect::<Vec<&str>>()
                        .join(delimiter);

                    prefix.push_str(new_name);

                    prefix
                } else {
                    new_name.to_string()
                }
            }
            None => new_name.to_string(),
        };

        let session = self.inner_mut();

        session.close().await?;

        session.rename(box_id, &new_name).await?;

        Ok(())
    }

    async fn create_mailbox(&mut self, box_id: &str) -> Result<()> {
        let session = self.inner_mut();

        session.create(box_id).await?;

        Ok(())
    }

    async fn get_messages(
        &mut self,
        box_id: &str,
        start: usize,
        end: usize,
    ) -> Result<Vec<Preview>> {
        self.check_selectable(box_id).await?;

        let stats = self.select(box_id).await?;

        let total_messages = stats.total();

        if total_messages < 1 {
            return Ok(Vec::new());
        }

        let sequence_start = if total_messages < end {
            1
        } else {
            total_messages.saturating_sub(end).saturating_add(1)
        };

        let sequence_end = if total_messages < start {
            1
        } else {
            total_messages.saturating_sub(start)
        };

        let sequence = format!("{}:{}", sequence_start, sequence_end);

        let session = self.inner_mut();

        let mut previews = Vec::new();

        let query = QueryBuilder::default()
            .headers(vec!["From", "Date", "Subject"])
            .build();

        {
            let mut preview_stream = session.fetch(sequence, &query).await?;

            while let Some(fetch) = preview_stream.next().await {
                let fetch = fetch?;

                if let Some(headers) = fetch.header() {
                    let message_id = match fetch.uid {
                        Some(uid) => uid,
                        None => err!(
                            ErrorKind::InvalidMessage,
                            "The requested message is missing a unique identifier"
                        ),
                    };

                    let flags = fetch
                        .flags()
                        .into_iter()
                        .filter_map(|flag| Flag::from_imap(&flag));

                    let builder: MessageBuilder = headers.try_into()?;

                    let preview: Preview = builder.flags(flags).id(message_id).build()?;

                    previews.push(preview);
                }
            }
        }

        Ok(previews)
    }

    async fn get_message(&mut self, box_id: &str, msg_id: &str) -> Result<Message> {
        self.check_selectable(box_id).await?;

        self.select(box_id).await?;

        let query = QueryBuilder::default().body().build();

        let mut fetch_stream = self.session.uid_fetch(msg_id, query).await?;

        let mut fetched = Vec::new();

        while let Some(fetch) = fetch_stream.next().await {
            let fetch = fetch?;

            let server_uid = match &fetch.uid {
                Some(uid) => uid,
                // Only returns None when the UID parameter is not specified.
                None => unreachable!(),
            };

            let uid: u32 = msg_id.parse()?;

            // Only add the fetches that match our uid
            if &uid == server_uid {
                fetched.push(fetch);
            }
        }

        let fetch = match fetched.first() {
            Some(first) => first,
            None => err!(
                ErrorKind::MessageNotFound,
                "Could not find a message with that id",
            ),
        };

        let message_id = match fetch.uid {
            Some(uid) => uid.to_string(),
            None => err!(
                ErrorKind::InvalidMessage,
                "The requested message is missing a unique identifier"
            ),
        };

        let flags = fetch
            .flags()
            .into_iter()
            .filter_map(|flag| Flag::from_imap(&flag));

        match fetch.body() {
            Some(body) => {
                let builder: MessageBuilder = body.try_into()?;

                let message: Message = builder.flags(flags).id(message_id).build()?;

                Ok(message)
            }
            None => {
                err!(
                    ErrorKind::InvalidMessage,
                    "The requested message is missing a body"
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use dotenv::dotenv;

    use std::env;

    async fn create_test_session() -> ImapSession<TlsStream<TcpStream>> {
        dotenv().ok();

        let username = env::var("IMAP_USERNAME").unwrap();
        let password = env::var("IMAP_PASSWORD").unwrap();

        let server = env::var("IMAP_SERVER").unwrap();
        let port: u16 = 993;

        let client = super::connect(server, port).await.unwrap();

        let session = client.login(&username, &password).await.unwrap();

        session
    }

    #[cfg_attr(feature = "runtime-async-std", async_std::test)]
    #[cfg_attr(feature = "runtime-tokio", tokio::test)]
    async fn login() {
        create_test_session().await;
    }

    #[cfg_attr(feature = "runtime-async-std", async_std::test)]
    #[cfg_attr(feature = "runtime-tokio", tokio::test)]
    async fn get_mailbox() {
        let mut session = create_test_session().await;

        let box_id = "Education";

        let mailbox = session.get_mailbox(box_id).await.unwrap();

        println!("{:?}", mailbox);
    }

    #[cfg_attr(feature = "runtime-async-std", async_std::test)]
    #[cfg_attr(feature = "runtime-tokio", tokio::test)]
    async fn get_messages() {
        let mut session = create_test_session().await;

        let box_name = "INBOX";

        let messages = session.get_messages(box_name, 0, 10).await.unwrap();

        for preview in messages.into_iter() {
            println!("{:?}", preview);
        }
    }

    #[cfg_attr(feature = "runtime-async-std", async_std::test)]
    #[cfg_attr(feature = "runtime-tokio", tokio::test)]
    async fn get_box_list() {
        // env_logger::Builder::from_env(Env::default().default_filter_or("trace")).init();
        let mut session = create_test_session().await;

        let box_list = session.get_mailbox_list().await.unwrap();

        println!("{:?}", box_list);
    }

    #[cfg_attr(feature = "runtime-async-std", async_std::test)]
    #[cfg_attr(feature = "runtime-tokio", tokio::test)]
    async fn get_message() {
        let mut session = create_test_session().await;

        let msg_id = "1658";
        let box_id = "INBOX";

        let message = session.get_message(box_id, msg_id).await.unwrap();

        println!("{:?}", message);
    }

    #[cfg_attr(feature = "runtime-async-std", async_std::test)]
    #[cfg_attr(feature = "runtime-tokio", tokio::test)]
    async fn rename_box() {
        let mut session = create_test_session().await;

        let new_box_name = "Delivery";
        let box_id = "Test";

        session.rename_mailbox(box_id, new_box_name).await.unwrap();
    }
}
