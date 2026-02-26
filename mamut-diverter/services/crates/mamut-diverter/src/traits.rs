use mamut_core::{DiverterId, Direction, MamutError};

/// Status of a diverter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiverterState {
    Idle,
    Active(Direction),
    Fault(String),
    Testing,
}

/// Result of a diverter self-test.
#[derive(Debug, Clone)]
pub struct SelfTestResult {
    pub passed: bool,
    pub details: String,
}

/// Pluggable diverter driver trait.
///
/// Each diverter type (MDR, pop-up, shoe, etc.) implements this trait.
#[allow(async_fn_in_trait)]
pub trait DiverterDriver: Send + Sync {
    /// Initialize the diverter.
    async fn init(&self, id: &DiverterId) -> Result<(), MamutError>;

    /// Activate diversion to the given direction.
    async fn activate(&self, id: &DiverterId, target: Direction) -> Result<(), MamutError>;

    /// Deactivate diversion (return to pass-through).
    async fn deactivate(&self, id: &DiverterId) -> Result<(), MamutError>;

    /// Get current diverter status.
    async fn get_status(&self, id: &DiverterId) -> Result<DiverterState, MamutError>;

    /// Run self-test diagnostics.
    async fn self_test(&self, id: &DiverterId) -> Result<SelfTestResult, MamutError>;
}
