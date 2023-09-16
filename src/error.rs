use std::{error, fmt, num::ParseIntError, result};

#[cfg(feature = "pop")]
use async_pop::error::Error as PopError;

#[cfg(feature = "imap")]
use async_imap::error::Error as ImapError;

#[cfg(feature = "smtp")]
use async_smtp::error::Error as SmtpError;

use async_native_tls::Error as TlsError;

use chrono::ParseError as ParseTimeError;

use email::results::ParsingError as AddressParseError;

use mailparse::MailParseError;

#[cfg(feature = "runtime-tokio")]
use tokio::{io::Error as IoError, task::JoinError, time::error::Elapsed};

#[cfg(feature = "runtime-async-std")]
use async_std::io::Error as IoError;

macro_rules! impl_from_error {
    ($error_type:ty, $error_kind:expr, $error_msg:expr) => {
        impl From<$error_type> for Error {
            fn from(err: $error_type) -> Self {
                Error::new($error_kind(err), $error_msg)
            }
        }
    };
}

macro_rules! err {
    ($kind:expr, $($arg:tt)*) => {{
		use crate::error::Error;

        let kind = $kind;
        let message = format!($($arg)*);
        return Err(Error::new( kind, message ));
    }};
}

#[derive(Debug)]
pub enum ErrorKind {
    MessageNotFound,
    /// The server responded with some unexpected data.
    UnexpectedBehavior,
    /// The requested feature/function is unsupported for this client type.
    Unsupported,
    Io(IoError),
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
    ParseInt(ParseIntError),
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

    ParseEmailAddress(AddressParseError),
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
            #[cfg(feature = "pop")]
            ErrorKind::Pop(e) => e.source(),
            #[cfg(feature = "imap")]
            ErrorKind::Imap(e) => e.source(),
            ErrorKind::Io(e) => e.source(),
            ErrorKind::Tls(e) => e.source(),
            ErrorKind::ParseMessage(e) => e.source(),
            _ => None,
        }
    }
}

#[cfg(feature = "pop")]
impl_from_error!(PopError, |err| ErrorKind::Pop(err), "Error from pop server");
#[cfg(feature = "imap")]
impl_from_error!(
    ImapError,
    |err| ErrorKind::Imap(err),
    "Error from imap server"
);
#[cfg(feature = "smtp")]
impl_from_error!(
    SmtpError,
    |err| ErrorKind::Smtp(err),
    "Error from smtp server"
);
impl_from_error!(
    TlsError,
    |err| ErrorKind::Tls(err),
    "Error creating a secure connection"
);
impl_from_error!(IoError, |err| ErrorKind::Io(err), "Io operation failed");
impl_from_error!(
    ParseTimeError,
    |err| ErrorKind::ParseTime(err),
    "Failed to parse date time"
);
impl_from_error!(
    MailParseError,
    |err| ErrorKind::ParseMessage(err),
    "Failed to parse mail message"
);
impl_from_error!(
    AddressParseError,
    |err| ErrorKind::ParseEmailAddress(err),
    "Failed to parse email address"
);
impl_from_error!(
    ParseIntError,
    |err| ErrorKind::ParseInt(err),
    "Failed to parse integer value from string"
);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub(crate) use err;

pub type Result<T> = result::Result<T, Error>;
