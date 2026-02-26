use mamut_core::time::now_unix_timestamp;
use mamut_proto::mamut::common::{ItemId, Timestamp};
use mamut_proto::mamut::identification::ScanResult;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{broadcast, RwLock};

#[derive(Debug, Error, PartialEq)]
pub enum ScanError {
    #[error("item_id must not be empty")]
    InvalidItemId,
    #[error("barcode must not be empty")]
    InvalidBarcode,
    #[error("confidence must be in range [0.0, 1.0]: {0}")]
    InvalidConfidence(f32),
    #[error("scan result must include item_id")]
    MissingItemId,
}

#[derive(Clone)]
pub struct ScannerService {
    scans: Arc<RwLock<HashMap<String, ScanResult>>>,
    tx: broadcast::Sender<ScanResult>,
}

impl ScannerService {
    pub fn new(channel_capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(channel_capacity);
        Self {
            scans: Arc::new(RwLock::new(HashMap::new())),
            tx,
        }
    }

    pub async fn record_scan(
        &self,
        item_id: impl AsRef<str>,
        barcode: impl AsRef<str>,
        confidence: f32,
    ) -> Result<ScanResult, ScanError> {
        let item_id = normalize_item_id(item_id.as_ref())?;
        let barcode = normalize_barcode(barcode.as_ref())?;
        validate_confidence(confidence)?;

        let scan = ScanResult {
            item_id: Some(ItemId {
                value: item_id.clone(),
            }),
            barcode,
            confidence,
            timestamp: Some(now_timestamp()),
        };

        self.store_scan(item_id, scan).await
    }

    pub async fn insert_scan(&self, mut scan: ScanResult) -> Result<ScanResult, ScanError> {
        let item_id = normalize_item_id(
            scan.item_id
                .as_ref()
                .map(|id| id.value.as_str())
                .ok_or(ScanError::MissingItemId)?,
        )?;
        let barcode = normalize_barcode(&scan.barcode)?;
        validate_confidence(scan.confidence)?;

        scan.item_id = Some(ItemId {
            value: item_id.clone(),
        });
        scan.barcode = barcode;
        if scan.timestamp.is_none() {
            scan.timestamp = Some(now_timestamp());
        }

        self.store_scan(item_id, scan).await
    }

    async fn store_scan(&self, item_id: String, scan: ScanResult) -> Result<ScanResult, ScanError> {
        self.scans.write().await.insert(item_id, scan.clone());
        let _ = self.tx.send(scan.clone());
        Ok(scan)
    }

    pub async fn get_scan(&self, item_id: &str) -> Option<ScanResult> {
        let item_id = item_id.trim();
        if item_id.is_empty() {
            return None;
        }
        self.scans.read().await.get(item_id).cloned()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ScanResult> {
        self.tx.subscribe()
    }
}

impl Default for ScannerService {
    fn default() -> Self {
        Self::new(1024)
    }
}

fn now_timestamp() -> Timestamp {
    let (seconds, nanos) = now_unix_timestamp();
    Timestamp { seconds, nanos }
}

fn validate_item_id(item_id: &str) -> Result<(), ScanError> {
    if item_id.is_empty() {
        return Err(ScanError::InvalidItemId);
    }
    Ok(())
}

fn validate_barcode(barcode: &str) -> Result<(), ScanError> {
    if barcode.is_empty() {
        return Err(ScanError::InvalidBarcode);
    }
    Ok(())
}

fn normalize_item_id(item_id: &str) -> Result<String, ScanError> {
    let normalized = item_id.trim();
    validate_item_id(normalized)?;
    Ok(normalized.to_string())
}

fn normalize_barcode(barcode: &str) -> Result<String, ScanError> {
    let normalized = barcode.trim();
    validate_barcode(normalized)?;
    Ok(normalized.to_string())
}

fn validate_confidence(confidence: f32) -> Result<(), ScanError> {
    if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
        return Err(ScanError::InvalidConfidence(confidence));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ScanError, ScannerService};
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn record_and_get_scan_result() {
        let scanner = ScannerService::new(16);
        scanner
            .record_scan("item-001", "PKG-123", 0.97)
            .await
            .expect("scan should be recorded");

        let scan = scanner
            .get_scan("item-001")
            .await
            .expect("scan should be present");
        assert_eq!(scan.barcode, "PKG-123");
        assert_eq!(scan.confidence, 0.97);
        assert!(scan.timestamp.is_some());
    }

    #[tokio::test]
    async fn rejects_invalid_confidence() {
        let scanner = ScannerService::new(16);
        let err = scanner
            .record_scan("item-001", "PKG-123", 1.5)
            .await
            .expect_err("confidence out of range must fail");
        assert_eq!(err, ScanError::InvalidConfidence(1.5));
    }

    #[tokio::test]
    async fn rejects_empty_barcode() {
        let scanner = ScannerService::new(16);
        let err = scanner
            .record_scan("item-001", "   ", 0.9)
            .await
            .expect_err("empty barcode must fail");
        assert_eq!(err, ScanError::InvalidBarcode);
    }

    #[tokio::test]
    async fn rejects_non_finite_confidence() {
        let scanner = ScannerService::new(16);
        let nan_err = scanner
            .record_scan("item-001", "PKG-123", f32::NAN)
            .await
            .expect_err("nan confidence must fail");
        assert!(matches!(nan_err, ScanError::InvalidConfidence(v) if v.is_nan()));

        let inf_err = scanner
            .record_scan("item-001", "PKG-123", f32::INFINITY)
            .await
            .expect_err("infinite confidence must fail");
        assert_eq!(inf_err, ScanError::InvalidConfidence(f32::INFINITY));
    }

    #[tokio::test]
    async fn normalizes_trimmed_item_id_and_barcode_values() {
        let scanner = ScannerService::new(16);
        scanner
            .record_scan("  item-003  ", "  PKG-003  ", 0.88)
            .await
            .expect("scan should be recorded");

        let scan = scanner
            .get_scan("item-003")
            .await
            .expect("scan should be found by normalized key");
        assert_eq!(scan.item_id.expect("item id required").value, "item-003");
        assert_eq!(scan.barcode, "PKG-003");
    }

    #[tokio::test]
    async fn latest_scan_overwrites_previous_by_item_id() {
        let scanner = ScannerService::new(16);
        scanner
            .record_scan("item-004", "PKG-OLD", 0.95)
            .await
            .expect("first scan should succeed");
        scanner
            .record_scan("item-004", "PKG-NEW", 0.80)
            .await
            .expect("second scan should succeed");

        let scan = scanner
            .get_scan("item-004")
            .await
            .expect("latest scan should exist");
        assert_eq!(scan.barcode, "PKG-NEW");
        assert_eq!(scan.confidence, 0.80);
    }

    #[tokio::test]
    async fn subscribe_receives_inserted_scan() {
        let scanner = ScannerService::new(16);
        let mut rx = scanner.subscribe();

        scanner
            .record_scan("item-002", "PKG-456", 0.91)
            .await
            .expect("scan should be recorded");

        let scan = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("scan should arrive")
            .expect("scan should be available");
        assert_eq!(scan.barcode, "PKG-456");
    }
}
