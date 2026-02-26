use async_stream::try_stream;
use mamut_core::time::now_unix_timestamp;
use mamut_proto::mamut::common::Timestamp;
use mamut_proto::mamut::telemetry::{Metric, StreamMetricsRequest};
use prometheus::{Encoder, GaugeVec, Opts, Registry, TextEncoder};
use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::sync::broadcast;
use tonic::Status;

pub type MetricStream = Pin<Box<dyn tokio_stream::Stream<Item = Result<Metric, Status>> + Send>>;

#[derive(Clone)]
struct GaugeEntry {
    label_keys: Vec<String>,
    gauge: GaugeVec,
}

#[derive(Clone)]
pub struct MetricsRegistry {
    registry: Registry,
    // NOTE: std::sync::Mutex is intentional here. The critical section is
    // short-lived, CPU-bound, and contains no `.await` points.
    gauges: Arc<Mutex<HashMap<String, GaugeEntry>>>,
    tx: broadcast::Sender<Metric>,
}

#[derive(Debug, Error)]
pub enum MetricRecordError {
    #[error("metric '{metric_name}' labels do not match existing schema: expected {expected:?}, got {got:?}")]
    LabelSetMismatch {
        metric_name: String,
        expected: Vec<String>,
        got: Vec<String>,
    },
    #[error("failed to create metric '{metric_name}': {source}")]
    CreateMetric {
        metric_name: String,
        source: prometheus::Error,
    },
    #[error("failed to register metric '{metric_name}': {source}")]
    RegisterMetric {
        metric_name: String,
        source: prometheus::Error,
    },
    #[error("failed to encode metrics as prometheus text: {0}")]
    Encode(prometheus::Error),
}

impl MetricsRegistry {
    pub fn new(channel_capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(channel_capacity);
        Self {
            registry: Registry::new(),
            gauges: Arc::new(Mutex::new(HashMap::new())),
            tx,
        }
    }

    pub fn record(
        &self,
        metric_name: &str,
        value: f64,
        labels: HashMap<String, String>,
    ) -> Result<(), MetricRecordError> {
        let normalized_name = normalize_metric_name(metric_name);
        let mut label_keys = labels.keys().cloned().collect::<Vec<_>>();
        label_keys.sort();
        let label_values = label_keys
            .iter()
            .map(|k| labels.get(k).expect("label key must exist").as_str())
            .collect::<Vec<_>>();

        let gauge = {
            let mut gauges = self.gauges.lock().expect("metrics lock must be available");
            match gauges.entry(normalized_name.clone()) {
                Entry::Occupied(entry) => {
                    if entry.get().label_keys != label_keys {
                        return Err(MetricRecordError::LabelSetMismatch {
                            metric_name: normalized_name,
                            expected: entry.get().label_keys.clone(),
                            got: label_keys,
                        });
                    }
                    entry.get().gauge.clone()
                }
                Entry::Vacant(entry) => {
                    let label_key_refs = label_keys.iter().map(String::as_str).collect::<Vec<_>>();
                    let gauge = GaugeVec::new(
                        Opts::new(
                            normalized_name.clone(),
                            format!("mamut dynamic metric: {normalized_name}"),
                        ),
                        &label_key_refs,
                    )
                    .map_err(|source| MetricRecordError::CreateMetric {
                        metric_name: normalized_name.clone(),
                        source,
                    })?;

                    self.registry.register(Box::new(gauge.clone())).map_err(|source| {
                        MetricRecordError::RegisterMetric {
                            metric_name: normalized_name.clone(),
                            source,
                        }
                    })?;
                    entry.insert(GaugeEntry {
                        label_keys: label_keys.clone(),
                        gauge: gauge.clone(),
                    });
                    gauge
                }
            }
        };

        gauge.with_label_values(&label_values).set(value);

        let _ = self.tx.send(Metric {
            name: normalized_name,
            value,
            labels,
            timestamp: Some(now_timestamp()),
        });
        Ok(())
    }

    pub fn stream_metrics(&self, request: StreamMetricsRequest) -> MetricStream {
        let filters = if request.metric_names.is_empty() {
            None
        } else {
            Some(request.metric_names.into_iter().collect::<HashSet<_>>())
        };

        let mut rx = self.tx.subscribe();
        let stream = try_stream! {
            loop {
                match rx.recv().await {
                    Ok(metric) => {
                        if filters
                            .as_ref()
                            .map(|set| set.contains(&metric.name))
                            .unwrap_or(true)
                        {
                            yield metric;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        };

        Box::pin(stream)
    }

    pub fn encode_prometheus(&self) -> Result<String, MetricRecordError> {
        let metric_families = self.registry.gather();
        let mut output = Vec::new();
        let encoder = TextEncoder::new();
        encoder
            .encode(&metric_families, &mut output)
            .map_err(MetricRecordError::Encode)?;
        Ok(String::from_utf8_lossy(&output).into_owned())
    }
}

pub fn now_timestamp() -> Timestamp {
    let (seconds, nanos) = now_unix_timestamp();
    Timestamp {
        seconds,
        nanos,
    }
}

fn normalize_metric_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len().max(1));
    for ch in name.chars() {
        let is_valid = ch.is_ascii_alphanumeric() || ch == '_' || ch == ':';
        out.push(if is_valid { ch } else { '_' });
    }

    if out.is_empty() {
        out.push_str("mamut_metric");
    } else if !matches!(out.chars().next(), Some(c) if c.is_ascii_alphabetic() || c == '_' || c == ':')
    {
        out.insert(0, '_');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::normalize_metric_name;
    use super::MetricsRegistry;
    use std::collections::HashMap;

    #[test]
    fn normalize_metric_name_keeps_prometheus_compatible_format() {
        assert_eq!(normalize_metric_name("zone.speed"), "zone_speed");
        assert_eq!(normalize_metric_name("9bad name"), "_9bad_name");
        assert_eq!(normalize_metric_name(""), "mamut_metric");
    }

    #[test]
    fn encode_prometheus_includes_recorded_metric() {
        let registry = MetricsRegistry::new(16);
        let mut labels = HashMap::new();
        labels.insert("zone".to_string(), "zone-01".to_string());

        registry
            .record("zone_speed_mps", 1.5, labels)
            .expect("metric should be recorded");
        let text = registry
            .encode_prometheus()
            .expect("prometheus text should be generated");

        assert!(text.contains("zone_speed_mps"));
    }
}
