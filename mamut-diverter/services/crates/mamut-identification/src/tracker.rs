use mamut_core::time::now_unix_timestamp;
use mamut_proto::mamut::common::{ItemId, Timestamp, ZoneId};
use mamut_proto::mamut::tracking::ItemPosition;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{broadcast, RwLock};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TrackingError {
    #[error("item_id must not be empty")]
    InvalidItemId,
    #[error("zone_id must not be empty")]
    InvalidZoneId,
    #[error("item position must include item_id")]
    MissingItemId,
    #[error("item position must include zone_id")]
    MissingZoneId,
}

#[derive(Clone)]
pub struct ItemTracker {
    positions: Arc<RwLock<HashMap<String, ItemPosition>>>,
    tx: broadcast::Sender<ItemPosition>,
}

impl ItemTracker {
    pub fn new(channel_capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(channel_capacity);
        Self {
            positions: Arc::new(RwLock::new(HashMap::new())),
            tx,
        }
    }

    pub async fn update_position(
        &self,
        item_id: impl AsRef<str>,
        zone_id: impl AsRef<str>,
    ) -> Result<ItemPosition, TrackingError> {
        let item_id = normalize_item_id(item_id.as_ref())?;
        let zone_id = normalize_zone_id(zone_id.as_ref())?;

        let position = ItemPosition {
            item_id: Some(ItemId {
                value: item_id.clone(),
            }),
            zone_id: Some(ZoneId {
                value: zone_id.clone(),
            }),
            timestamp: Some(now_timestamp()),
        };

        self.store_position(item_id, position).await
    }

    pub async fn insert_position(
        &self,
        mut position: ItemPosition,
    ) -> Result<ItemPosition, TrackingError> {
        let item_id = normalize_item_id(
            position
                .item_id
                .as_ref()
                .map(|id| id.value.as_str())
                .ok_or(TrackingError::MissingItemId)?,
        )?;
        let zone_id = normalize_zone_id(
            position
                .zone_id
                .as_ref()
                .map(|id| id.value.as_str())
                .ok_or(TrackingError::MissingZoneId)?,
        )?;

        position.item_id = Some(ItemId {
            value: item_id.clone(),
        });
        position.zone_id = Some(ZoneId { value: zone_id });
        if position.timestamp.is_none() {
            position.timestamp = Some(now_timestamp());
        }

        self.store_position(item_id, position).await
    }

    async fn store_position(
        &self,
        item_id: String,
        position: ItemPosition,
    ) -> Result<ItemPosition, TrackingError> {
        self.positions
            .write()
            .await
            .insert(item_id, position.clone());
        let _ = self.tx.send(position.clone());
        Ok(position)
    }

    pub async fn get_position(&self, item_id: &str) -> Option<ItemPosition> {
        let item_id = item_id.trim();
        if item_id.is_empty() {
            return None;
        }
        self.positions.read().await.get(item_id).cloned()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ItemPosition> {
        self.tx.subscribe()
    }
}

impl Default for ItemTracker {
    fn default() -> Self {
        Self::new(1024)
    }
}

fn now_timestamp() -> Timestamp {
    let (seconds, nanos) = now_unix_timestamp();
    Timestamp { seconds, nanos }
}

fn validate_item_id(item_id: &str) -> Result<(), TrackingError> {
    if item_id.is_empty() {
        return Err(TrackingError::InvalidItemId);
    }
    Ok(())
}

fn validate_zone_id(zone_id: &str) -> Result<(), TrackingError> {
    if zone_id.is_empty() {
        return Err(TrackingError::InvalidZoneId);
    }
    Ok(())
}

fn normalize_item_id(item_id: &str) -> Result<String, TrackingError> {
    let normalized = item_id.trim();
    validate_item_id(normalized)?;
    Ok(normalized.to_string())
}

fn normalize_zone_id(zone_id: &str) -> Result<String, TrackingError> {
    let normalized = zone_id.trim();
    validate_zone_id(normalized)?;
    Ok(normalized.to_string())
}

#[cfg(test)]
mod tests {
    use super::{ItemTracker, TrackingError};
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn update_and_get_position() {
        let tracker = ItemTracker::new(16);
        tracker
            .update_position("item-101", "zone-03")
            .await
            .expect("position should be updated");

        let position = tracker
            .get_position("item-101")
            .await
            .expect("position should exist");
        assert_eq!(position.zone_id.expect("zone id required").value, "zone-03");
        assert!(position.timestamp.is_some());
    }

    #[tokio::test]
    async fn rejects_empty_zone_id() {
        let tracker = ItemTracker::new(16);
        let err = tracker
            .update_position("item-101", " ")
            .await
            .expect_err("empty zone id must fail");
        assert_eq!(err, TrackingError::InvalidZoneId);
    }

    #[tokio::test]
    async fn subscribe_receives_position_updates() {
        let tracker = ItemTracker::new(16);
        let mut rx = tracker.subscribe();
        tracker
            .update_position("item-102", "zone-05")
            .await
            .expect("position should update");

        let position = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("position should arrive")
            .expect("position should be available");
        assert_eq!(position.zone_id.expect("zone id required").value, "zone-05");
    }

    #[tokio::test]
    async fn normalizes_trimmed_item_and_zone_values() {
        let tracker = ItemTracker::new(16);
        tracker
            .update_position("  item-103  ", "  zone-06  ")
            .await
            .expect("position should be updated");

        let position = tracker
            .get_position("item-103")
            .await
            .expect("position should be found by normalized key");
        assert_eq!(
            position.item_id.expect("item id required").value,
            "item-103"
        );
        assert_eq!(position.zone_id.expect("zone id required").value, "zone-06");
    }
}
