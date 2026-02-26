use mamut_core::{DiverterId, Direction, MamutError};
use crate::traits::{DiverterDriver, DiverterState, SelfTestResult};

/// Simulated diverter for testing.
pub struct SimulatedDriver;

impl DiverterDriver for SimulatedDriver {
    async fn init(&self, _id: &DiverterId) -> Result<(), MamutError> {
        Ok(())
    }

    async fn activate(&self, _id: &DiverterId, _target: Direction) -> Result<(), MamutError> {
        Ok(())
    }

    async fn deactivate(&self, _id: &DiverterId) -> Result<(), MamutError> {
        Ok(())
    }

    async fn get_status(&self, _id: &DiverterId) -> Result<DiverterState, MamutError> {
        Ok(DiverterState::Idle)
    }

    async fn self_test(&self, _id: &DiverterId) -> Result<SelfTestResult, MamutError> {
        Ok(SelfTestResult {
            passed: true,
            details: "simulated self-test passed".to_string(),
        })
    }
}
