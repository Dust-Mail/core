#[cfg(feature = "autoconfig")]
use autoconfig::{self, config};

#[cfg(feature = "autodiscover")]
use ms_autodiscover::types as autodiscover;

use crate::types::ConnectionSecurity;

use super::{
    config::OAuth2Config, error::Result, AuthenticationType, Config, ConfigType, ServerConfig,
    ServerConfigType,
};

#[cfg(feature = "autoconfig")]
pub struct AutoConfigParser;

#[cfg(feature = "autoconfig")]
impl AutoConfigParser {
    fn map_server_to_server_config(server: &config::Server) -> Option<ServerConfig> {
        {
            let domain: String = server.hostname()?.into();

            let port: u16 = server.port()?.clone();

            let security: ConnectionSecurity = match server.security_type() {
                Some(security) => match security {
                    config::SecurityType::Plain => ConnectionSecurity::Plain,
                    config::SecurityType::Starttls => ConnectionSecurity::StartTls,
                    config::SecurityType::Tls => ConnectionSecurity::Tls,
                },
                None => return None,
            };

            let auth_type: Vec<AuthenticationType> = server
                .authentication_type()
                .iter()
                .map(|authentication_type| match authentication_type {
                    config::AuthenticationType::None => AuthenticationType::None,
                    config::AuthenticationType::PasswordCleartext => AuthenticationType::ClearText,
                    config::AuthenticationType::PasswordEncrypted => AuthenticationType::Encrypted,
                    config::AuthenticationType::OAuth2 => AuthenticationType::OAuth2,
                    _ => AuthenticationType::Unknown,
                })
                .collect();

            let server_type = match server.server_type() {
                config::ServerType::Imap => ServerConfigType::Imap,
                config::ServerType::Pop3 => ServerConfigType::Pop,
                config::ServerType::Smtp => ServerConfigType::Smtp,
                config::ServerType::Exchange => ServerConfigType::Exchange,
            };

            let server_config = ServerConfig::new(server_type, port, domain, security, auth_type);

            Some(server_config)
        }
    }

    pub fn parse(autoconfig: config::Config) -> Config {
        let provider: String = autoconfig.email_provider().id().into();

        let display_name: Option<String> = autoconfig
            .email_provider()
            .display_name()
            .map(|name| name.to_string());

        let incoming: Vec<ServerConfig> = autoconfig
            .email_provider()
            .incoming_servers()
            .iter()
            .filter_map(|server| Self::map_server_to_server_config(server))
            .collect();

        let outgoing: Vec<ServerConfig> = autoconfig
            .email_provider()
            .outgoing_servers()
            .iter()
            .filter_map(|server| Self::map_server_to_server_config(server))
            .collect();

        let config_type = ConfigType::MultiServer { incoming, outgoing };

        let oauth2_config = autoconfig.oauth2().map(|config| {
            OAuth2Config::new(
                config.token_url().into(),
                config.auth_url().into(),
                config.scope(),
            )
        });

        Config::new(config_type, provider, oauth2_config, display_name)
    }
}

#[cfg(feature = "autodiscover")]
pub struct AutodiscoverParser;

#[cfg(feature = "autodiscover")]
impl AutodiscoverParser {
    fn account_to_server_config(account: &autodiscover::pox::Account) -> Option<ServerConfig> {
        use autodiscover::pox::Type::*;

        let protocol = account.protocol()?;

        let port = protocol.port()?.clone();
        let domain = protocol.server()?;
        let security = if protocol.ssl() {
            ConnectionSecurity::Tls
        } else {
            ConnectionSecurity::Plain
        };

        let auth_type = vec![AuthenticationType::ClearText];

        let protocol_type = protocol.r#type()?;

        match protocol_type {
            Imap => {
                let server_config =
                    ServerConfig::new(ServerConfigType::Imap, port, domain, security, auth_type);

                return Some(server_config);
            }
            Smtp => {
                let server_config =
                    ServerConfig::new(ServerConfigType::Smtp, port, domain, security, auth_type);

                return Some(server_config);
            }
            _ => None,
        }
    }

    pub fn parse(response: autodiscover::response::AutodiscoverResponse) -> Result<Config> {
        use autodiscover::response::AutodiscoverResponse::*;

        let mut providers: Vec<String> = Vec::new();
        let mut display_name = None;

        let mut incoming = Vec::new();
        let mut outgoing = Vec::new();

        match &response {
            Pox(response) => {
                if let Some(user) = response.user() {
                    display_name = user.display_name();
                }

                for account in response.account() {
                    if let Some(protocol) = account.protocol() {
                        if let Some(domain_name) = protocol.domain_name() {
                            providers.push(domain_name.to_string());
                        }
                    }

                    if let Some(server_config) = Self::account_to_server_config(account) {
                        if server_config.r#type().is_outgoing() {
                            outgoing.push(server_config)
                        } else {
                            incoming.push(server_config)
                        }
                    }
                }
            }
        };

        let config_type = ConfigType::MultiServer { incoming, outgoing };

        let provider = match providers.first() {
            Some(provider) => provider,
            None => "Unknown",
        };

        let config = Config::new(config_type, provider, None, display_name);

        Ok(config)
    }
}

pub struct DnsDiscoverParser;

impl DnsDiscoverParser {
    pub fn parse(servers: Vec<dns_mail_discover::server::Server>) -> Config {
        use dns_mail_discover::server::ServerType::*;

        let mut incoming = Vec::new();
        let mut outgoing = Vec::new();

        for server in servers {
            let security = if server.protocol().secure() {
                ConnectionSecurity::Tls
            } else {
                ConnectionSecurity::Plain
            };

            let port = server.port();
            let domain = server.domain();
            let auth_type = vec![AuthenticationType::ClearText];

            match server.protocol().r#type() {
                Imap => {
                    let server_config = ServerConfig::new(
                        ServerConfigType::Imap,
                        port,
                        domain,
                        security,
                        auth_type,
                    );

                    incoming.push(server_config)
                }
                Pop => {
                    let server_config =
                        ServerConfig::new(ServerConfigType::Pop, port, domain, security, auth_type);

                    incoming.push(server_config)
                }
                Smtp => {
                    let server_config = ServerConfig::new(
                        ServerConfigType::Smtp,
                        port,
                        domain,
                        security,
                        auth_type,
                    );

                    outgoing.push(server_config)
                }
            }
        }

        let provider = match incoming.first() {
            Some(server) => server.domain().to_string(),
            None => match outgoing.first() {
                Some(server) => server.domain().to_string(),
                None => String::from("Unknown"),
            },
        };

        let config_type = ConfigType::MultiServer { incoming, outgoing };

        Config::new(config_type, provider, None, None::<String>)
    }
}
