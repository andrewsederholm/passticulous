//! Configuration from environment variables.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

pub const DEFAULT_PORT: u16 = 8080;
pub const DEFAULT_BIND_ADDR: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);

/// Environment variables for theme colors and the CSS variable each sets.
pub const THEME_COLOR_VARS: [(&str, &str); 6] = [
    ("THEME_BG", "bg"),
    ("THEME_SURFACE", "surface"),
    ("THEME_TEXT", "text"),
    ("THEME_MUTED", "muted"),
    ("THEME_BORDER", "border"),
    ("THEME_ACCENT", "accent"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub bind_addr: IpAddr,
    pub port: u16,
    pub theme: Theme,
}

/// The default look for every visitor. Visitors can override it in the page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Theme {
    pub mode: ThemeMode,
    /// CSS variable names (without `--`) and `#rrggbb` colors, applied in
    /// both light and dark mode.
    pub colors: Vec<(&'static str, String)>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ThemeMode {
    /// Follow the visitor's device setting.
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        [Self::System, Self::Light, Self::Dark]
            .into_iter()
            .find(|m| m.name() == name)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    Port(String),
    BindAddr(String),
    Theme(String),
    Color(&'static str, String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Port(v) => write!(f, "PORT must be a number from 1 to 65535, got '{v}'"),
            Self::BindAddr(v) => {
                write!(
                    f,
                    "BIND_ADDR must be an IP address like 0.0.0.0 or ::1, got '{v}'"
                )
            }
            Self::Theme(v) => {
                write!(f, "THEME must be one of system, light or dark, got '{v}'")
            }
            Self::Color(key, v) => {
                write!(f, "{key} must be a hex color like #4f46e5, got '{v}'")
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
                _ => return Err(ConfigError::Port(v)),
            },
        };
        let bind_addr = match lookup("BIND_ADDR").filter(|v| !v.trim().is_empty()) {
            None => DEFAULT_BIND_ADDR,
            Some(v) => v
                .trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .parse()
                .map_err(|_| ConfigError::BindAddr(v))?,
        };
        let mode = match lookup("THEME").filter(|v| !v.trim().is_empty()) {
            None => ThemeMode::default(),
            Some(v) => {
                ThemeMode::from_name(&v.trim().to_ascii_lowercase()).ok_or(ConfigError::Theme(v))?
            }
        };
        let mut colors = Vec::new();
        for (key, var) in THEME_COLOR_VARS {
            if let Some(v) = lookup(key).filter(|v| !v.trim().is_empty()) {
                let color = parse_hex_color(&v).ok_or(ConfigError::Color(key, v))?;
                colors.push((var, color));
            }
        }
        Ok(Self {
            bind_addr,
            port,
            theme: Theme { mode, colors },
        })
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

/// Accepts `#rgb` or `#rrggbb` (the `#` is optional) and returns `#rrggbb`.
fn parse_hex_color(v: &str) -> Option<String> {
    let hex = v.trim().trim_start_matches('#');
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let hex = match hex.len() {
        3 => hex.chars().flat_map(|c| [c, c]).collect(),
        6 => hex.to_string(),
        _ => return None,
    };
    Some(format!("#{}", hex.to_ascii_lowercase()))
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
        assert_eq!(config.theme, Theme::default());
    }

    #[test]
    fn theme_values() {
        let config = parse(&[
            ("THEME", "Dark"),
            ("THEME_ACCENT", "#2F9E44"),
            ("THEME_BG", "000"),
            ("THEME_TEXT", ""),
        ])
        .unwrap();
        assert_eq!(config.theme.mode, ThemeMode::Dark);
        assert_eq!(
            config.theme.colors,
            vec![
                ("bg", "#000000".to_string()),
                ("accent", "#2f9e44".to_string())
            ]
        );
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
        assert!(matches!(parse(&[("PORT", "0")]), Err(ConfigError::Port(_))));
        assert!(matches!(
            parse(&[("PORT", "http")]),
            Err(ConfigError::Port(_))
        ));
        assert!(matches!(
            parse(&[("PORT", "70000")]),
            Err(ConfigError::Port(_))
        ));
        assert!(matches!(
            parse(&[("BIND_ADDR", "localhost")]),
            Err(ConfigError::BindAddr(_))
        ));
        assert!(matches!(
            parse(&[("THEME", "neon")]),
            Err(ConfigError::Theme(_))
        ));
        for bad in ["blue", "#12345", "#ggg", "#1234567"] {
            assert_eq!(
                parse(&[("THEME_ACCENT", bad)]),
                Err(ConfigError::Color("THEME_ACCENT", bad.to_string())),
                "{bad}"
            );
        }
    }
}
