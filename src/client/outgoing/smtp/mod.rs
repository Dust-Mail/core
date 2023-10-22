use crate::{
    client::{
        connection::ConnectionSecurity,
        protocol::{OutgoingProtocol, SmtpCredentials},
        Credentials, ServerCredentials,
    },
    error::Result,
    runtime::{
        io::{BufRead, BufStream, Write},
        net::TcpStream,
    },
};

use async_native_tls::{TlsConnector, TlsStream};
use async_smtp::{self, authentication::Mechanism, SmtpTransport};
use async_trait::async_trait;

use super::types::sendable::SendableMessage;

pub struct SmtpClient {
    credentials: SmtpCredentials,
}

impl SmtpClient {
    pub fn new(credentials: SmtpCredentials) -> Self {
        Self { credentials }
    }
}

async fn connect<S: AsRef<str>, P: Into<u16>>(
    server: S,
    port: P,
) -> Result<SmtpTransport<BufStream<TlsStream<TcpStream>>>> {
    let tls = TlsConnector::new();

    let tcp_stream = TcpStream::connect((server.as_ref(), port.into())).await?;

    let tls_stream = tls.connect(server.as_ref(), tcp_stream).await?;

    let buf_stream = BufStream::new(tls_stream);

    let client = async_smtp::SmtpClient::new();

    let transport = SmtpTransport::new(client, buf_stream).await?;

    Ok(transport)
}

async fn connect_plain<S: AsRef<str>, P: Into<u16>>(
    server: S,
    port: P,
) -> Result<SmtpTransport<BufStream<TcpStream>>> {
    let stream = TcpStream::connect((server.as_ref(), port.into())).await?;

    let buf_stream = BufStream::new(stream);

    let client = async_smtp::SmtpClient::new();

    let transport = SmtpTransport::new(client, buf_stream).await?;

    Ok(transport)
}

async fn send<S: BufRead + Write + Unpin>(
    mut transport: SmtpTransport<S>,
    message: SendableMessage,
) -> Result<()> {
    transport.send(message.try_into()?).await?;

    transport.quit().await?;

    Ok(())
}

const PASSWORD_MECHANISMS: [Mechanism; 2] = [Mechanism::Plain, Mechanism::Login];
const OAUTH_MECHANISMS: [Mechanism; 1] = [Mechanism::Xoauth2];

async fn login<S: BufRead + Write + Unpin>(
    transport: &mut SmtpTransport<S>,
    creds: &Credentials,
) -> Result<()> {
    match creds {
        Credentials::Password { username, password } => {
            let smtp_credentials =
                async_smtp::authentication::Credentials::new(username.clone(), password.clone());

            transport
                .try_login(&smtp_credentials, &PASSWORD_MECHANISMS)
                .await?;
        }
        Credentials::OAuth { username, token } => {
            let smtp_credentials =
                async_smtp::authentication::Credentials::new(username.clone(), token.clone());

            transport
                .try_login(&smtp_credentials, &OAUTH_MECHANISMS)
                .await?;
        }
    }

    Ok(())
}

#[async_trait]
impl OutgoingProtocol for SmtpClient {
    async fn send_message(&mut self, message: SendableMessage) -> Result<()> {
        match self.credentials.server().security() {
            ConnectionSecurity::Tls => {
                let mut transport = connect(
                    self.credentials.server().domain(),
                    self.credentials.server().port(),
                )
                .await?;

                login(&mut transport, self.credentials.credentials()).await?;

                send(transport, message).await
            }
            _ => {
                let mut transport = connect_plain(
                    self.credentials.server().domain(),
                    self.credentials.server().port(),
                )
                .await?;

                login(&mut transport, self.credentials.credentials()).await?;

                send(transport, message).await
            }
        }
    }
}

pub fn create(credentials: SmtpCredentials) -> Result<Box<dyn OutgoingProtocol + Sync + Send>> {
    let client = SmtpClient::new(credentials);

    Ok(Box::new(client))
}

// #[cfg(test)]
// mod test {
//     use std::env;

//     use dotenv::dotenv;

//     use crate::client::{builder::MessageBuilder, protocol::RemoteServer};

//     use super::*;

//     async fn create_test_session() -> Box<dyn OutgoingProtocol + Send + Sync> {
//         dotenv().ok();

//         let server = RemoteServer::new(
//             env::var("SMTP_SERVER").unwrap(),
//             465,
//             ConnectionSecurity::Tls,
//         );

//         let creds = Credentials::password(
//             env::var("SMTP_USERNAME").unwrap(),
//             env::var("SMTP_PASSWORD").unwrap(),
//         );

//         let smtp_creds = SmtpCredentials::new(server, creds);

//         let client = create(smtp_creds).unwrap();

//         client
//     }
// }
