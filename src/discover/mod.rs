use crate::{failed, validate::validate_email};

mod client;
pub mod config;
mod error;
mod parse;

use error::Result;
pub use error::{Error, ErrorKind};

use config::{AuthenticationType, ConfigType, ServerConfig, ServerConfigType};

use client::Client;

use self::config::Config;

const INVALID_EMAIL_MESSAGE: &str = "Invalid email address";

fn parse_domain<E: AsRef<str>>(email: E) -> Result<String> {
    if !validate_email(email.as_ref()) {
        failed!(ErrorKind::InvalidEmailAddress, "{}", INVALID_EMAIL_MESSAGE);
    };

    let mut email_split = email.as_ref().split('@');

    email_split.next();

    let domain = match email_split.next() {
        Some(domain) => domain,
        None => failed!(ErrorKind::InvalidEmailAddress, "{}", INVALID_EMAIL_MESSAGE),
    };

    Ok(domain.to_string())
}

/// Automatically detect an email providers config for a given email address
pub async fn from_email<E: AsRef<str>, P: AsRef<str>>(
    email: E,
    password: Option<P>,
) -> Result<Config> {
    let email = email.as_ref();
    let domain = parse_domain(email)?;

    let mut errors: Vec<_> = Vec::new();

    #[cfg(feature = "autoconfig")]
    {
        let result = Client::from_autoconfig(&domain).await;

        match result {
            Ok(config) => return Ok(config),
            Err(error) => errors.push(error),
        }
    }

    #[cfg(feature = "autodiscover")]
    {
        let result = Client::from_autodiscover(email, password).await;

        match result {
            Ok(config) => return Ok(config),
            Err(error) => errors.push(error),
        }
    }

    let result = Client::from_dns(domain).await;

    match result {
        Ok(config) => return Ok(config),
        Err(error) => errors.push(error),
    }

    Err(Error::new(
        ErrorKind::NotFound(errors),
        "Could not detect an email server config from the given email address",
    ))
}

mod test {
    #[tokio::test]
    async fn from_email() {
        let email = "example@gmail.com";

        let config = super::from_email(email, None::<String>).await.unwrap();

        println!("{}", config.to_json().unwrap());
    }
}
