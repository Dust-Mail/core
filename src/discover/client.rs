use super::{Config, Result};

pub struct Client {}

impl Client {
    #[cfg(feature = "autoconfig")]
    pub async fn from_autoconfig<D: AsRef<str>>(domain: D) -> Result<Config> {
        use super::parse::AutoConfigParser;

        let autoconfig = autoconfig::from_domain(domain).await?;

        let config = AutoConfigParser::parse(autoconfig);

        Ok(config)
    }

    #[cfg(feature = "autodiscover")]
    pub async fn from_autodiscover<E: AsRef<str>, P: AsRef<str>>(
        email: E,
        password: Option<P>,
    ) -> Result<Config> {
        use super::parse::AutodiscoverParser;

        let autodiscover = ms_autodiscover::from_email(email, password, None::<String>).await?;

        let config = AutodiscoverParser::parse(autodiscover)?;

        Ok(config)
    }

    pub async fn from_dns<D: AsRef<str>>(domain: D) -> Result<Config> {
        use super::parse::DnsDiscoverParser;

        let servers = dns_mail_discover::from_domain(domain).await?;

        let config = DnsDiscoverParser::parse(servers);

        Ok(config)
    }
}
