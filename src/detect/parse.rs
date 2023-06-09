#[cfg(feature = "autoconfig")]
use autoconfig::{
    self,
    types::config::{
        AuthenticationType as AutoConfigAuthenticationType, Config as AutoConfig,
        OAuth2Config as AutoConfigOAuth2Config, SecurityType as AutoConfigSecurityType, Server,
        ServerType as AutoConfigServerType,
    },
};

use crate::types::{ConnectionSecurity, Result};

use super::{
    types::OAuth2Config, AuthenticationType, Config, ConfigType, ServerConfig, ServerConfigType,
};

#[cfg(feature = "autoconfig")]
pub struct AutoConfigParser;

#[cfg(feature = "autoconfig")]
impl AutoConfigParser {
    fn map_server_to_server_config(server: &Server) -> Option<ServerConfig> {
        {
            let domain: String = server.hostname()?.into();

            let port: u16 = server.port().cloned()?;

            let security: ConnectionSecurity = match server.security_type() {
                Some(security) => match security {
                    AutoConfigSecurityType::Plain => ConnectionSecurity::Plain,
                    AutoConfigSecurityType::Starttls => ConnectionSecurity::StartTls,
                    AutoConfigSecurityType::Tls => ConnectionSecurity::Tls,
                },
                None => return None,
            };

            let auth_type: Vec<AuthenticationType> = server
                .authentication_type()
                .iter()
                .map(|authentication_type| match authentication_type {
                    AutoConfigAuthenticationType::None => AuthenticationType::None,
                    AutoConfigAuthenticationType::PasswordCleartext => {
                        AuthenticationType::ClearText
                    }
                    AutoConfigAuthenticationType::PasswordEncrypted => {
                        AuthenticationType::Encrypted
                    }
                    AutoConfigAuthenticationType::OAuth2 => AuthenticationType::OAuth2,
                    _ => AuthenticationType::Unknown,
                })
                .collect();

            let server_type = match server.server_type() {
                AutoConfigServerType::Imap => ServerConfigType::Imap,
                AutoConfigServerType::Pop3 => ServerConfigType::Pop,
                AutoConfigServerType::Smtp => ServerConfigType::Smtp,
                AutoConfigServerType::Exchange => ServerConfigType::Exchange,
            };

            let server_config = ServerConfig::new(server_type, port, domain, security, auth_type);

            Some(server_config)
        }
    }

    fn parse_oauth2_config(config: &AutoConfigOAuth2Config) -> OAuth2Config {
        OAuth2Config::new(
            config.token_url().into(),
            config.auth_url().into(),
            config.scope(),
        )
    }

    pub fn parse(autoconfig: AutoConfig) -> Result<Config> {
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

        let oauth2_config = autoconfig.oauth2().map(Self::parse_oauth2_config);

        let config = Config::new(config_type, provider, oauth2_config, display_name);

        Ok(config)
    }
}
