use mamut_core::Direction;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRoute {
    pub destination: String,
    pub target: Direction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BarcodeRule {
    prefix: String,
    route: ResolvedRoute,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SortError {
    #[error("item_id must not be empty")]
    InvalidItemId,
    #[error("barcode must not be empty")]
    InvalidBarcode,
    #[error("barcode prefix must not be empty")]
    InvalidPrefix,
    #[error("destination must not be empty")]
    InvalidDestination,
    #[error("no route for item '{item_id}' and barcode '{barcode}'")]
    NoRoute { item_id: String, barcode: String },
}

#[derive(Clone)]
pub struct SortEngine {
    item_overrides: Arc<RwLock<HashMap<String, ResolvedRoute>>>,
    barcode_rules: Arc<RwLock<Vec<BarcodeRule>>>,
}

impl SortEngine {
    pub fn new() -> Self {
        Self {
            item_overrides: Arc::new(RwLock::new(HashMap::new())),
            barcode_rules: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn add_barcode_prefix_rule(
        &self,
        prefix: impl AsRef<str>,
        destination: impl AsRef<str>,
        target: Direction,
    ) -> Result<(), SortError> {
        let prefix = normalize_prefix(prefix.as_ref())?;
        let destination = normalize_destination(destination.as_ref())?;

        let mut rules = self.barcode_rules.write().await;
        rules.push(BarcodeRule {
            prefix,
            route: ResolvedRoute {
                destination,
                target,
            },
        });
        Ok(())
    }

    pub async fn set_item_override(
        &self,
        item_id: impl AsRef<str>,
        destination: impl AsRef<str>,
        target: Direction,
    ) -> Result<(), SortError> {
        let item_id = normalize_item_id(item_id.as_ref())?;
        let destination = normalize_destination(destination.as_ref())?;
        self.item_overrides.write().await.insert(
            item_id,
            ResolvedRoute {
                destination,
                target,
            },
        );
        Ok(())
    }

    pub async fn resolve(
        &self,
        item_id: impl AsRef<str>,
        barcode: impl AsRef<str>,
    ) -> Result<ResolvedRoute, SortError> {
        let item_id = normalize_item_id(item_id.as_ref())?;

        if let Some(route) = self.item_overrides.read().await.get(&item_id) {
            return Ok(route.clone());
        }

        let barcode = normalize_barcode(barcode.as_ref())?;
        let barcode_upper = barcode.to_ascii_uppercase();
        let rules = self.barcode_rules.read().await;
        if let Some(rule) = rules
            .iter()
            .find(|rule| barcode_upper.starts_with(&rule.prefix))
        {
            return Ok(rule.route.clone());
        }

        Err(SortError::NoRoute { item_id, barcode })
    }
}

impl Default for SortEngine {
    fn default() -> Self {
        Self {
            item_overrides: Arc::new(RwLock::new(HashMap::new())),
            barcode_rules: Arc::new(RwLock::new(vec![
                BarcodeRule {
                    prefix: "RS".to_string(),
                    route: ResolvedRoute {
                        destination: "postal-rs".to_string(),
                        target: Direction::Left,
                    },
                },
                BarcodeRule {
                    prefix: "EU".to_string(),
                    route: ResolvedRoute {
                        destination: "export-eu".to_string(),
                        target: Direction::Right,
                    },
                },
                BarcodeRule {
                    prefix: "LOC".to_string(),
                    route: ResolvedRoute {
                        destination: "local-sort".to_string(),
                        target: Direction::Straight,
                    },
                },
            ])),
        }
    }
}

fn normalize_item_id(item_id: &str) -> Result<String, SortError> {
    let normalized = item_id.trim();
    if normalized.is_empty() {
        return Err(SortError::InvalidItemId);
    }
    Ok(normalized.to_string())
}

fn normalize_barcode(barcode: &str) -> Result<String, SortError> {
    let normalized = barcode.trim();
    if normalized.is_empty() {
        return Err(SortError::InvalidBarcode);
    }
    Ok(normalized.to_string())
}

fn normalize_prefix(prefix: &str) -> Result<String, SortError> {
    let normalized = prefix.trim().to_ascii_uppercase();
    if normalized.is_empty() {
        return Err(SortError::InvalidPrefix);
    }
    Ok(normalized)
}

fn normalize_destination(destination: &str) -> Result<String, SortError> {
    let normalized = destination.trim();
    if normalized.is_empty() {
        return Err(SortError::InvalidDestination);
    }
    Ok(normalized.to_string())
}

#[cfg(test)]
mod tests {
    use super::{SortEngine, SortError};
    use mamut_core::Direction;

    #[tokio::test]
    async fn resolve_uses_prefix_rule() {
        let engine = SortEngine::default();
        let route = engine
            .resolve("item-001", "rs123456789")
            .await
            .expect("prefix rule should resolve");
        assert_eq!(route.target, Direction::Left);
        assert_eq!(route.destination, "postal-rs");
    }

    #[tokio::test]
    async fn item_override_takes_precedence_over_prefix() {
        let engine = SortEngine::default();
        engine
            .set_item_override("item-002", "priority-lane", Direction::Straight)
            .await
            .expect("override should be stored");

        let route = engine
            .resolve("item-002", "EU777")
            .await
            .expect("override should resolve");
        assert_eq!(route.target, Direction::Straight);
        assert_eq!(route.destination, "priority-lane");
    }

    #[tokio::test]
    async fn unknown_route_returns_no_route_error() {
        let engine = SortEngine::default();
        let err = engine
            .resolve("item-003", "ZZ123")
            .await
            .expect_err("unknown barcode should fail");
        assert!(matches!(err, SortError::NoRoute { .. }));
    }

    #[tokio::test]
    async fn empty_barcode_is_rejected() {
        let engine = SortEngine::default();
        let err = engine
            .resolve("item-004", "   ")
            .await
            .expect_err("empty barcode should fail");
        assert_eq!(err, SortError::InvalidBarcode);
    }
}
