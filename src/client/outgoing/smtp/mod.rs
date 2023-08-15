use crate::{
    client::protocol::{OutgoingProtocol, SmtpCredentials},
    error::Result,
    types::{ConnectionSecurity, Message},
};

use async_native_tls::{TlsConnector, TlsStream};
use async_smtp::{self, SmtpTransport};
use async_trait::async_trait;
use tokio::io::{AsyncBufRead, AsyncWrite, BufStream};
use tokio::net::TcpStream;

use self::parse::create_sendable_message;

mod parse;

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

async fn send<S: AsyncBufRead + AsyncWrite + Unpin>(
    mut transport: SmtpTransport<S>,
    message: Message,
) -> Result<()> {
    let email = create_sendable_message(message)?;

    transport.send(email).await?;

    transport.quit().await?;

    Ok(())
}

#[async_trait]
impl OutgoingProtocol for SmtpClient {
    async fn send_message(&mut self, message: Message) -> Result<()> {
        match self.credentials.server().security() {
            ConnectionSecurity::Tls => {
                let transport = connect(
                    self.credentials.server().domain(),
                    self.credentials.server().port(),
                )
                .await?;

                send(transport, message).await
            }
            _ => {
                let transport = connect_plain(
                    self.credentials.server().domain(),
                    self.credentials.server().port(),
                )
                .await?;

                send(transport, message).await
            }
        }
    }
}

pub fn create(credentials: SmtpCredentials) -> Result<Box<dyn OutgoingProtocol + Sync + Send>> {
    let client = SmtpClient::new(credentials);

    Ok(Box::new(client))
}
