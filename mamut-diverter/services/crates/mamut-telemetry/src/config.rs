use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub prometheus_listen: String,
    pub log_level: String,
    pub service_name: String,
    pub health_default: bool,
    pub metrics_channel_capacity: usize,
    pub metrics_flush_interval_ms: u64,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ConfigShape {
    Flat(TelemetryConfig),
    Nested { telemetry: TelemetryConfig },
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

impl TelemetryConfig {
    pub fn from_toml_str(input: &str) -> Result<Self, toml::de::Error> {
        match toml::from_str::<ConfigShape>(input)? {
            ConfigShape::Flat(config) => Ok(config),
            ConfigShape::Nested { telemetry } => Ok(telemetry),
        }
    }

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
    use super::TelemetryConfig;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parse_nested_shape() {
        let input = r#"
[telemetry]
prometheus_listen = "0.0.0.0:9090"
log_level = "info"
service_name = "mamut-telemetry"
health_default = true
metrics_channel_capacity = 1024
metrics_flush_interval_ms = 1000
"#;

        let cfg = TelemetryConfig::from_toml_str(input).expect("config should parse");
        assert_eq!(cfg.prometheus_listen, "0.0.0.0:9090");
        assert_eq!(cfg.log_level, "info");
        assert_eq!(cfg.service_name, "mamut-telemetry");
        assert!(cfg.health_default);
        assert_eq!(cfg.metrics_channel_capacity, 1024);
        assert_eq!(cfg.metrics_flush_interval_ms, 1000);
    }

    #[test]
    fn parse_flat_shape() {
        let input = r#"
prometheus_listen = "127.0.0.1:9090"
log_level = "debug"
service_name = "mamut-controller"
health_default = false
metrics_channel_capacity = 256
metrics_flush_interval_ms = 500
"#;

        let cfg = TelemetryConfig::from_toml_str(input).expect("config should parse");
        assert_eq!(cfg.prometheus_listen, "127.0.0.1:9090");
        assert_eq!(cfg.log_level, "debug");
        assert_eq!(cfg.service_name, "mamut-controller");
        assert!(!cfg.health_default);
        assert_eq!(cfg.metrics_channel_capacity, 256);
        assert_eq!(cfg.metrics_flush_interval_ms, 500);
    }

    #[test]
    fn load_from_file() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let path: PathBuf = std::env::temp_dir().join(format!("mamut-telemetry-{unique}.toml"));

        let data = r#"
[telemetry]
prometheus_listen = "0.0.0.0:9090"
log_level = "info"
service_name = "mamut-telemetry"
health_default = true
metrics_channel_capacity = 1024
metrics_flush_interval_ms = 1000
"#;

        fs::write(&path, data).expect("temporary config should be written");
        let loaded = TelemetryConfig::from_file(&path).expect("config should load");
        assert_eq!(loaded.service_name, "mamut-telemetry");
        assert_eq!(loaded.metrics_channel_capacity, 1024);

        fs::remove_file(path).expect("temporary file should be removed");
    }
}
