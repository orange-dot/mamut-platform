use mamut_core::time::now_unix_timestamp;
use mamut_proto::mamut::alarms::{Alarm, AlarmState};
use mamut_proto::mamut::common::{Severity, Timestamp};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{broadcast, RwLock};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AlarmError {
    #[error("alarm not found: {0}")]
    NotFound(String),
}

#[derive(Clone)]
pub struct AlarmManager {
    alarms: Arc<RwLock<HashMap<String, Alarm>>>,
    seq: Arc<AtomicU64>,
    tx: broadcast::Sender<Alarm>,
}

impl AlarmManager {
    pub fn new(channel_capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(channel_capacity);
        Self {
            alarms: Arc::new(RwLock::new(HashMap::new())),
            seq: Arc::new(AtomicU64::new(1)),
            tx,
        }
    }

    pub async fn raise_alarm(
        &self,
        source: impl Into<String>,
        severity: Severity,
        message: impl Into<String>,
    ) -> Alarm {
        let alarm = Alarm {
            alarm_id: format!("alarm-{:06}", self.seq.fetch_add(1, Ordering::Relaxed)),
            severity: severity as i32,
            source: source.into(),
            message: message.into(),
            timestamp: Some(now_timestamp()),
            state: AlarmState::Active as i32,
        };

        self.alarms
            .write()
            .await
            .insert(alarm.alarm_id.clone(), alarm.clone());
        let _ = self.tx.send(alarm.clone());
        alarm
    }

    pub async fn acknowledge_alarm(&self, alarm_id: &str) -> Result<Alarm, AlarmError> {
        // TODO: Enforce stricter lifecycle transitions (e.g. reject duplicate
        // acknowledge operations) when controller escalation policy is wired.
        let mut guard = self.alarms.write().await;
        let alarm = guard
            .get_mut(alarm_id)
            .ok_or_else(|| AlarmError::NotFound(alarm_id.to_string()))?;

        alarm.state = AlarmState::Acknowledged as i32;
        alarm.timestamp = Some(now_timestamp());
        let updated = alarm.clone();
        drop(guard);

        let _ = self.tx.send(updated.clone());
        Ok(updated)
    }

    pub async fn clear_alarm(&self, alarm_id: &str) -> Result<Alarm, AlarmError> {
        // Clear removes the alarm from in-memory active storage to avoid
        // unbounded growth in long-running processes.
        let mut guard = self.alarms.write().await;
        let mut alarm = guard
            .remove(alarm_id)
            .ok_or_else(|| AlarmError::NotFound(alarm_id.to_string()))?;

        alarm.state = AlarmState::Cleared as i32;
        alarm.timestamp = Some(now_timestamp());
        let updated = alarm.clone();
        drop(guard);

        let _ = self.tx.send(updated.clone());
        Ok(updated)
    }

    pub async fn get_active_alarms(&self) -> Vec<Alarm> {
        let guard = self.alarms.read().await;
        guard
            .values()
            .filter(|alarm| alarm.state != AlarmState::Cleared as i32)
            .cloned()
            .collect()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Alarm> {
        self.tx.subscribe()
    }
}

impl Default for AlarmManager {
    fn default() -> Self {
        Self::new(1024)
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
    use super::AlarmManager;
    use mamut_proto::mamut::alarms::AlarmState;
    use mamut_proto::mamut::common::Severity;

    #[tokio::test]
    async fn raise_ack_clear_lifecycle() {
        let manager = AlarmManager::new(16);
        let alarm = manager
            .raise_alarm("diverter/div-01", Severity::Critical, "Actuator stalled")
            .await;

        assert_eq!(alarm.state, AlarmState::Active as i32);

        let acknowledged = manager
            .acknowledge_alarm(&alarm.alarm_id)
            .await
            .expect("alarm should be acknowledged");
        assert_eq!(
            acknowledged.state,
            AlarmState::Acknowledged as i32
        );

        let cleared = manager
            .clear_alarm(&alarm.alarm_id)
            .await
            .expect("alarm should be cleared");
        assert_eq!(cleared.state, AlarmState::Cleared as i32);

        let active = manager.get_active_alarms().await;
        assert!(active.is_empty());
    }
}
