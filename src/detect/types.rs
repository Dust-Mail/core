use serde::Serialize;
use tokio::net::ToSocketAddrs;

use crate::{
    parse::to_json,
    types::{ConnectionSecurity, Result},
};

#[derive(Debug, Serialize, Clone, PartialEq, Eq, Hash)]
pub enum ServerConfigType {
    Imap,
    Pop,
    Smtp,
    Exchange,
}

impl ServerConfigType {
    pub fn is_outgoing(&self) -> bool {
        match self {
            Self::Smtp => true,
            _ => false,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    r#type: ServerConfigType,
    port: u16,
    domain: String,
    security: ConnectionSecurity,
    auth_type: Vec<AuthenticationType>,
}

impl ServerConfig {
    pub fn new<S: Into<String>>(
        r#type: ServerConfigType,
        port: u16,
        domain: S,
        security: ConnectionSecurity,
        auth_type: Vec<AuthenticationType>,
    ) -> Self {
        Self {
            r#type,
            port,
            domain: domain.into(),
            security,
            auth_type,
        }
    }

    pub fn r#type(&self) -> &ServerConfigType {
        &self.r#type
    }

    pub fn port(&self) -> &u16 {
        &self.port
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub fn security(&self) -> &ConnectionSecurity {
        &self.security
    }

    pub fn auth_type(&self) -> &Vec<AuthenticationType> {
        &self.auth_type
    }
}

#[derive(Debug, Serialize, Clone)]
pub enum AuthenticationType {
    ClearText,
    Encrypted,
    OAuth2,
    None,
    Unknown,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OAuth2Config {
    token_url: String,
    oauth_url: String,
    scopes: Vec<String>,
}

impl OAuth2Config {
    pub fn new<S: Into<String>>(token_url: S, oauth_url: S, scopes: Vec<S>) -> Self {
        let scopes = scopes.into_iter().map(|scope| scope.into()).collect();

        Self {
            oauth_url: oauth_url.into(),
            token_url: token_url.into(),
            scopes,
        }
    }
    pub fn oauth_url(&self) -> &str {
        &self.oauth_url
    }

    pub fn token_url(&self) -> &str {
        &self.token_url
    }

    pub fn scopes(&self) -> &Vec<String> {
        &self.scopes
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum ConfigType {
    MultiServer {
        incoming: Vec<ServerConfig>,
        outgoing: Vec<ServerConfig>,
    },
}

impl ConfigType {
    pub fn new_multiserver(incoming: Vec<ServerConfig>, outgoing: Vec<ServerConfig>) -> Self {
        ConfigType::MultiServer { incoming, outgoing }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    r#type: ConfigType,
    provider: String,
    oauth2: Option<OAuth2Config>,
    display_name: Option<String>,
}

impl Config {
    pub fn new<S: Into<String>>(
        r#type: ConfigType,
        provider: S,
        oauth2_config: Option<OAuth2Config>,
        display_name: Option<String>,
    ) -> Self {
        Self {
            display_name,
            oauth2: oauth2_config,
            provider: provider.into(),
            r#type,
        }
    }

    pub fn oauth2(&self) -> &Option<OAuth2Config> {
        &self.oauth2
    }

    /// The kind of config
    pub fn config_type(&self) -> &ConfigType {
        &self.r#type
    }

    /// The email provider name
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// The display name for the email provider
    pub fn display_name(&self) -> &Option<String> {
        &self.display_name
    }

    pub fn to_json(&self) -> Result<String> {
        to_json(self)
    }
}

pub struct Socket {
    domain: String,
    port: u16,
    security: ConnectionSecurity,
}

impl Socket {
    pub fn new<D: Into<String>>(domain: D, port: u16, security: ConnectionSecurity) -> Self {
        Self {
            domain: domain.into(),
            port,
            security,
        }
    }

    pub fn from_tuple<S: AsRef<str>>(tuple: (S, u16, ConnectionSecurity)) -> Self {
        Self::new(tuple.0.as_ref(), tuple.1, tuple.2)
    }

    pub fn addr(&self) -> impl ToSocketAddrs + '_ {
        (self.domain.as_ref(), self.port.clone())
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub fn into_server_config(self, config_type: ServerConfigType) -> ServerConfig {
        ServerConfig::new(
            config_type,
            self.port,
            self.domain,
            self.security,
            vec![AuthenticationType::Encrypted],
        )
    }

    pub fn security(&self) -> &ConnectionSecurity {
        &self.security
    }
}
