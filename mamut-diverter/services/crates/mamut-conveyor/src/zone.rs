use mamut_proto::mamut::common::ZoneId;
use mamut_proto::mamut::conveyor::{ZoneState, ZoneStatus};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ZoneError {
    #[error("speed cannot be negative: {0}")]
    NegativeSpeed(f32),
    #[error("speed must be finite: {0}")]
    InvalidSpeed(f32),
    #[error("zone is in fault state")]
    Faulted,
}

#[derive(Debug, Clone)]
pub struct Zone {
    id: String,
    state: ZoneState,
    speed_mps: f32,
    item_present: bool,
    max_speed_mps: f32,
}

impl Zone {
    pub fn new(id: impl Into<String>, max_speed_mps: f32) -> Self {
        Self {
            id: id.into(),
            state: ZoneState::Idle,
            speed_mps: 0.0,
            item_present: false,
            max_speed_mps: max_speed_mps.max(0.0),
        }
    }

    pub fn status(&self) -> ZoneStatus {
        ZoneStatus {
            zone_id: Some(ZoneId {
                value: self.id.clone(),
            }),
            state: self.state as i32,
            speed_mps: self.speed_mps,
            item_present: self.item_present,
        }
    }

    pub fn set_speed(&mut self, requested_speed_mps: f32) -> Result<ZoneStatus, ZoneError> {
        if !requested_speed_mps.is_finite() {
            return Err(ZoneError::InvalidSpeed(requested_speed_mps));
        }
        if requested_speed_mps < 0.0 {
            return Err(ZoneError::NegativeSpeed(requested_speed_mps));
        }
        if self.state == ZoneState::Fault {
            return Err(ZoneError::Faulted);
        }

        let accepted_speed = requested_speed_mps.min(self.max_speed_mps);
        self.speed_mps = accepted_speed;

        self.state = if accepted_speed == 0.0 {
            match self.state {
                ZoneState::Running => ZoneState::Stopped,
                ZoneState::Idle => ZoneState::Idle,
                ZoneState::Stopped => ZoneState::Stopped,
                ZoneState::Unspecified => ZoneState::Idle,
                ZoneState::Fault => ZoneState::Fault,
            }
        } else {
            match self.state {
                ZoneState::Fault => ZoneState::Fault,
                ZoneState::Idle | ZoneState::Stopped | ZoneState::Running | ZoneState::Unspecified => {
                    ZoneState::Running
                }
            }
        };

        Ok(self.status())
    }

    /// Marks zone as faulted and forces speed to zero.
    pub fn mark_fault(&mut self) {
        self.state = ZoneState::Fault;
        self.speed_mps = 0.0;
    }

    /// Resets zone from fault back to idle state.
    pub fn reset(&mut self) {
        if self.state == ZoneState::Fault {
            self.state = ZoneState::Idle;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Zone, ZoneError};
    use mamut_proto::mamut::conveyor::ZoneState;

    #[test]
    fn speed_limits_are_enforced() {
        let mut zone = Zone::new("zone-1", 2.0);
        let status = zone.set_speed(3.5).expect("speed should be accepted with clamp");
        assert_eq!(status.speed_mps, 2.0);
        assert_eq!(status.state, ZoneState::Running as i32);
    }

    #[test]
    fn transitions_running_to_stopped_on_zero_speed() {
        let mut zone = Zone::new("zone-1", 2.0);
        zone.set_speed(1.0).expect("zone should start");
        let status = zone.set_speed(0.0).expect("zone should stop");
        assert_eq!(status.state, ZoneState::Stopped as i32);
        assert_eq!(status.speed_mps, 0.0);
    }

    #[test]
    fn negative_speed_is_rejected() {
        let mut zone = Zone::new("zone-1", 2.0);
        let err = zone
            .set_speed(-0.1)
            .expect_err("negative speed must be rejected");
        assert_eq!(err, ZoneError::NegativeSpeed(-0.1));
    }

    #[test]
    fn negative_zero_is_treated_as_zero() {
        let mut zone = Zone::new("zone-1", 2.0);
        let status = zone.set_speed(-0.0).expect("negative zero should be accepted");
        assert_eq!(status.speed_mps, 0.0);
        assert_eq!(status.state, ZoneState::Idle as i32);
    }

    #[test]
    fn faulted_zone_rejects_speed_change_until_reset() {
        let mut zone = Zone::new("zone-1", 2.0);
        zone.mark_fault();
        let err = zone
            .set_speed(1.0)
            .expect_err("faulted zone must reject speed command");
        assert_eq!(err, ZoneError::Faulted);

        zone.reset();
        let status = zone.set_speed(1.0).expect("zone should accept speed after reset");
        assert_eq!(status.state, ZoneState::Running as i32);
    }
}
