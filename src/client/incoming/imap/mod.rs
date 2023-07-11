mod keep_alive;
mod oauth;
mod parse;
use std::collections::HashMap;
// use std::collections::HashMap;
use std::fmt::Debug;

use async_native_tls::{TlsConnector, TlsStream};
use async_trait::async_trait;
use futures::StreamExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::time::Duration;

use crate::cache::{Cache, Refresher};
use crate::client::protocol::{Credentials, ImapCredentials, IncomingProtocol, ServerCredentials};
use crate::debug;
use crate::types::{
    ConnectionSecurity, Error, ErrorKind, MailBox, MailBoxList, Message, MessageCounts, Preview,
    Result,
};

use self::keep_alive::ImapSessionWithKeepAlive;
use self::oauth::OAuthCredentials;

const QUERY_PREVIEW: &str = "(FLAGS INTERNALDATE RFC822.SIZE ENVELOPE UID)";
const QUERY_FULL_MESSAGE: &str = "(FLAGS INTERNALDATE RFC822.SIZE ENVELOPE RFC822 UID)";

// const STATUS_ITEMS: &str = "(UNSEEN MESSAGES)";

const REFRESH_BOX_LIST: Duration = Duration::from_secs(10);
const REFRESH_MESSAGE_COUNT: Duration = Duration::from_secs(60);

struct BoxListRefresher<'a, S: AsyncRead + AsyncWrite + Unpin + Debug + Send + Sync> {
    session: &'a mut ImapSessionWithKeepAlive<S>,
    selected_box: &'a mut Option<String>,
    message_count: &'a mut HashMap<String, Cache<MessageCounts>>,
}

#[async_trait]
impl<S: AsyncRead + AsyncWrite + Unpin + Debug + Send + Sync> Refresher<MailBoxList>
    for BoxListRefresher<'_, S>
{
    async fn refresh(&mut self) -> Result<MailBoxList> {
        debug!("Refreshing box list...");

        // A planar graph of all of the mailboxes in the users account
        let mut mailboxes_planar: Vec<MailBox> = Vec::new();

        {
            let mut mailbox_stream = self.session.list(None, Some("*")).await?;

            while let Some(mailbox) = mailbox_stream.next().await {
                if let Ok(mailbox) = mailbox {
                    mailboxes_planar.push(MailBox::from(mailbox));
                }
            }
        }

        let mut boxes = MailBoxList::new(mailboxes_planar);

        let selected_box_before_refresh = self.selected_box.clone();

        for mailbox_mut in boxes.get_vec_mut() {
            if *mailbox_mut.selectable() {
                let message_counts_cache = self
                    .message_count
                    .entry(mailbox_mut.id().to_string())
                    .or_insert(Cache::new(REFRESH_MESSAGE_COUNT));

                let mut message_count_refresher = MessageCountRefresher {
                    selected_box: &mut self.selected_box,
                    box_id: mailbox_mut.id(),
                    session: &mut self.session,
                };

                let message_count = message_counts_cache
                    .get(&mut message_count_refresher)
                    .await?;

                // debug!("{:?}", message_count);

                mailbox_mut.create_counts(message_count.clone())
            }
        }

        // This is kind of a hacky work around we have to use,
        // as refreshing the message count will select a different mailbox because we cannot use the STATUS command at the moment
        if selected_box_before_refresh != *self.selected_box {
            debug!(
                "Selected box ({:?}, {:?}) changed when updating message counts, returning to old box",
                selected_box_before_refresh,
				self.selected_box
            );
            if let Some(selected_box_before_refresh) = selected_box_before_refresh.as_ref() {
                self.session.select(selected_box_before_refresh).await?;
            } else {
                debug!("Closed the box");
                self.session.close().await?;
            }

            *self.selected_box = selected_box_before_refresh
        }

        Ok(boxes)
    }
}

struct MessageCountRefresher<'a, S: AsyncRead + AsyncWrite + Unpin + Debug + Send> {
    session: &'a mut async_imap::Session<S>,
    selected_box: &'a mut Option<String>,
    box_id: &'a str,
}

#[async_trait]
impl<S: AsyncRead + AsyncWrite + Unpin + Debug + Send + Sync> Refresher<MessageCounts>
    for MessageCountRefresher<'_, S>
{
    async fn refresh(&mut self) -> Result<MessageCounts> {
        debug!("Refreshing message counts for {}", self.box_id);
        // TODO: use STATUS command
        // The status command always returns `total: 0` and `unseen: 0`, so for now we use examine to retrieve the message counts
        let imap_counts = self.session.examine(self.box_id).await?;

        let counts = MessageCounts::from(imap_counts);

        *self.selected_box = Some(self.box_id.to_string());

        Ok(counts)
    }
}

pub struct ImapClient<S: AsyncRead + AsyncWrite + Unpin + Debug + Send> {
    client: async_imap::Client<S>,
}

pub struct ImapSession<S: AsyncWrite + AsyncRead + Unpin + Debug + Send + Sync> {
    session: ImapSessionWithKeepAlive<S>,
    /// Counts per box
    message_count: HashMap<String, Cache<MessageCounts>>,
    box_list: Cache<MailBoxList>,
    /// The currently selected box' id.
    selected_box: Option<String>,
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

async fn create_session<S: AsyncRead + AsyncWrite + Unpin + Debug + Send + Sync>(
    imap_client: ImapClient<S>,
    credentials: &Credentials,
) -> Result<ImapSession<S>> {
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

impl<S: AsyncRead + AsyncWrite + Unpin + Debug + Send + Sync> ImapClient<S> {
    fn new_imap_session(session: async_imap::Session<S>) -> ImapSession<S> {
        let box_list_cache = Cache::new(REFRESH_BOX_LIST);

        let session_with_keep_alive = ImapSessionWithKeepAlive::new(session);

        ImapSession {
            session: session_with_keep_alive,
            message_count: HashMap::new(),
            box_list: box_list_cache,
            selected_box: None,
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
            .map_err(|(error, _)| {
                Error::new(
                    ErrorKind::Imap(error),
                    "Failed to login to remote IMAP server using password",
                )
            })?;

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
            .map_err(|(error, _)| {
                Error::new(
                    ErrorKind::Imap(error),
                    "Failed to login to remote IMAP server using oauth",
                )
            })?;

        let imap_session = Self::new_imap_session(session);

        Ok(imap_session)
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin + Debug + Send + Sync> ImapSession<S> {
    pub fn inner_mut(&mut self) -> &mut async_imap::Session<S> {
        &mut self.session
    }

    async fn get_mailbox_list(&mut self) -> Result<&MailBoxList> {
        let mut refresher = BoxListRefresher {
            selected_box: &mut self.selected_box,
            session: &mut self.session,
            message_count: &mut self.message_count,
        };

        let mail_box_list = self.box_list.get(&mut refresher).await?;

        Ok(mail_box_list)
    }

    async fn get_mailbox_list_mut(&mut self) -> Result<&mut MailBoxList> {
        let mut refresher = BoxListRefresher {
            selected_box: &mut self.selected_box,
            session: &mut self.session,
            message_count: &mut self.message_count,
        };

        let mail_box_list_mut = self.box_list.get_mut(&mut refresher).await?;

        Ok(mail_box_list_mut)
    }

    /// This function will check if a box with a given box id is actually selectable, throwing an error if it is not.
    async fn check_selectable(&mut self, box_id: &str) -> Result<()> {
        let box_list = self.get_mailbox_list().await?;

        let requested_box = box_list.get_box(box_id);

        match requested_box {
            Some(mailbox) => {
                if !mailbox.selectable() {
                    return Err(Error::new(
                        ErrorKind::MailServer,
                        format!("The mailbox with id '{}' is not selectable", box_id),
                    ));
                }
            }
            None => {}
        }

        Ok(())
    }

    /// Select a given box if it hasn't already been selected, otherwise return the already selected box.
    async fn select(&mut self, box_id: &str) -> Result<&MailBox> {
        let box_id = box_id.trim();

        let box_is_selected_already = self.selected_box.is_some();

        // If there is no box selected yet or the box we have selected is not the box we want to select, we have to request the server.
        if !box_is_selected_already || self.selected_box.as_ref().unwrap() != box_id {
            debug!("Selecting box: {}", box_id);

            let session = self.inner_mut();

            // If there is already a box selected we must close it first
            if box_is_selected_already {
                session.close().await?;
            }

            let imap_counts = session.select(&box_id).await?;

            let message_counts = MessageCounts::from(imap_counts);

            // Update the cached value so we don't refetch it in the near future.
            if let Some(cached_message_count) = self.message_count.get_mut(box_id) {
                cached_message_count.set(message_counts.clone());
            }

            self.selected_box = Some(String::from(box_id));

            let box_list_mut = self.get_mailbox_list_mut().await?;

            if let Some(box_mut) = box_list_mut.get_box_mut(box_id) {
                debug!("Creating counts for: {:?}", box_mut);
                box_mut.create_counts(message_counts);
            }
        };

        let box_list = self.get_mailbox_list().await?;

        if let Some(found_box) = box_list.get_box(box_id) {
            Ok(found_box)
        } else {
            Err(Error::new(
                ErrorKind::MailBoxNotFound,
                "Could not find a mailbox with that id",
            ))
        }
    }
}

#[async_trait]
impl<S: AsyncRead + AsyncWrite + Unpin + Debug + Send + Sync> IncomingProtocol for ImapSession<S> {
    async fn get_mailbox_list(&mut self) -> Result<&MailBoxList> {
        let mailbox_list = self.get_mailbox_list().await?;

        Ok(mailbox_list)
    }

    async fn get_mailbox(&mut self, mailbox_id: &str) -> Result<&MailBox> {
        let mailbox_list = self.get_mailbox_list().await?;

        if let Some(mailbox) = mailbox_list.get_box(mailbox_id) {
            Ok(mailbox)
        } else {
            Err(Error::new(
                ErrorKind::MailBoxNotFound,
                format!("Could not find a mailbox with id {}", mailbox_id),
            ))
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
        let mailbox_list = self.get_mailbox_list().await?;

        let mailbox = mailbox_list.get_box(box_id).ok_or(Error::new(
            ErrorKind::MessageNotFound,
            format!("Message with id {} could not be found", box_id),
        ))?;

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

        let selected_box = self.select(&box_id).await?;

        if let Some(counts) = selected_box.counts() {
            let total_messages = *counts.total();

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

            let mut previews: Vec<Preview> =
                Vec::with_capacity((end.saturating_sub(start)) as usize);

            {
                let mut preview_stream = session.fetch(sequence, QUERY_PREVIEW).await?;

                while let Some(fetch) = preview_stream.next().await {
                    let fetch = fetch?;

                    let preview = parse::fetch_to_preview(&fetch)?;

                    previews.push(preview);
                }
            }

            Ok(previews)
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_message(&mut self, box_id: &str, msg_id: &str) -> Result<Message> {
        self.check_selectable(box_id).await?;

        self.select(box_id).await?;

        let session = self.inner_mut();

        let mut fetch_stream = session.uid_fetch(msg_id, QUERY_FULL_MESSAGE).await?;

        let mut fetched = Vec::new();

        while let Some(fetch) = fetch_stream.next().await {
            let fetch = fetch?;

            let uid = match &fetch.uid {
                Some(uid) => uid,
                // Only returns None when the UID parameter is not specified.
                None => unreachable!(),
            };

            let msg_id: u32 = msg_id.parse().map_err(|_| {
                Error::new(
                    ErrorKind::ParseString,
                    "Failed to parse imap message uid to u32",
                )
            })?;

            // Only add the fetches that match our uid
            if uid == &msg_id {
                fetched.push(fetch);
            }
        }

        if fetched.len() < 1 {
            return Err(Error::new(
                ErrorKind::MessageNotFound,
                "Could not find a message with that id",
            ));
        };

        let fetch = match fetched.first() {
            Some(first) => first,
            None => unreachable!(),
        };

        parse::fetch_to_message(fetch).await
    }
}

#[cfg(test)]
mod tests {
    use async_native_tls::TlsStream;

    use env_logger::Env;
    use tokio::net::TcpStream;

    use crate::client::protocol::IncomingProtocol;

    use super::ImapSession;

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

    #[tokio::test]
    async fn login() {
        create_test_session().await;
    }

    #[tokio::test]
    async fn get_mailbox() {
        let mut session = create_test_session().await;

        let box_name = "INBOX";

        let mailbox_list = session.get_mailbox_list().await.unwrap();

        println!("{:?}", mailbox_list.get_box(box_name));
    }

    #[tokio::test]
    async fn get_messages() {
        let mut session = create_test_session().await;

        let box_name = "INBOX";

        let messages = session.get_messages(box_name, 0, 10).await.unwrap();

        for preview in messages.into_iter() {
            println!("{}", preview.sent().unwrap());
        }
    }

    #[tokio::test]
    async fn get_box_list() {
        env_logger::Builder::from_env(Env::default().default_filter_or("trace")).init();
        let mut session = create_test_session().await;

        let box_list = session.get_mailbox_list().await.unwrap();

        println!("{:?}", box_list);
    }

    #[tokio::test]
    async fn get_message() {
        let mut session = create_test_session().await;

        let msg_id = "1";
        let box_id = "INBOX";

        let message = session.get_message(box_id, msg_id).await.unwrap();

        println!("{}", message.id());
    }

    #[tokio::test]
    async fn rename_box() {
        let mut session = create_test_session().await;

        let new_box_name = "Delivery";
        let box_id = "Test";

        session.rename_mailbox(box_id, new_box_name).await.unwrap();
    }
}
