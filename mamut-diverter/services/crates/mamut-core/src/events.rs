use crate::types::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    ItemScanned { item_id: ItemId, barcode: String },
    ItemEnteredZone { item_id: ItemId, zone_id: ZoneId },
    DiverterActivated { diverter_id: DiverterId, direction: Direction },
    DiverterDeactivated { diverter_id: DiverterId },
    AlarmRaised { source: String, severity: Severity, message: String },
}
