use crate::sort_engine::{ResolvedRoute, SortEngine, SortError};
use mamut_core::Direction;
use mamut_proto::mamut::common::{Direction as ProtoDirection, ItemId};
use mamut_proto::mamut::wms::wms_service_server::WmsService;
use mamut_proto::mamut::wms::{GetSortDecisionRequest, SortDecision};
use tonic::{Request, Response, Status};

#[derive(Clone)]
pub struct WmsGrpcService {
    engine: SortEngine,
}

impl WmsGrpcService {
    pub fn new(engine: SortEngine) -> Self {
        Self { engine }
    }

    pub fn engine(&self) -> SortEngine {
        self.engine.clone()
    }
}

impl Default for WmsGrpcService {
    fn default() -> Self {
        Self::new(SortEngine::default())
    }
}

#[tonic::async_trait]
impl WmsService for WmsGrpcService {
    async fn get_sort_decision(
        &self,
        request: Request<GetSortDecisionRequest>,
    ) -> Result<Response<SortDecision>, Status> {
        let req = request.into_inner();
        let item_id = req
            .item_id
            .map(|id| id.value)
            .ok_or_else(|| Status::invalid_argument("item_id is required"))?;

        let route = self
            .engine
            .resolve(&item_id, &req.barcode)
            .await
            .map_err(map_sort_error)?;

        Ok(Response::new(to_sort_decision(item_id, route)))
    }
}

fn to_sort_decision(item_id: String, route: ResolvedRoute) -> SortDecision {
    SortDecision {
        item_id: Some(ItemId { value: item_id }),
        target: to_proto_direction(route.target) as i32,
        destination: route.destination,
    }
}

fn to_proto_direction(direction: Direction) -> ProtoDirection {
    match direction {
        Direction::Left => ProtoDirection::Left,
        Direction::Right => ProtoDirection::Right,
        Direction::Straight => ProtoDirection::Straight,
    }
}

fn map_sort_error(err: SortError) -> Status {
    match err {
        SortError::InvalidItemId | SortError::InvalidBarcode => {
            Status::invalid_argument(err.to_string())
        }
        SortError::InvalidPrefix | SortError::InvalidDestination => {
            Status::internal("invalid routing configuration")
        }
        SortError::NoRoute { .. } => Status::not_found(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::WmsGrpcService;
    use crate::sort_engine::SortEngine;
    use mamut_core::Direction;
    use mamut_proto::mamut::common::ItemId;
    use mamut_proto::mamut::wms::wms_service_server::WmsService;
    use mamut_proto::mamut::wms::GetSortDecisionRequest;
    use tonic::Request;

    #[tokio::test]
    async fn get_sort_decision_resolves_known_barcode() {
        let service = WmsGrpcService::default();
        let decision = WmsService::get_sort_decision(
            &service,
            Request::new(GetSortDecisionRequest {
                item_id: Some(ItemId {
                    value: "item-900".to_string(),
                }),
                barcode: "RS123456".to_string(),
            }),
        )
        .await
        .expect("decision should resolve")
        .into_inner();

        assert_eq!(
            decision.target,
            mamut_proto::mamut::common::Direction::Left as i32
        );
        assert_eq!(decision.destination, "postal-rs");
    }

    #[tokio::test]
    async fn get_sort_decision_uses_item_override() {
        let engine = SortEngine::default();
        engine
            .set_item_override("item-priority", "priority-lane", Direction::Straight)
            .await
            .expect("override should be set");
        let service = WmsGrpcService::new(engine);

        let decision = WmsService::get_sort_decision(
            &service,
            Request::new(GetSortDecisionRequest {
                item_id: Some(ItemId {
                    value: "item-priority".to_string(),
                }),
                barcode: "".to_string(),
            }),
        )
        .await
        .expect("override route should resolve")
        .into_inner();

        assert_eq!(
            decision.target,
            mamut_proto::mamut::common::Direction::Straight as i32
        );
        assert_eq!(decision.destination, "priority-lane");
    }

    #[tokio::test]
    async fn get_sort_decision_returns_not_found_for_unknown_route() {
        let service = WmsGrpcService::default();
        let result = WmsService::get_sort_decision(
            &service,
            Request::new(GetSortDecisionRequest {
                item_id: Some(ItemId {
                    value: "item-901".to_string(),
                }),
                barcode: "ZZ999".to_string(),
            }),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            result.expect_err("must be error").code(),
            tonic::Code::NotFound
        );
    }

    #[tokio::test]
    async fn get_sort_decision_rejects_missing_item_id() {
        let service = WmsGrpcService::default();
        let result = WmsService::get_sort_decision(
            &service,
            Request::new(GetSortDecisionRequest {
                item_id: None,
                barcode: "RS123".to_string(),
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
