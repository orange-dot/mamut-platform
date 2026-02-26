use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Base transport config used by gateway-like components that require both
/// gRPC listener and MQTT broker endpoints.
pub struct TransportConfig {
    pub grpc_listen: String,
    pub mqtt_broker: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ConfigShape {
    Flat(TransportConfig),
    Nested { gateway: TransportConfig },
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to parse config file {path}: {source}")]
    Parse {
        path: String,
        source: toml::de::Error,
    },
}

impl TransportConfig {
    /// Parses transport config from a TOML string.
    pub fn from_toml_str(input: &str) -> Result<Self, toml::de::Error> {
        match toml::from_str::<ConfigShape>(input)? {
            ConfigShape::Flat(config) => Ok(config),
            ConfigShape::Nested { gateway } => Ok(gateway),
        }
    }

    /// Loads transport config from a TOML file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path_ref = path.as_ref();
        let path_str = path_ref.display().to_string();
        let content = fs::read_to_string(path_ref).map_err(|source| ConfigError::Io {
            path: path_str.clone(),
            source,
        })?;
        Self::from_toml_str(&content).map_err(|source| ConfigError::Parse {
            path: path_str,
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::TransportConfig;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parse_nested_gateway_shape() {
        let input = r#"
[gateway]
grpc_listen = "0.0.0.0:50051"
mqtt_broker = "localhost:1883"
"#;

        let config = TransportConfig::from_toml_str(input).expect("config should parse");
        assert_eq!(config.grpc_listen, "0.0.0.0:50051");
        assert_eq!(config.mqtt_broker, "localhost:1883");
    }

    #[test]
    fn parse_flat_shape() {
        let input = r#"
grpc_listen = "127.0.0.1:50051"
mqtt_broker = "broker.example:1883"
"#;

        let config = TransportConfig::from_toml_str(input).expect("config should parse");
        assert_eq!(config.grpc_listen, "127.0.0.1:50051");
        assert_eq!(config.mqtt_broker, "broker.example:1883");
    }

    #[test]
    fn load_from_file() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let path: PathBuf = std::env::temp_dir().join(format!("mamut-transport-{unique}.toml"));

        let data = r#"
[gateway]
grpc_listen = "0.0.0.0:50051"
mqtt_broker = "localhost:1883"
"#;

        fs::write(&path, data).expect("temporary config should be written");
        let loaded = TransportConfig::from_file(&path).expect("config should load");
        assert_eq!(loaded.grpc_listen, "0.0.0.0:50051");
        assert_eq!(loaded.mqtt_broker, "localhost:1883");

        fs::remove_file(path).expect("temporary file should be removed");
    }
}
