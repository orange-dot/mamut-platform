use crate::scanner::ScannerService;
use crate::tracker::ItemTracker;
use async_stream::try_stream;
use mamut_proto::mamut::common::ItemId;
use mamut_proto::mamut::identification::identification_service_server::IdentificationService;
use mamut_proto::mamut::identification::{GetScanResultRequest, ScanResult, StreamScansRequest};
use mamut_proto::mamut::tracking::tracking_service_server::TrackingService;
use mamut_proto::mamut::tracking::{GetItemPositionRequest, ItemPosition, StreamPositionsRequest};
use std::pin::Pin;
use tonic::{Request, Response, Status};

pub type ScanResultStream =
    Pin<Box<dyn tokio_stream::Stream<Item = Result<ScanResult, Status>> + Send>>;
pub type ItemPositionStream =
    Pin<Box<dyn tokio_stream::Stream<Item = Result<ItemPosition, Status>> + Send>>;

#[derive(Clone)]
pub struct IdentificationGrpcService {
    scanner: ScannerService,
    tracker: ItemTracker,
}

impl IdentificationGrpcService {
    pub fn new(scanner: ScannerService, tracker: ItemTracker) -> Self {
        Self { scanner, tracker }
    }

    pub fn scanner(&self) -> ScannerService {
        self.scanner.clone()
    }

    pub fn tracker(&self) -> ItemTracker {
        self.tracker.clone()
    }
}

impl Default for IdentificationGrpcService {
    fn default() -> Self {
        Self::new(ScannerService::default(), ItemTracker::default())
    }
}

#[tonic::async_trait]
impl IdentificationService for IdentificationGrpcService {
    type StreamScansStream = ScanResultStream;

    async fn get_scan_result(
        &self,
        request: Request<GetScanResultRequest>,
    ) -> Result<Response<ScanResult>, Status> {
        let item_id = validate_item_id(request.into_inner().item_id).map_err(map_item_id_error)?;
        let scan = self
            .scanner
            .get_scan(&item_id)
            .await
            .ok_or_else(|| Status::not_found("scan result not found"))?;
        Ok(Response::new(scan))
    }

    async fn stream_scans(
        &self,
        _request: Request<StreamScansRequest>,
    ) -> Result<Response<Self::StreamScansStream>, Status> {
        let mut rx = self.scanner.subscribe();
        let stream = try_stream! {
            loop {
                match rx.recv().await {
                    Ok(scan) => yield scan,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        };

        Ok(Response::new(Box::pin(stream)))
    }
}

#[tonic::async_trait]
impl TrackingService for IdentificationGrpcService {
    type StreamPositionsStream = ItemPositionStream;

    async fn get_item_position(
        &self,
        request: Request<GetItemPositionRequest>,
    ) -> Result<Response<ItemPosition>, Status> {
        let item_id = validate_item_id(request.into_inner().item_id).map_err(map_item_id_error)?;
        let position = self
            .tracker
            .get_position(&item_id)
            .await
            .ok_or_else(|| Status::not_found("item position not found"))?;
        Ok(Response::new(position))
    }

    async fn stream_positions(
        &self,
        _request: Request<StreamPositionsRequest>,
    ) -> Result<Response<Self::StreamPositionsStream>, Status> {
        let mut rx = self.tracker.subscribe();
        let stream = try_stream! {
            loop {
                match rx.recv().await {
                    Ok(position) => yield position,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        };

        Ok(Response::new(Box::pin(stream)))
    }
}

fn validate_item_id(item_id: Option<ItemId>) -> Result<String, ItemIdValidationError> {
    let item_id = item_id
        .map(|id| id.value)
        .ok_or(ItemIdValidationError::Missing)?;
    if item_id.trim().is_empty() {
        return Err(ItemIdValidationError::Empty);
    }
    Ok(item_id)
}

fn map_item_id_error(err: ItemIdValidationError) -> Status {
    match err {
        ItemIdValidationError::Missing => Status::invalid_argument("item_id is required"),
        ItemIdValidationError::Empty => Status::invalid_argument("item_id must not be empty"),
    }
}

enum ItemIdValidationError {
    Missing,
    Empty,
}

#[cfg(test)]
mod tests {
    use super::IdentificationGrpcService;
    use crate::scanner::ScannerService;
    use crate::tracker::ItemTracker;
    use mamut_proto::mamut::common::ItemId;
    use mamut_proto::mamut::identification::identification_service_server::IdentificationService;
    use mamut_proto::mamut::identification::{GetScanResultRequest, StreamScansRequest};
    use mamut_proto::mamut::tracking::tracking_service_server::TrackingService;
    use mamut_proto::mamut::tracking::{GetItemPositionRequest, StreamPositionsRequest};
    use std::time::Duration;
    use tokio::time::timeout;
    use tokio_stream::StreamExt;
    use tonic::Request;

    #[tokio::test]
    async fn scan_track_and_query_position_flow() {
        let scanner = ScannerService::new(16);
        let tracker = ItemTracker::new(16);
        let service = IdentificationGrpcService::new(scanner.clone(), tracker.clone());

        scanner
            .record_scan("item-200", "PKG-200", 0.95)
            .await
            .expect("scan should be recorded");
        tracker
            .update_position("item-200", "zone-07")
            .await
            .expect("position should be updated");

        let scan = IdentificationService::get_scan_result(
            &service,
            Request::new(GetScanResultRequest {
                item_id: Some(ItemId {
                    value: "item-200".to_string(),
                }),
            }),
        )
        .await
        .expect("scan query should succeed")
        .into_inner();
        assert_eq!(scan.barcode, "PKG-200");

        let position = TrackingService::get_item_position(
            &service,
            Request::new(GetItemPositionRequest {
                item_id: Some(ItemId {
                    value: "item-200".to_string(),
                }),
            }),
        )
        .await
        .expect("position query should succeed")
        .into_inner();
        assert_eq!(
            position.item_id.expect("item id required").value,
            "item-200".to_string()
        );
        assert_eq!(
            position.zone_id.expect("zone id required").value,
            "zone-07".to_string()
        );
    }

    #[tokio::test]
    async fn stream_scans_emits_new_scan() {
        let scanner = ScannerService::new(16);
        let tracker = ItemTracker::new(16);
        let service = IdentificationGrpcService::new(scanner.clone(), tracker);

        let mut stream =
            IdentificationService::stream_scans(&service, Request::new(StreamScansRequest {}))
                .await
                .expect("scan stream should start")
                .into_inner();

        scanner
            .record_scan("item-201", "PKG-201", 0.92)
            .await
            .expect("scan should be recorded");

        let scan = timeout(Duration::from_secs(1), stream.next())
            .await
            .expect("scan should arrive in time")
            .expect("stream item expected")
            .expect("stream item should be ok");
        assert_eq!(scan.barcode, "PKG-201");
    }

    #[tokio::test]
    async fn stream_positions_emits_new_position() {
        let scanner = ScannerService::new(16);
        let tracker = ItemTracker::new(16);
        let service = IdentificationGrpcService::new(scanner, tracker.clone());

        let mut stream =
            TrackingService::stream_positions(&service, Request::new(StreamPositionsRequest {}))
                .await
                .expect("position stream should start")
                .into_inner();

        tracker
            .update_position("item-202", "zone-08")
            .await
            .expect("position should update");

        let position = timeout(Duration::from_secs(1), stream.next())
            .await
            .expect("position should arrive in time")
            .expect("stream item expected")
            .expect("stream item should be ok");
        assert_eq!(
            position.item_id.expect("item id required").value,
            "item-202".to_string()
        );
    }

    #[tokio::test]
    async fn get_scan_result_rejects_missing_item_id() {
        let service = IdentificationGrpcService::default();
        let result = IdentificationService::get_scan_result(
            &service,
            Request::new(GetScanResultRequest { item_id: None }),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            result.expect_err("must be error").code(),
            tonic::Code::InvalidArgument
        );
    }
}
