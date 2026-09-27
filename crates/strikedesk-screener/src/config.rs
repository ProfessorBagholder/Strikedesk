use serde::Deserialize;
use std::path::{Path, PathBuf};
use strikedesk_core::BadgeParams;

use crate::ScanError;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub data: DataConfig,
    #[serde(default)]
    pub universe: UniverseConfig,
    #[serde(default)]
    pub badges: BadgeParams,
    #[serde(default)]
    pub presets: Vec<Preset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DataConfig {
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default = "default_fixtures")]
    pub fixtures_path: String,
    #[serde(default = "default_cache")]
    pub cache_dir: String,
    #[serde(default = "default_ttl")]
    pub cache_ttl_hours: u64,
    #[serde(default = "default_benchmark")]
    pub benchmark: String,
    #[serde(default = "default_yahoo")]
    pub yahoo_base: String,
    #[serde(default = "default_alerts")]
    pub alerts_path: String,
}

impl Default for DataConfig {
    fn default() -> Self {
        Self {
            source: default_source(),
            fixtures_path: default_fixtures(),
            cache_dir: default_cache(),
            cache_ttl_hours: default_ttl(),
            benchmark: default_benchmark(),
            yahoo_base: default_yahoo(),
            alerts_path: default_alerts(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UniverseConfig {
    #[serde(default)]
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Preset {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub require: Vec<String>,
    #[serde(default)]
    pub min_strike: u8,
}

fn default_host() -> String {
    "127.0.0.1".into()
}
fn default_port() -> u16 {
    8790
}
fn default_source() -> String {
    "fixtures".into()
}
fn default_fixtures() -> String {
    "fixtures/universe.toml".into()
}
fn default_cache() -> String {
    "data/cache".into()
}
fn default_ttl() -> u64 {
    12
}
fn default_benchmark() -> String {
    "SPY".into()
}
fn default_yahoo() -> String {
    "https://query1.finance.yahoo.com".into()
}
fn default_alerts() -> String {
    "data/alerts.jsonl".into()
}

fn anchor(root: &Path, value: &str) -> String {
    let path = Path::new(value);
    if path.is_absolute() {
        value.to_string()
    } else {
        root.join(path).to_string_lossy().into_owned()
    }
}

pub fn load_config(path: &Path) -> Result<Config, ScanError> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| ScanError::Config(format!("{}: {err}", path.display())))?;
    let mut config: Config =
        toml::from_str(&text).map_err(|err| ScanError::Config(err.to_string()))?;
    let root = path
        .parent()
        .and_then(|dir| dir.parent())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    config.data.fixtures_path = anchor(&root, &config.data.fixtures_path);
    config.data.cache_dir = anchor(&root, &config.data.cache_dir);
    config.data.alerts_path = anchor(&root, &config.data.alerts_path);
    if config.presets.is_empty() {
        config.presets.push(Preset {
            id: "qullamaggie".into(),
            name: "Qullamaggie".into(),
            require: Vec::new(),
            min_strike: 0,
        });
    }
    apply_env(&mut config)?;
    Ok(config)
}

fn apply_env(config: &mut Config) -> Result<(), ScanError> {
    if let Ok(host) = std::env::var("STRIKEDESK_HOST") {
        if !host.is_empty() {
            config.server.host = host;
        }
    }
    if let Ok(port) = std::env::var("STRIKEDESK_PORT") {
        if !port.is_empty() {
            config.server.port = port
                .parse()
                .map_err(|_| ScanError::Config(format!("STRIKEDESK_PORT is not a port: {port}")))?;
        }
    }
    if let Ok(source) = std::env::var("STRIKEDESK_DATA_SOURCE") {
        if !source.is_empty() {
            config.data.source = source;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use strikedesk_core::BadgeParams;

    #[test]
    fn shipped_config_uses_engine_defaults() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/screener.toml");
        let config = load_config(&path).unwrap();
        assert_eq!(config.server.port, 8790);
        assert_eq!(config.data.source, "fixtures");
        assert_eq!(config.badges, BadgeParams::default());
        assert!(config
            .presets
            .iter()
            .any(|preset| preset.id == "qullamaggie"));
    }
}
