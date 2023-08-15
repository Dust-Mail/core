use std::{error, fmt, result};

#[cfg(feature = "pop")]
use async_pop::types::Error as PopError;

#[cfg(feature = "imap")]
use async_imap::error::Error as ImapError;

#[cfg(feature = "smtp")]
use async_smtp::error::Error as SmtpError;

use async_native_tls::Error as TlsError;

use chrono::ParseError as ParseTimeError;

use mailparse::MailParseError;
use tokio::{task::JoinError, time::error::Elapsed};

#[derive(Debug)]
pub enum ErrorKind {
    MessageNotFound,
    /// The server responded with some unexpected data.
    UnexpectedBehavior,
    /// The requested feature/function is unsupported for this client type.
    Unsupported,
    Io(tokio::io::Error),
    #[cfg(feature = "imap")]
    /// An error from the Imap server.
    Imap(ImapError),
    #[cfg(feature = "pop")]
    /// An error from the Pop server.
    Pop(PopError),
    #[cfg(feature = "smtp")]
    Smtp(SmtpError),
    Tls(TlsError),
    /// Failed to parse a date/time from the server.
    ParseTime(ParseTimeError),
    Timeout(Elapsed),
    /// Failed to parse a string given by the server.
    ParseString,
    /// Failed to parse a socket address which is used to connect to the remote mail server
    ParseAddress,
    /// Failed to parse provided login config.
    InvalidLoginConfig,
    /// Failed to parse mail message.
    ParseMessage(MailParseError),
    InvalidMessage,
    /// Error from the remote mail server.
    MailServer,
    /// Failed to serialize the given data to JSON.
    SerializeJSON,
    /// Could not detect a config from the given email address.
    ConfigNotFound,
    SpawnAsync,
    MailBoxNotFound,
    NoClientAvailable,
}

#[derive(Debug)]
pub struct Error {
    message: String,
    kind: ErrorKind,
}

impl Error {
    pub fn new<S: Into<String>>(kind: ErrorKind, msg: S) -> Self {
        Self {
            message: msg.into(),
            kind,
        }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self.kind() {
            ErrorKind::Pop(e) => e.source(),
            ErrorKind::Imap(e) => e.source(),
            ErrorKind::Io(e) => e.source(),
            ErrorKind::Tls(e) => e.source(),
            ErrorKind::ParseMessage(e) => e.source(),
            _ => None,
        }
    }
}

#[cfg(feature = "pop")]
impl From<PopError> for Error {
    fn from(pop_error: PopError) -> Self {
        Self::new(ErrorKind::Pop(pop_error), "Error from pop server")
    }
}

#[cfg(feature = "imap")]
impl From<ImapError> for Error {
    fn from(imap_error: ImapError) -> Self {
        Self::new(
            ErrorKind::Imap(imap_error),
            format!("Error from imap server"),
        )
    }
}

#[cfg(feature = "smtp")]
impl From<SmtpError> for Error {
    fn from(smtp_error: SmtpError) -> Self {
        Self::new(
            ErrorKind::Smtp(smtp_error),
            format!("Error from smtp server"),
        )
    }
}

impl From<JoinError> for Error {
    fn from(join_error: JoinError) -> Self {
        Self::new(
            ErrorKind::SpawnAsync,
            format!("Failed to spawn async task: {}", join_error),
        )
    }
}

impl From<TlsError> for Error {
    fn from(native_tls_error: TlsError) -> Self {
        Error::new(
            ErrorKind::Tls(native_tls_error),
            format!("Error creating a secure connection"),
        )
    }
}

impl From<tokio::io::Error> for Error {
    fn from(io_error: tokio::io::Error) -> Self {
        Error::new(ErrorKind::Io(io_error), "Error with io")
    }
}

impl From<ParseTimeError> for Error {
    fn from(chrono_error: ParseTimeError) -> Self {
        Error::new(
            ErrorKind::ParseTime(chrono_error),
            "Failed to parse date time",
        )
    }
}

impl From<Elapsed> for Error {
    fn from(timeout_error: Elapsed) -> Self {
        Error::new(ErrorKind::Timeout(timeout_error), "Timeout error")
    }
}

impl From<MailParseError> for Error {
    fn from(mailparse_error: MailParseError) -> Self {
        Error::new(
            ErrorKind::ParseMessage(mailparse_error),
            "Failed to parse mail message",
        )
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[macro_export]
macro_rules! failed {
    ($kind:expr, $($arg:tt)*) => {{
		use crate::error::Error;

        let kind = $kind;
        let message = format!($($arg)*);
        return Err(Error::new( kind, message ));
    }};
}

pub type Result<T> = result::Result<T, Error>;
