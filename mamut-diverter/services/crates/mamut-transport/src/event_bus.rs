use mamut_core::events::SystemEvent;
use tokio::sync::broadcast;

/// In-process pub/sub bus for cross-crate system events.
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<SystemEvent>,
}

impl EventBus {
    /// Creates a new event bus with the provided channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Publishes an event to all active subscribers.
    pub fn publish(
        &self,
        event: SystemEvent,
    ) -> Result<usize, broadcast::error::SendError<SystemEvent>> {
        self.tx.send(event)
    }

    /// Subscribes to all future events from this bus.
    ///
    /// NOTE: `tokio::sync::broadcast` drops older messages if a receiver lags
    /// behind channel capacity. Slow consumers must handle `RecvError::Lagged`.
    pub fn subscribe(&self) -> broadcast::Receiver<SystemEvent> {
        self.tx.subscribe()
    }

    /// Returns the number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1024)
    }
}

#[cfg(test)]
mod tests {
    use super::EventBus;
    use mamut_core::events::SystemEvent;
    use mamut_core::types::ItemId;

    #[tokio::test]
    async fn publish_subscribe_round_trip() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();

        let event = SystemEvent::ItemScanned {
            item_id: ItemId("item-123".to_string()),
            barcode: "RS123456789".to_string(),
        };

        bus.publish(event).expect("event should be published");
        let received = rx.recv().await.expect("event should be received");

        match received {
            SystemEvent::ItemScanned { item_id, barcode } => {
                assert_eq!(item_id.0, "item-123");
                assert_eq!(barcode, "RS123456789");
            }
            other => panic!("unexpected event variant: {other:?}"),
        }
    }
}
