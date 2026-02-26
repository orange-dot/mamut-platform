use crate::zone::{Zone, ZoneError};
use async_stream::try_stream;
use mamut_core::time::now_unix_timestamp;
use mamut_proto::mamut::common::{Timestamp, ZoneId};
use mamut_proto::mamut::conveyor::conveyor_service_server::ConveyorService;
use mamut_proto::mamut::conveyor::{
    GetZoneStatusRequest, SetZoneSpeedRequest, StreamZoneEventsRequest, ZoneEvent, ZoneStatus,
};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tonic::{Request, Response, Status};

pub type ZoneEventStream =
    Pin<Box<dyn tokio_stream::Stream<Item = Result<ZoneEvent, Status>> + Send>>;
const EVENT_ZONE_RUNNING: &str = "zone_running";
const EVENT_ZONE_STOPPED: &str = "zone_stopped";

#[derive(Clone)]
pub struct ConveyorGrpcService {
    zones: Arc<RwLock<HashMap<String, Zone>>>,
    tx: broadcast::Sender<ZoneEvent>,
    max_speed_mps: f32,
}

impl ConveyorGrpcService {
    pub fn new(max_speed_mps: f32, event_channel_capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(event_channel_capacity);
        Self {
            zones: Arc::new(RwLock::new(HashMap::new())),
            tx,
            max_speed_mps: max_speed_mps.max(0.0),
        }
    }

    fn is_valid_zone_id(zone_id: &str) -> bool {
        !zone_id.trim().is_empty()
    }

    fn publish_event(&self, zone_id: &str, event_type: impl Into<String>) {
        let _ = self.tx.send(ZoneEvent {
            zone_id: Some(ZoneId {
                value: zone_id.to_string(),
            }),
            timestamp: Some(now_timestamp()),
            event_type: event_type.into(),
        });
    }
}

impl Default for ConveyorGrpcService {
    fn default() -> Self {
        // default from config/defaults/conveyor.toml max_speed_mm_per_s = 2000
        // => 2.0 m/s
        Self::new(2.0, 1024)
    }
}

#[tonic::async_trait]
impl ConveyorService for ConveyorGrpcService {
    type StreamZoneEventsStream = ZoneEventStream;

    async fn get_zone_status(
        &self,
        request: Request<GetZoneStatusRequest>,
    ) -> Result<Response<ZoneStatus>, Status> {
        let req = request.into_inner();
        let zone_id = req
            .zone_id
            .map(|id| id.value)
            .ok_or_else(|| Status::invalid_argument("zone_id is required"))?;
        if !Self::is_valid_zone_id(&zone_id) {
            return Err(Status::invalid_argument("zone_id must not be empty"));
        }

        if let Some(existing) = self.zones.read().await.get(&zone_id) {
            return Ok(Response::new(existing.status()));
        }

        let mut guard = self.zones.write().await;
        let zone = guard
            .entry(zone_id.clone())
            .or_insert_with(|| Zone::new(zone_id, self.max_speed_mps));
        Ok(Response::new(zone.status()))
    }

    async fn set_zone_speed(
        &self,
        request: Request<SetZoneSpeedRequest>,
    ) -> Result<Response<ZoneStatus>, Status> {
        let req = request.into_inner();
        let zone_id = req
            .zone_id
            .map(|id| id.value)
            .ok_or_else(|| Status::invalid_argument("zone_id is required"))?;
        if !Self::is_valid_zone_id(&zone_id) {
            return Err(Status::invalid_argument("zone_id must not be empty"));
        }

        let mut guard = self.zones.write().await;
        let zone = guard
            .entry(zone_id.clone())
            .or_insert_with(|| Zone::new(zone_id.clone(), self.max_speed_mps));
        let status = zone.set_speed(req.speed_mps).map_err(map_zone_error)?;
        drop(guard);

        let event_type = if status.speed_mps == 0.0 {
            EVENT_ZONE_STOPPED
        } else {
            EVENT_ZONE_RUNNING
        };
        self.publish_event(&zone_id, event_type);

        Ok(Response::new(status))
    }

    async fn stream_zone_events(
        &self,
        _request: Request<StreamZoneEventsRequest>,
    ) -> Result<Response<Self::StreamZoneEventsStream>, Status> {
        let mut rx = self.tx.subscribe();
        let stream = try_stream! {
            loop {
                match rx.recv().await {
                    Ok(event) => yield event,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        };

        Ok(Response::new(Box::pin(stream)))
    }
}

fn map_zone_error(err: ZoneError) -> Status {
    match err {
        ZoneError::NegativeSpeed(_) | ZoneError::InvalidSpeed(_) => {
            Status::invalid_argument(err.to_string())
        }
        ZoneError::Faulted => Status::failed_precondition(err.to_string()),
    }
}

fn now_timestamp() -> Timestamp {
    let (seconds, nanos) = now_unix_timestamp();
    Timestamp {
        seconds,
        nanos,
    }
}

#[cfg(test)]
mod tests {
    use super::{ConveyorGrpcService, EVENT_ZONE_RUNNING};
    use mamut_proto::mamut::common::ZoneId;
    use mamut_proto::mamut::conveyor::conveyor_service_server::ConveyorService;
    use mamut_proto::mamut::conveyor::{
        GetZoneStatusRequest, SetZoneSpeedRequest, StreamZoneEventsRequest, ZoneState,
    };
    use std::time::Duration;
    use tokio::time::timeout;
    use tokio_stream::StreamExt;
    use tonic::Request;

    #[tokio::test]
    async fn set_speed_applies_clamp_and_transitions() {
        let service = ConveyorGrpcService::new(2.0, 16);

        let running = ConveyorService::set_zone_speed(
            &service,
            Request::new(SetZoneSpeedRequest {
                zone_id: Some(ZoneId {
                    value: "zone-1".to_string(),
                }),
                speed_mps: 3.2,
            }),
        )
        .await
        .expect("set speed should succeed")
        .into_inner();
        assert_eq!(running.state, ZoneState::Running as i32);
        assert_eq!(running.speed_mps, 2.0);

        let stopped = ConveyorService::set_zone_speed(
            &service,
            Request::new(SetZoneSpeedRequest {
                zone_id: Some(ZoneId {
                    value: "zone-1".to_string(),
                }),
                speed_mps: 0.0,
            }),
        )
        .await
        .expect("stop should succeed")
        .into_inner();
        assert_eq!(stopped.state, ZoneState::Stopped as i32);
        assert_eq!(stopped.speed_mps, 0.0);
    }

    #[tokio::test]
    async fn get_zone_status_returns_default_idle_zone() {
        let service = ConveyorGrpcService::new(2.0, 16);
        let status = ConveyorService::get_zone_status(
            &service,
            Request::new(GetZoneStatusRequest {
                zone_id: Some(ZoneId {
                    value: "zone-2".to_string(),
                }),
            }),
        )
        .await
        .expect("status should be returned")
        .into_inner();

        assert_eq!(status.state, ZoneState::Idle as i32);
        assert_eq!(status.speed_mps, 0.0);
        assert!(!status.item_present);
    }

    #[tokio::test]
    async fn stream_zone_events_emits_on_speed_changes() {
        let service = ConveyorGrpcService::new(2.0, 16);
        let stream_response = ConveyorService::stream_zone_events(
            &service,
            Request::new(StreamZoneEventsRequest {}),
        )
        .await
        .expect("stream should be created");
        let mut stream = stream_response.into_inner();

        let _ = ConveyorService::set_zone_speed(
            &service,
            Request::new(SetZoneSpeedRequest {
                zone_id: Some(ZoneId {
                    value: "zone-3".to_string(),
                }),
                speed_mps: 1.0,
            }),
        )
        .await
        .expect("speed set should succeed");

        let event = timeout(Duration::from_secs(1), stream.next())
            .await
            .expect("event should arrive in time")
            .expect("stream item expected")
            .expect("event should be ok");

        assert_eq!(
            event.zone_id.expect("zone id is required").value,
            "zone-3".to_string()
        );
        assert_eq!(event.event_type, EVENT_ZONE_RUNNING);
    }

    #[tokio::test]
    async fn get_zone_status_rejects_missing_zone_id() {
        let service = ConveyorGrpcService::new(2.0, 16);
        let result = ConveyorService::get_zone_status(
            &service,
            Request::new(GetZoneStatusRequest { zone_id: None }),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            result.expect_err("must be error").code(),
            tonic::Code::InvalidArgument
        );
    }

    #[tokio::test]
    async fn set_zone_speed_rejects_empty_zone_id() {
        let service = ConveyorGrpcService::new(2.0, 16);
        let result = ConveyorService::set_zone_speed(
            &service,
            Request::new(SetZoneSpeedRequest {
                zone_id: Some(ZoneId {
                    value: "  ".to_string(),
                }),
                speed_mps: 1.0,
            }),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            result.expect_err("must be error").code(),
            tonic::Code::InvalidArgument
        );
    }
}
