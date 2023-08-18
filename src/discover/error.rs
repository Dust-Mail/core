use std::{error, fmt::Display, result};

#[cfg(feature = "autoconfig")]
use autoconfig::error::Error as AutoconfigError;
#[cfg(feature = "autodiscover")]
use ms_autodiscover::error::Error as AutodiscoverError;

use dns_mail_discover::error::Error as DnsDiscoverError;

#[derive(Debug)]
pub enum ErrorKind {
    InvalidEmailAddress,
    InvalidConfig,
    NotFound(Vec<Error>),
    DnsDiscover(DnsDiscoverError),
    #[cfg(feature = "autoconfig")]
    Autoconfig(AutoconfigError),
    #[cfg(feature = "autodiscover")]
    Autodiscover(AutodiscoverError),
}

#[derive(Debug)]
pub struct Error {
    message: String,
    kind: ErrorKind,
}

impl Error {
    pub fn new<M: Into<String>>(kind: ErrorKind, message: M) -> Self {
        Self {
            message: message.into(),
            kind,
        }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error: {}", self.message)
    }
}

#[cfg(feature = "autoconfig")]
impl From<AutoconfigError> for Error {
    fn from(autoconfig_error: AutoconfigError) -> Self {
        Error::new(
            ErrorKind::Autoconfig(autoconfig_error),
            "Failed to retrieve config using autoconfig protocol",
        )
    }
}

#[cfg(feature = "autodiscover")]
impl From<AutodiscoverError> for Error {
    fn from(error: AutodiscoverError) -> Self {
        Error::new(
            ErrorKind::Autodiscover(error),
            "Failed to retrieve config using autodiscover protocol",
        )
    }
}

impl From<DnsDiscoverError> for Error {
    fn from(error: DnsDiscoverError) -> Self {
        Error::new(
            ErrorKind::DnsDiscover(error),
            "Failed to retrieve config using dns mail discover protocol",
        )
    }
}

#[macro_export]
macro_rules! failed {
    ($kind:expr, $($arg:tt)*) => {{
		use crate::discover::error::Error;

        let kind = $kind;
        let message = format!($($arg)*);
        return Err(Error::new( kind, message ));
    }};
}

pub type Result<T> = result::Result<T, Error>;
