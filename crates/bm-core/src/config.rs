use crate::network::NetworkConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenEdge {
    pub left: Option<String>,
    pub right: Option<String>,
    pub top: Option<String>,
    pub bottom: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub port: u16,
    pub secret: Option<String>,
    pub screens: Vec<ScreenEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub connect_addr: String,
    pub port: u16,
    pub secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: Option<ServerConfig>,
    pub client: Option<ClientConfig>,
    pub network: Option<NetworkConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: Some(ServerConfig {
                bind_addr: "0.0.0.0".into(),
                port: crate::protocol::DEFAULT_PORT,
                secret: None,
                screens: Vec::new(),
            }),
            client: Some(ClientConfig {
                connect_addr: "127.0.0.1".into(),
                port: crate::protocol::DEFAULT_PORT,
                secret: None,
            }),
            network: Some(NetworkConfig::default()),
        }
    }
}

pub fn config_path() -> PathBuf {
    if let Some(dir) = directories::ProjectDirs::from("com", "borderless", "mouse") {
        dir.config_dir().join("config.toml")
    } else {
        PathBuf::from("borderless-mouse.toml")
    }
}

pub fn load_config(path: &std::path::Path) -> anyhow::Result<AppConfig> {
    if path.exists() {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    } else {
        Ok(AppConfig::default())
    }
}

pub fn save_config(path: &std::path::Path, config: &AppConfig) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(config)?;
    std::fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::DEFAULT_PORT;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn unique_path() -> std::path::PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("borderless-mouse-test-config-{n}.toml"))
    }

    #[test]
    fn default_config_is_valid_toml() {
        let config = AppConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.server.is_some());
        assert!(parsed.client.is_some());
    }

    #[test]
    fn default_config_has_default_values() {
        let config = AppConfig::default();
        let server = config.server.unwrap();
        let client = config.client.unwrap();
        assert_eq!(server.bind_addr, "0.0.0.0");
        assert_eq!(server.port, DEFAULT_PORT);
        assert!(server.secret.is_none());
        assert!(server.screens.is_empty());
        assert_eq!(client.connect_addr, "127.0.0.1");
        assert_eq!(client.port, DEFAULT_PORT);
        assert!(client.secret.is_none());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let path = unique_path();
        let config = AppConfig {
            server: Some(ServerConfig {
                bind_addr: "192.168.1.1".into(),
                port: 9999,
                secret: Some("s3cr3t".into()),
                screens: vec![ScreenEdge {
                    left: Some("mac".into()),
                    right: None,
                    top: None,
                    bottom: Some("ipad".into()),
                }],
            }),
            client: Some(ClientConfig {
                connect_addr: "10.0.0.1".into(),
                port: 24800,
                secret: None,
            }),
            network: None,
        };
        save_config(&path, &config).unwrap();
        let loaded = load_config(&path).unwrap();
        assert_eq!(loaded.server.as_ref().unwrap().bind_addr, "192.168.1.1");
        assert_eq!(loaded.server.as_ref().unwrap().port, 9999);
        assert_eq!(
            loaded.server.as_ref().unwrap().secret.as_deref(),
            Some("s3cr3t")
        );
        assert_eq!(
            loaded.server.as_ref().unwrap().screens[0].left.as_deref(),
            Some("mac")
        );
        assert_eq!(
            loaded.server.as_ref().unwrap().screens[0].bottom.as_deref(),
            Some("ipad")
        );
        assert_eq!(loaded.client.as_ref().unwrap().connect_addr, "10.0.0.1");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn load_missing_file_returns_default() {
        let path = unique_path();
        let config = load_config(&path).unwrap();
        assert!(config.server.is_some());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn load_invalid_file_returns_error() {
        let path = unique_path();
        std::fs::write(&path, "invalid toml {{{").unwrap();
        let result = load_config(&path);
        assert!(result.is_err());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn config_with_minimal_fields() {
        let config = AppConfig {
            server: None,
            client: None,
            network: None,
        };
        let path = unique_path();
        save_config(&path, &config).unwrap();
        let loaded = load_config(&path).unwrap();
        assert!(loaded.server.is_none());
        assert!(loaded.client.is_none());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn multiple_screen_edges() {
        let config = AppConfig {
            server: Some(ServerConfig {
                bind_addr: "0.0.0.0".into(),
                port: 24800,
                secret: None,
                screens: vec![
                    ScreenEdge {
                        left: Some("mac-left".into()),
                        right: None,
                        top: None,
                        bottom: None,
                    },
                    ScreenEdge {
                        left: None,
                        right: Some("mac-right".into()),
                        top: None,
                        bottom: None,
                    },
                ],
            }),
            client: None,
            network: None,
        };
        let path = unique_path();
        save_config(&path, &config).unwrap();
        let loaded = load_config(&path).unwrap();
        let screens = loaded.server.unwrap().screens;
        assert_eq!(screens.len(), 2);
        assert_eq!(screens[0].left.as_deref(), Some("mac-left"));
        assert_eq!(screens[1].right.as_deref(), Some("mac-right"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn config_path_is_valid() {
        let path = config_path();
        let path_str = path.to_str().unwrap();
        assert!(path_str.ends_with("config.toml"), "path should end with config.toml, got: {path_str}");
        // Should be an absolute path or a relative filename
        assert!(
            path.is_absolute() || path_str == "borderless-mouse.toml",
            "unexpected path format: {path_str}"
        );
    }
}
