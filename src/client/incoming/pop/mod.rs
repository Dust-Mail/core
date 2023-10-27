mod constants;

use std::{collections::HashMap, fmt::Display};

use async_native_tls::{TlsConnector, TlsStream};
use async_pop::{
    response::{
        types::DataType,
        uidl::{UidlResponse, UniqueId},
    },
    sasl::OAuth2Authenticator,
};
use async_trait::async_trait;

use crate::{
    client::{
        builder::MessageBuilder,
        connection::ConnectionSecurity,
        protocol::{Credentials, IncomingProtocol, PopCredentials, ServerCredentials},
    },
    error::{err, ErrorKind, Result},
    runtime::{
        io::{Read, Write},
        net::TcpStream,
    },
    tree::Node,
};

use self::constants::ACTIVITY_TIMEOUT;

use super::types::{
    flag::Flag,
    mailbox::{Mailbox, MailboxStats},
    message::{Message, Preview},
};

pub struct PopClient<S: Read + Write + Unpin + Send> {
    session: async_pop::Client<S>,
}

impl<S: Read + Write + Unpin + Send> PopClient<S> {
    pub async fn login<U: AsRef<str>, P: AsRef<str>>(
        mut self,
        username: U,
        password: P,
    ) -> Result<PopSession<S>> {
        self.session.login(username, password).await?;

        let session = PopSession::new(self.session);

        Ok(session)
    }

    pub async fn oauth_login<U: AsRef<str>, T: AsRef<str>>(
        mut self,
        username: U,
        token: T,
    ) -> Result<PopSession<S>> {
        let oauth_authenticator = OAuth2Authenticator::new(username.as_ref(), token.as_ref());

        self.session.auth(oauth_authenticator).await?;

        let session = PopSession::new(self.session);

        Ok(session)
    }
}

struct UniqueIdMap {
    map: HashMap<String, usize>,
}

impl UniqueIdMap {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    fn reset(&mut self) {
        self.map.clear();
    }

    fn get_id(&self, index: usize) -> Option<&str> {
        for (curr_id, curr_index) in &self.map {
            if &index == curr_index {
                return Some(curr_id);
            }
        }

        None
    }

    fn get<I: AsRef<str>>(&self, id: I) -> Option<usize> {
        self.map.get(id.as_ref()).map(|index| *index)
    }

    fn set<I: Display>(&mut self, id: I, index: usize) {
        self.map.insert(id.to_string(), index);
    }

    fn extend<'a, L: IntoIterator<Item = &'a UniqueId>>(&mut self, list: L) -> Result<()> {
        for unique_id in list {
            self.set(unique_id.id(), unique_id.index().value()?)
        }

        Ok(())
    }
}

pub struct PopSession<S: Read + Write + Unpin + Send> {
    session: async_pop::Client<S>,
    unique_id_map: UniqueIdMap,
}

pub async fn connect<S: AsRef<str>, P: Into<u16>>(
    server: S,
    port: P,
) -> Result<PopClient<TlsStream<TcpStream>>> {
    let tls = TlsConnector::new();

    let tcp_stream = TcpStream::connect((server.as_ref(), port.into())).await?;

    let tls_stream = tls.connect(server.as_ref(), tcp_stream).await?;

    let session = async_pop::new(tls_stream).await?;

    Ok(PopClient { session })
}

pub async fn connect_plain<S: AsRef<str>, P: Into<u16>>(
    server: S,
    port: P,
) -> Result<PopClient<TcpStream>> {
    let tcp_stream = TcpStream::connect((server.as_ref(), port.into())).await?;

    let session = async_pop::new(tcp_stream).await?;

    Ok(PopClient { session })
}

async fn login<S: Read + Write + Unpin + Send>(
    client: PopClient<S>,
    credentials: &Credentials,
) -> Result<PopSession<S>> {
    match credentials {
        Credentials::Password { username, password } => {
            let session = client.login(username, password).await?;

            Ok(session)
        }
        Credentials::OAuth { username, token } => {
            let session = client.oauth_login(username, token).await?;

            Ok(session)
        }
    }
}

pub async fn create(
    credentials: &PopCredentials,
) -> Result<Box<dyn IncomingProtocol + Sync + Send>> {
    match credentials.server().security() {
        ConnectionSecurity::Tls => {
            let client =
                connect(credentials.server().domain(), credentials.server().port()).await?;

            let session = login(client, credentials.credentials()).await?;

            Ok(Box::new(session))
        }
        _ => {
            let client =
                connect_plain(credentials.server().domain(), credentials.server().port()).await?;

            let session = login(client, credentials.credentials()).await?;

            Ok(Box::new(session))
        }
    }
}

impl<S: Read + Write + Unpin + Send> PopSession<S> {
    pub fn new(session: async_pop::Client<S>) -> Self {
        Self {
            session,

            unique_id_map: UniqueIdMap::new(),
        }
    }

    async fn get_stats(&mut self) -> Result<MailboxStats> {
        let stats = self.session.stat().await?;

        let message_count = stats.counter();

        let stats = MailboxStats::new(0, message_count.value()?);

        Ok(stats)
    }

    /// Fetches the message count from the pop server and creates a new 'fake' mailbox.
    ///
    /// We do this because Pop does not support mailboxes.
    async fn get_inbox(&mut self) -> Result<Mailbox> {
        let stats = self.get_stats().await?;

        // Create default inbox with stats
        let mailbox: Mailbox = stats.into();

        Ok(mailbox)
    }

    async fn update_uidl_map(&mut self) -> Result<()> {
        let uidl = match self.session.uidl(None).await? {
            UidlResponse::Multiple(list) => list,
            _ => unreachable!(),
        };

        self.unique_id_map.extend(uidl.items())?;

        Ok(())
    }

    async fn get_index<T: AsRef<str>>(&mut self, unique_id: T) -> Result<usize> {
        if let Some(index) = self.unique_id_map.get(&unique_id) {
            return Ok(index);
        };

        self.update_uidl_map().await?;

        match self.unique_id_map.get(&unique_id) {
            Some(msg_number) => Ok(msg_number),
            None => err!(
                ErrorKind::MessageNotFound,
                "Could not find a message with id {}",
                unique_id.as_ref()
            ),
        }
    }
}

#[async_trait]
impl<S: Read + Write + Unpin + Send> IncomingProtocol for PopSession<S> {
    async fn send_keep_alive(&mut self) -> Result<()> {
        self.session.noop().await?;

        Ok(())
    }

    fn should_keep_alive(&self) -> bool {
        match self.session.last_activity() {
            Some(last_activity) => last_activity.elapsed() > ACTIVITY_TIMEOUT,
            None => false,
        }
    }

    async fn get_mailbox_list(&mut self) -> Result<Node<Mailbox>> {
        Ok(self.get_inbox().await?.into())
    }

    async fn get_mailbox(&mut self, _mailbox_id: &str) -> Result<Node<Mailbox>> {
        Ok(self.get_inbox().await?.into())
    }

    async fn logout(&mut self) -> Result<()> {
        self.unique_id_map.reset();

        self.session.quit().await?;

        Ok(())
    }

    async fn delete_mailbox(&mut self, _: &str) -> Result<()> {
        err!(
            ErrorKind::Unsupported,
            "Pop does not support deleting mailboxes",
        )
    }

    async fn rename_mailbox(&mut self, _: &str, _: &str) -> Result<()> {
        err!(
            ErrorKind::Unsupported,
            "Pop does not support renaming mailboxes",
        )
    }

    async fn create_mailbox(&mut self, _: &str) -> Result<()> {
        err!(
            ErrorKind::Unsupported,
            "Pop does not support creating mailboxes",
        )
    }

    async fn get_messages(&mut self, _: &str, start: usize, end: usize) -> Result<Vec<Preview>> {
        let total_messages = self.get_stats().await?.total();

        let sequence_start = if total_messages < end {
            1
        } else {
            total_messages.saturating_sub(end).saturating_add(1)
        };

        let sequence_end = if total_messages < start {
            1
        } else {
            total_messages.saturating_sub(start).saturating_add(1)
        };

        let msg_count = end.saturating_sub(start) as usize;

        let mut previews: Vec<Preview> = Vec::with_capacity(msg_count);

        for msg_number in sequence_start..sequence_end {
            let unique_id = match self.unique_id_map.get_id(msg_number) {
                Some(id) => id.to_string(),
                None => {
                    let uidl_response = self.session.uidl(Some(msg_number)).await?;

                    let unique_id = match &uidl_response {
                        UidlResponse::Multiple(_) => unreachable!(),
                        UidlResponse::Single(item) => item.id(),
                    };

                    unique_id.value()?
                }
            };

            let body = self.session.top(msg_number, 0).await?;

            let mut flags = vec![Flag::Read];

            // If we have marked a message as deleted, we will add the corresponding flag
            if self.session.is_deleted(&msg_number) {
                flags.push(Flag::Deleted)
            }

            let builder: MessageBuilder = body.as_ref().try_into()?;

            let preview: Preview = builder.flags(flags).id(&unique_id).build()?;

            previews.push(preview)
        }

        Ok(previews)
    }

    async fn get_message(&mut self, _box_id: &str, message_id: &str) -> Result<Message> {
        let msg_number = self.get_index(message_id).await?;

        let body = self.session.retr(msg_number).await?;

        let mut flags = vec![Flag::Read];

        // If we have marked a message as deleted, we will add the corresponding flag
        if self.session.is_deleted(&msg_number) {
            flags.push(Flag::Deleted)
        }

        let builder: MessageBuilder = body.as_ref().try_into()?;

        let message: Message = builder.flags(flags).id(message_id).build()?;

        Ok(message)
    }
}

#[cfg(test)]
mod test {

    use super::*;

    use dotenv::dotenv;
    use std::env;

    async fn create_test_session() -> PopSession<TlsStream<TcpStream>> {
        dotenv().ok();

        let username = env::var("POP_USERNAME").unwrap();
        let password = env::var("POP_PASSWORD").unwrap();

        let server = env::var("POP_SERVER").unwrap();
        let port: u16 = 995;

        let client = super::connect(server, port).await.unwrap();

        let session = client.login(&username, &password).await.unwrap();

        session
    }

    #[cfg_attr(feature = "runtime-async-std", async_std::test)]
    #[cfg_attr(feature = "runtime-tokio", tokio::test)]
    async fn get_messages() {
        let mut session = create_test_session().await;

        let previews = session.get_messages("Inbox", 0, 10).await.unwrap();

        for preview in previews.iter() {
            println!("{:?}", preview);
        }
    }

    #[cfg_attr(feature = "runtime-async-std", async_std::test)]
    #[cfg_attr(feature = "runtime-tokio", tokio::test)]
    async fn get_message() {
        let mut session = create_test_session().await;

        let message = session.get_message("Inbox", "1").await.unwrap();

        println!("{:?}", message.to());
    }
}
