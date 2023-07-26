mod parse;

use std::collections::HashMap;

use async_native_tls::{TlsConnector, TlsStream};
use async_pop::types::UniqueIDResponse;
use async_trait::async_trait;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};

use crate::{
    client::protocol::{Credentials, IncomingProtocol, PopCredentials, ServerCredentials},
    parse::{parse_headers, parse_rfc822},
    types::{
        ConnectionSecurity, Error, ErrorKind, Flag, MailBox, MailBoxList, Message, MessageCounts,
        Preview, Result,
    },
};

use parse::parse_address;

use self::parse::parse_preview_from_headers;

const MAILBOX_DEFAULT_NAME: &str = "Inbox";

pub struct PopClient<S: AsyncRead + AsyncWrite + Unpin> {
    session: async_pop::Client<S>,
}

impl<S: AsyncRead + AsyncWrite + Unpin> PopClient<S> {
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
        _: U,
        token: T,
    ) -> Result<PopSession<S>> {
        self.session.auth(token).await?;

        let session = PopSession::new(self.session);

        Ok(session)
    }
}

pub struct PopSession<S: AsyncRead + AsyncWrite + Unpin> {
    session: async_pop::Client<S>,
    mailbox_list: MailBoxList,
    unique_id_map: HashMap<String, u32>,
}

pub async fn connect<S: AsRef<str>, P: Into<u16>>(
    server: S,
    port: P,
) -> Result<PopClient<TlsStream<TcpStream>>> {
    let tls = TlsConnector::new();

    let session =
        async_pop::connect((server.as_ref(), port.into()), server.as_ref(), &tls, None).await?;

    Ok(PopClient { session })
}

pub async fn connect_plain<S: AsRef<str>, P: Into<u16>>(
    server: S,
    port: P,
) -> Result<PopClient<TcpStream>> {
    let session = async_pop::connect_plain((server.as_ref(), port.into()), None).await?;

    Ok(PopClient { session })
}

async fn login<S: AsyncRead + AsyncWrite + Unpin>(
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

impl<S: AsyncRead + AsyncWrite + Unpin> PopSession<S> {
    pub fn new(session: async_pop::Client<S>) -> Self {
        Self {
            session,
            mailbox_list: MailBoxList::new(Vec::new()),
            unique_id_map: HashMap::new(),
        }
    }

    fn get_session_mut(&mut self) -> &mut async_pop::Client<S> {
        &mut self.session
    }

    /// Fetches the message count from the pop server and creates a new 'fake' mailbox.
    ///
    /// We do this because Pop does not support mailboxes.
    async fn create_default_box(&mut self) -> Result<MailBox> {
        let session = self.get_session_mut();

        let stats = session.stat().await?;

        let message_count = stats.messsage_count();

        let box_name = MAILBOX_DEFAULT_NAME;

        let counts = MessageCounts::new(0, *message_count as usize);

        let mailbox = MailBox::new(Some(counts), None, Vec::new(), true, box_name, box_name);

        Ok(mailbox)
    }

    async fn get_msg_number_from_msg_id<T: AsRef<str>>(&mut self, msg_id: T) -> Result<u32> {
        match self.unique_id_map.get(msg_id.as_ref()) {
            Some(msg_number) => return Ok(msg_number.clone()),
            None => {}
        };

        let session = self.get_session_mut();

        let unique_id_vec = match session.uidl(None).await? {
            UniqueIDResponse::UniqueID(_) => {
                // We gave the function a 'None' so it should never return this
                unreachable!()
            }
            UniqueIDResponse::UniqueIDList(list) => list,
        };

        self.unique_id_map = unique_id_vec
            .into_iter()
            .map(|list| (list.unique_id().to_string(), list.count().to_owned()))
            .collect();

        match self.unique_id_map.get(msg_id.as_ref()) {
            Some(msg_number) => Ok(msg_number.clone()),
            None => Err(Error::new(
                ErrorKind::UnexpectedBehavior,
                format!("Could not find a message with id {}", msg_id.as_ref()),
            )),
        }
    }
}

#[async_trait]
impl<S: AsyncRead + AsyncWrite + Unpin + Send> IncomingProtocol for PopSession<S> {
    async fn send_keep_alive(&mut self) -> Result<()> {
        self.session.noop().await?;

        Ok(())
    }

    fn should_keep_alive(&mut self) -> bool {
        match self.session.is_expiring() {
            Ok(is_expiring) => is_expiring,
            Err(_) => false,
        }
    }

    async fn get_mailbox_list(&mut self) -> Result<&MailBoxList> {
        self.mailbox_list = MailBoxList::new(vec![self.create_default_box().await?]);

        Ok(&self.mailbox_list)
    }

    async fn get_mailbox(&mut self, mailbox_id: &str) -> Result<&MailBox> {
        if mailbox_id != MAILBOX_DEFAULT_NAME {
            return Err(Error::new(
                ErrorKind::MailBoxNotFound,
                format!("Could not find a mailbox with id {}", mailbox_id),
            ));
        }

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
        self.session.quit().await?;

        Ok(())
    }

    async fn delete_mailbox(&mut self, _: &str) -> Result<()> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "Pop does not support deleting mailboxes",
        ))
    }

    async fn rename_mailbox(&mut self, _: &str, _: &str) -> Result<()> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "Pop does not support renaming mailboxes",
        ))
    }

    async fn create_mailbox(&mut self, _: &str) -> Result<()> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "Pop does not support creating mailboxes",
        ))
    }

    async fn get_messages(&mut self, _: &str, start: usize, end: usize) -> Result<Vec<Preview>> {
        let mailbox = self.create_default_box().await?;

        let total_messages = mailbox.counts().unwrap().total();

        let session = self.get_session_mut();

        let sequence_start = if total_messages < &end {
            1
        } else {
            total_messages.saturating_sub(end).saturating_add(1)
        };

        let sequence_end = if total_messages < &start {
            1
        } else {
            total_messages.saturating_sub(start).saturating_add(1)
        };

        let msg_count = end.saturating_sub(start) as usize;

        let mut previews: Vec<Preview> = Vec::with_capacity(msg_count);

        let mut unique_id_map = HashMap::new();

        for msg_number in sequence_start..sequence_end {
            let uidl_response = session.uidl(Some(msg_number as u32)).await?;

            let unique_id = match uidl_response {
                UniqueIDResponse::UniqueIDList(list) => {
                    let first = list.first().ok_or(Error::new(
                        ErrorKind::UnexpectedBehavior,
                        "Missing unique id for message",
                    ))?;

                    first.unique_id().to_string()
                }
                UniqueIDResponse::UniqueID(item) => item.unique_id().to_string(),
            };

            let header_bytes = session.top(msg_number as u32, 0).await?;

            let headers = parse_headers(&header_bytes)?;

            let (from, mut flags, sent, subject) = parse_preview_from_headers(&headers)?;

            // If we have marked a message as deleted, we will add the corresponding flag
            if session.is_deleted(&(msg_number as u32)) {
                flags.push(Flag::Deleted)
            }

            let preview = Preview::new(from, flags, &unique_id, sent, subject);

            // Add the unique id to the local map so we don't have to retrieve the entire list of unique id's later
            // just to get this message's msg_number.
            unique_id_map.insert(unique_id, msg_number as u32);

            previews.push(preview)
        }

        self.unique_id_map.extend(unique_id_map);

        Ok(previews)
    }

    async fn get_message(&mut self, _box_id: &str, msg_id: &str) -> Result<Message> {
        let msg_number = self.get_msg_number_from_msg_id(msg_id).await?;

        let session = self.get_session_mut();

        let message_bytes = session.retr(msg_number).await?;

        let content = parse_rfc822(&message_bytes)?;

        let headers = parse_headers(&message_bytes)?;

        let (from, mut flags, sent, subject) = parse_preview_from_headers(&headers)?;

        // If we have marked a message as deleted, we will add the corresponding flag
        if session.is_deleted(&msg_number) {
            flags.push(Flag::Deleted)
        }

        let to = match headers.get("To") {
            Some(to) => parse_address(to),
            None => Vec::new(),
        };

        let cc = match headers.get("CC") {
            Some(cc) => parse_address(cc),
            None => Vec::new(),
        };

        let bcc = match headers.get("BCC") {
            Some(bcc) => parse_address(bcc),
            None => Vec::new(),
        };

        let message = Message::new(
            from, to, cc, bcc, headers, flags, msg_id, sent, subject, content,
        );

        Ok(message)
    }
}

#[cfg(test)]
mod test {

    use super::{IncomingProtocol, PopSession};

    use async_native_tls::TlsStream;
    use dotenv::dotenv;
    use std::env;
    use tokio::net::TcpStream;

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

    #[tokio::test]
    async fn get_messages() {
        let mut session = create_test_session().await;

        let previews = session.get_messages("Inbox", 0, 10).await.unwrap();

        for preview in previews.iter() {
            println!(
                "{}: {:?}, \"{}\"",
                preview.id(),
                preview.sent(),
                preview.subject().unwrap()
            );
        }
    }

    #[tokio::test]
    async fn get_message() {
        let mut session = create_test_session().await;

        let message = session.get_message("Inbox", "17812").await.unwrap();

        println!("{:?}", message.to());
    }
}
