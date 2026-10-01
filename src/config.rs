//! Configuration from environment variables.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

pub const DEFAULT_PORT: u16 = 8080;
pub const DEFAULT_BIND_ADDR: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub bind_addr: IpAddr,
    pub port: u16,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    InvalidPort(String),
    InvalidBindAddr(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPort(v) => write!(f, "PORT must be a number from 1 to 65535, got '{v}'"),
            Self::InvalidBindAddr(v) => {
                write!(
                    f,
                    "BIND_ADDR must be an IP address like 0.0.0.0 or ::1, got '{v}'"
                )
            }
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let port = match lookup("PORT").filter(|v| !v.trim().is_empty()) {
            None => DEFAULT_PORT,
            Some(v) => match v.trim().parse::<u16>() {
                Ok(p) if p != 0 => p,
                _ => return Err(ConfigError::InvalidPort(v)),
            },
        };
        let bind_addr = match lookup("BIND_ADDR").filter(|v| !v.trim().is_empty()) {
            None => DEFAULT_BIND_ADDR,
            Some(v) => v
                .trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .parse()
                .map_err(|_| ConfigError::InvalidBindAddr(v))?,
        };
        Ok(Self { bind_addr, port })
    }

    pub fn listen_addr(&self) -> SocketAddr {
        SocketAddr::new(self.bind_addr, self.port)
    }

    /// Address the `healthcheck` command connects to: loopback when bound to
    /// all interfaces, otherwise the specific bind address.
    pub fn probe_addr(&self) -> SocketAddr {
        let ip = match self.bind_addr {
            IpAddr::V4(ip) if ip.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(ip) if ip.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
            ip => ip,
        };
        SocketAddr::new(ip, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn parse(vars: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::from_lookup(|key| map.get(key).cloned())
    }

    #[test]
    fn defaults() {
        let config = parse(&[]).unwrap();
        assert_eq!(config.listen_addr(), "0.0.0.0:8080".parse().unwrap());
        assert_eq!(config.probe_addr(), "127.0.0.1:8080".parse().unwrap());
    }

    #[test]
    fn custom_values() {
        let config = parse(&[("PORT", "3000"), ("BIND_ADDR", "[::]")]).unwrap();
        assert_eq!(config.listen_addr(), "[::]:3000".parse().unwrap());
        assert_eq!(config.probe_addr(), "[::1]:3000".parse().unwrap());

        let config = parse(&[("BIND_ADDR", "192.168.1.5")]).unwrap();
        assert_eq!(config.probe_addr(), "192.168.1.5:8080".parse().unwrap());
    }

    #[test]
    fn invalid_values() {
        assert!(matches!(
            parse(&[("PORT", "0")]),
            Err(ConfigError::InvalidPort(_))
        ));
        assert!(matches!(
            parse(&[("PORT", "http")]),
            Err(ConfigError::InvalidPort(_))
        ));
        assert!(matches!(
            parse(&[("PORT", "70000")]),
            Err(ConfigError::InvalidPort(_))
        ));
        assert!(matches!(
            parse(&[("BIND_ADDR", "localhost")]),
            Err(ConfigError::InvalidBindAddr(_))
        ));
    }
}
