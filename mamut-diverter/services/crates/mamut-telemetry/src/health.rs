use crate::metrics::{now_timestamp, MetricStream, MetricsRegistry};
use mamut_proto::mamut::telemetry::telemetry_service_server::TelemetryService;
use mamut_proto::mamut::telemetry::{
    GetHealthRequest, HealthStatus, StreamMetricsRequest,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};

#[derive(Clone)]
pub struct HealthRegistry {
    default_service_name: String,
    statuses: Arc<RwLock<HashMap<String, bool>>>,
    metrics: MetricsRegistry,
}

impl HealthRegistry {
    pub fn new(default_service_name: impl Into<String>, metrics: MetricsRegistry) -> Self {
        let service_name = default_service_name.into();
        let mut initial = HashMap::new();
        initial.insert(service_name.clone(), true);
        Self {
            default_service_name: service_name,
            statuses: Arc::new(RwLock::new(initial)),
            metrics,
        }
    }

    pub fn with_default_metrics(default_service_name: impl Into<String>) -> Self {
        Self::new(default_service_name, MetricsRegistry::new(1024))
    }

    pub async fn set_health(&self, service_name: impl Into<String>, healthy: bool) {
        self.statuses.write().await.insert(service_name.into(), healthy);
    }

    pub async fn health_of(&self, service_name: &str) -> Option<bool> {
        self.statuses.read().await.get(service_name).copied()
    }

    pub fn metrics(&self) -> MetricsRegistry {
        self.metrics.clone()
    }
}

#[tonic::async_trait]
impl TelemetryService for HealthRegistry {
    type StreamMetricsStream = MetricStream;

    async fn get_health(
        &self,
        request: Request<GetHealthRequest>,
    ) -> Result<Response<HealthStatus>, Status> {
        let requested_name = request.into_inner().service_name;
        let service_name = if requested_name.is_empty() {
            self.default_service_name.clone()
        } else {
            requested_name
        };

        let healthy = self.health_of(&service_name).await.unwrap_or(false);
        Ok(Response::new(HealthStatus {
            service_name,
            healthy,
            timestamp: Some(now_timestamp()),
        }))
    }

    async fn stream_metrics(
        &self,
        request: Request<StreamMetricsRequest>,
    ) -> Result<Response<Self::StreamMetricsStream>, Status> {
        Ok(Response::new(self.metrics.stream_metrics(request.into_inner())))
    }
}

#[cfg(test)]
mod tests {
    use super::HealthRegistry;
    use crate::metrics::MetricsRegistry;
    use mamut_proto::mamut::telemetry::telemetry_service_server::TelemetryService;
    use mamut_proto::mamut::telemetry::{GetHealthRequest, StreamMetricsRequest};
    use std::collections::HashMap;
    use std::time::Duration;
    use tokio::time::timeout;
    use tokio_stream::StreamExt;
    use tonic::Request;

    #[tokio::test]
    async fn get_health_returns_registered_status() {
        let registry = HealthRegistry::new("mamut-controller", MetricsRegistry::new(16));
        registry.set_health("mamut-controller", true).await;

        let response = TelemetryService::get_health(
            &registry,
            Request::new(GetHealthRequest {
                service_name: "mamut-controller".to_string(),
            }),
        )
        .await
        .expect("health response should succeed")
        .into_inner();

        assert_eq!(response.service_name, "mamut-controller");
        assert!(response.healthy);
        assert!(response.timestamp.is_some());
    }

    #[tokio::test]
    async fn get_health_returns_unhealthy_state() {
        let registry = HealthRegistry::new("mamut-controller", MetricsRegistry::new(16));
        registry.set_health("mamut-controller", false).await;

        let response = TelemetryService::get_health(
            &registry,
            Request::new(GetHealthRequest {
                service_name: "mamut-controller".to_string(),
            }),
        )
        .await
        .expect("health response should succeed")
        .into_inner();

        assert_eq!(response.service_name, "mamut-controller");
        assert!(!response.healthy);
        assert!(response.timestamp.is_some());
    }

    #[tokio::test]
    async fn stream_metrics_delivers_recorded_sample() {
        let registry = HealthRegistry::new("mamut-gateway", MetricsRegistry::new(16));
        let response = TelemetryService::stream_metrics(
            &registry,
            Request::new(StreamMetricsRequest {
                metric_names: vec!["zone_speed_mps".to_string()],
            }),
        )
        .await
        .expect("metric stream should be created");

        let mut stream = response.into_inner();
        let mut labels = HashMap::new();
        labels.insert("zone".to_string(), "zone-01".to_string());
        registry
            .metrics()
            .record("zone_speed_mps", 1.25, labels.clone())
            .expect("metric should be recorded");

        let message = timeout(Duration::from_secs(1), stream.next())
            .await
            .expect("stream should yield in time")
            .expect("stream should yield an item")
            .expect("stream item should be ok");

        assert_eq!(message.name, "zone_speed_mps");
        assert_eq!(message.value, 1.25);
        assert_eq!(message.labels.get("zone"), Some(&"zone-01".to_string()));
    }
}
