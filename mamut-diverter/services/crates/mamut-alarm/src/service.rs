use crate::alarm_manager::AlarmManager;
use async_stream::try_stream;
use mamut_proto::mamut::alarms::alarm_service_server::AlarmService;
use mamut_proto::mamut::alarms::{
    AcknowledgeAlarmRequest, Alarm, GetActiveAlarmsRequest, GetActiveAlarmsResponse,
    StreamAlarmsRequest,
};
use std::pin::Pin;
use tonic::{Request, Response, Status};

pub type AlarmStream = Pin<Box<dyn tokio_stream::Stream<Item = Result<Alarm, Status>> + Send>>;

#[derive(Clone)]
pub struct AlarmGrpcService {
    manager: AlarmManager,
}

impl AlarmGrpcService {
    pub fn new(manager: AlarmManager) -> Self {
        Self { manager }
    }

    pub fn manager(&self) -> AlarmManager {
        self.manager.clone()
    }
}

#[tonic::async_trait]
impl AlarmService for AlarmGrpcService {
    type StreamAlarmsStream = AlarmStream;

    async fn get_active_alarms(
        &self,
        _request: Request<GetActiveAlarmsRequest>,
    ) -> Result<Response<GetActiveAlarmsResponse>, Status> {
        let alarms = self.manager.get_active_alarms().await;
        Ok(Response::new(GetActiveAlarmsResponse { alarms }))
    }

    async fn acknowledge_alarm(
        &self,
        request: Request<AcknowledgeAlarmRequest>,
    ) -> Result<Response<Alarm>, Status> {
        let req = request.into_inner();
        if req.alarm_id.trim().is_empty() {
            return Err(Status::invalid_argument("alarm_id must not be empty"));
        }

        let alarm = self
            .manager
            .acknowledge_alarm(&req.alarm_id)
            .await
            .map_err(|_| Status::not_found("alarm not found"))?;
        Ok(Response::new(alarm))
    }

    async fn stream_alarms(
        &self,
        _request: Request<StreamAlarmsRequest>,
    ) -> Result<Response<Self::StreamAlarmsStream>, Status> {
        let mut rx = self.manager.subscribe();
        let stream = try_stream! {
            loop {
                match rx.recv().await {
                    Ok(alarm) => yield alarm,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        };

        Ok(Response::new(Box::pin(stream)))
    }
}

#[cfg(test)]
mod tests {
    use super::AlarmGrpcService;
    use crate::alarm_manager::AlarmManager;
    use mamut_proto::mamut::alarms::alarm_service_server::AlarmService;
    use mamut_proto::mamut::alarms::{
        AcknowledgeAlarmRequest, AlarmState, GetActiveAlarmsRequest, StreamAlarmsRequest,
    };
    use mamut_proto::mamut::common::Severity;
    use std::time::Duration;
    use tokio::time::timeout;
    use tokio_stream::StreamExt;
    use tonic::Request;

    #[tokio::test]
    async fn stream_alarms_receives_new_alarm() {
        let manager = AlarmManager::new(16);
        let service = AlarmGrpcService::new(manager.clone());

        let stream_response = AlarmService::stream_alarms(
            &service,
            Request::new(StreamAlarmsRequest {}),
        )
        .await
        .expect("stream should be created");
        let mut stream = stream_response.into_inner();

        let raised = manager
            .raise_alarm("safety/lightcurtain-01", Severity::Warning, "Beam interrupted")
            .await;
        let item = timeout(Duration::from_secs(1), stream.next())
            .await
            .expect("stream should emit quickly")
            .expect("stream item should exist")
            .expect("stream item should be ok");

        assert_eq!(item.alarm_id, raised.alarm_id);
    }

    #[tokio::test]
    async fn acknowledge_alarm_updates_state_and_active_list() {
        let manager = AlarmManager::new(16);
        let service = AlarmGrpcService::new(manager.clone());
        let raised = manager
            .raise_alarm("diverter/div-02", Severity::Critical, "Motor overload")
            .await;

        let ack = AlarmService::acknowledge_alarm(
            &service,
            Request::new(AcknowledgeAlarmRequest {
                alarm_id: raised.alarm_id.clone(),
            }),
        )
        .await
        .expect("ack should succeed")
        .into_inner();
        assert_eq!(ack.state, AlarmState::Acknowledged as i32);

        let active = AlarmService::get_active_alarms(
            &service,
            Request::new(GetActiveAlarmsRequest {}),
        )
        .await
        .expect("active alarms should be returned")
        .into_inner();

        assert_eq!(active.alarms.len(), 1);
        assert_eq!(active.alarms[0].alarm_id, raised.alarm_id);
    }

    #[tokio::test]
    async fn acknowledge_unknown_alarm_returns_not_found() {
        let service = AlarmGrpcService::new(AlarmManager::new(16));
        let result = AlarmService::acknowledge_alarm(
            &service,
            Request::new(AcknowledgeAlarmRequest {
                alarm_id: "nonexistent".to_string(),
            }),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(result.expect_err("must be error").code(), tonic::Code::NotFound);
    }
}
