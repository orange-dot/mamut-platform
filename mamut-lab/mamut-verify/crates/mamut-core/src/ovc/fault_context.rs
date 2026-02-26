//! Fault Context - Time fault tracking for verification.
//!
//! `TimeFaultContext` captures information about time-related faults that
//! have been injected or detected in the system under test. This enables
//! verification of how systems handle clock anomalies.

use serde::{Deserialize, Serialize};
use std::fmt;

use super::controller_time::ControllerTime;
use super::observed_time::NodeId;

/// Types of time-related faults that can occur or be injected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TimeFaultType {
    /// Clock jumped forward significantly.
    ClockSkewForward = 0,

    /// Clock jumped backward (NTP correction, manual adjustment, etc.).
    ClockSkewBackward = 1,

    /// Clock is drifting faster than expected.
    ClockDriftFast = 2,

    /// Clock is drifting slower than expected.
    ClockDriftSlow = 3,

    /// Clock stopped advancing (frozen).
    ClockFrozen = 4,

    /// NTP synchronization failure.
    NtpSyncFailure = 5,

    /// Leap second occurred.
    LeapSecond = 6,

    /// Time zone change.
    TimezoneChange = 7,

    /// Daylight saving time transition.
    DstTransition = 8,

    /// Hardware clock failure.
    HardwareClockFailure = 9,

    /// Virtualization-related time anomaly.
    VirtualizationAnomaly = 10,

    /// Network time protocol (PTP) fault.
    PtpFault = 11,

    /// Clock source switchover.
    ClockSourceChange = 12,

    /// Injected fault for testing.
    InjectedFault = 13,

    /// Unknown or unclassified fault.
    Unknown = 255,
}

impl TimeFaultType {
    /// Returns a human-readable description of the fault type.
    pub const fn description(&self) -> &'static str {
        match self {
            TimeFaultType::ClockSkewForward => "Clock jumped forward",
            TimeFaultType::ClockSkewBackward => "Clock jumped backward",
            TimeFaultType::ClockDriftFast => "Clock drifting fast",
            TimeFaultType::ClockDriftSlow => "Clock drifting slow",
            TimeFaultType::ClockFrozen => "Clock frozen",
            TimeFaultType::NtpSyncFailure => "NTP synchronization failure",
            TimeFaultType::LeapSecond => "Leap second occurred",
            TimeFaultType::TimezoneChange => "Timezone change",
            TimeFaultType::DstTransition => "DST transition",
            TimeFaultType::HardwareClockFailure => "Hardware clock failure",
            TimeFaultType::VirtualizationAnomaly => "Virtualization time anomaly",
            TimeFaultType::PtpFault => "PTP fault",
            TimeFaultType::ClockSourceChange => "Clock source changed",
            TimeFaultType::InjectedFault => "Injected fault",
            TimeFaultType::Unknown => "Unknown fault",
        }
    }

    /// Returns whether this fault type represents a backward time jump.
    pub const fn is_backward_jump(&self) -> bool {
        matches!(self, TimeFaultType::ClockSkewBackward)
    }

    /// Returns whether this fault type represents a forward time jump.
    pub const fn is_forward_jump(&self) -> bool {
        matches!(self, TimeFaultType::ClockSkewForward)
    }

    /// Returns whether this fault type is related to clock drift.
    pub const fn is_drift(&self) -> bool {
        matches!(
            self,
            TimeFaultType::ClockDriftFast | TimeFaultType::ClockDriftSlow
        )
    }

    /// Returns whether this is an injected (artificial) fault.
    pub const fn is_injected(&self) -> bool {
        matches!(self, TimeFaultType::InjectedFault)
    }

    /// Converts to a compact u32 representation for storage.
    pub const fn to_compact(&self) -> u32 {
        *self as u32
    }

    /// Creates from a compact u32 representation.
    pub fn from_compact(value: u32) -> Self {
        match value {
            0 => TimeFaultType::ClockSkewForward,
            1 => TimeFaultType::ClockSkewBackward,
            2 => TimeFaultType::ClockDriftFast,
            3 => TimeFaultType::ClockDriftSlow,
            4 => TimeFaultType::ClockFrozen,
            5 => TimeFaultType::NtpSyncFailure,
            6 => TimeFaultType::LeapSecond,
            7 => TimeFaultType::TimezoneChange,
            8 => TimeFaultType::DstTransition,
            9 => TimeFaultType::HardwareClockFailure,
            10 => TimeFaultType::VirtualizationAnomaly,
            11 => TimeFaultType::PtpFault,
            12 => TimeFaultType::ClockSourceChange,
            13 => TimeFaultType::InjectedFault,
            _ => TimeFaultType::Unknown,
        }
    }
}

impl fmt::Display for TimeFaultType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Severity level of a time fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FaultSeverity {
    /// Informational - no impact on correctness.
    Info,
    /// Warning - potential impact, should be investigated.
    Warning,
    /// Error - likely correctness impact.
    Error,
    /// Critical - definite correctness impact.
    Critical,
}

impl fmt::Display for FaultSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FaultSeverity::Info => write!(f, "INFO"),
            FaultSeverity::Warning => write!(f, "WARN"),
            FaultSeverity::Error => write!(f, "ERROR"),
            FaultSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Context about a time-related fault.
///
/// This structure captures comprehensive information about a time fault,
/// including when and where it occurred, its type and severity, and
/// the magnitude of the time discrepancy.
///
/// # Example
///
/// ```
/// use mamut_core::ovc::{TimeFaultContext, TimeFaultType, FaultSeverity, ControllerTime, NodeId};
///
/// let fault = TimeFaultContext::new(
///     TimeFaultType::ClockSkewBackward,
///     NodeId::new(1),
///     ControllerTime::from_secs(100),
/// )
/// .with_magnitude_nanos(-5_000_000_000) // 5 second backward jump
/// .with_severity(FaultSeverity::Error);
///
/// assert!(fault.fault_type().is_backward_jump());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeFaultContext {
    /// The type of fault that occurred.
    fault_type: TimeFaultType,

    /// The node where the fault was observed.
    affected_node: NodeId,

    /// Controller time when the fault was detected.
    detection_time: ControllerTime,

    /// Magnitude of the time discrepancy in nanoseconds.
    /// Positive means time jumped forward, negative means backward.
    magnitude_nanos: Option<i128>,

    /// Severity of the fault.
    severity: FaultSeverity,

    /// Optional description providing additional context.
    description: Option<String>,

    /// Whether this fault was intentionally injected for testing.
    injected: bool,

    /// Optional correlation ID for linking related faults.
    correlation_id: Option<u64>,
}

impl TimeFaultContext {
    /// Creates a new fault context.
    pub fn new(
        fault_type: TimeFaultType,
        affected_node: NodeId,
        detection_time: ControllerTime,
    ) -> Self {
        TimeFaultContext {
            fault_type,
            affected_node,
            detection_time,
            magnitude_nanos: None,
            severity: FaultSeverity::Warning,
            description: None,
            injected: fault_type == TimeFaultType::InjectedFault,
            correlation_id: None,
        }
    }

    /// Creates a fault context for an injected fault.
    pub fn injected(
        fault_type: TimeFaultType,
        affected_node: NodeId,
        detection_time: ControllerTime,
    ) -> Self {
        let mut ctx = Self::new(fault_type, affected_node, detection_time);
        ctx.injected = true;
        ctx
    }

    /// Sets the magnitude of the time discrepancy.
    #[inline]
    pub fn with_magnitude_nanos(mut self, nanos: i128) -> Self {
        self.magnitude_nanos = Some(nanos);
        self
    }

    /// Sets the severity level.
    #[inline]
    pub fn with_severity(mut self, severity: FaultSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Sets a description for additional context.
    #[inline]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the correlation ID.
    #[inline]
    pub fn with_correlation_id(mut self, id: u64) -> Self {
        self.correlation_id = Some(id);
        self
    }

    /// Returns the fault type.
    #[inline]
    pub const fn fault_type(&self) -> TimeFaultType {
        self.fault_type
    }

    /// Returns the affected node.
    #[inline]
    pub const fn affected_node(&self) -> NodeId {
        self.affected_node
    }

    /// Returns the detection time.
    #[inline]
    pub const fn detection_time(&self) -> ControllerTime {
        self.detection_time
    }

    /// Returns the magnitude in nanoseconds, if known.
    #[inline]
    pub const fn magnitude_nanos(&self) -> Option<i128> {
        self.magnitude_nanos
    }

    /// Returns the severity.
    #[inline]
    pub const fn severity(&self) -> FaultSeverity {
        self.severity
    }

    /// Returns the description, if any.
    #[inline]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns whether this fault was injected.
    #[inline]
    pub const fn is_injected(&self) -> bool {
        self.injected
    }

    /// Returns the correlation ID, if any.
    #[inline]
    pub const fn correlation_id(&self) -> Option<u64> {
        self.correlation_id
    }

    /// Returns the absolute magnitude in nanoseconds.
    #[inline]
    pub fn abs_magnitude_nanos(&self) -> Option<u128> {
        self.magnitude_nanos.map(|m| m.unsigned_abs())
    }

    /// Returns the magnitude as a human-readable duration string.
    pub fn magnitude_display(&self) -> Option<String> {
        self.magnitude_nanos.map(|nanos| {
            let abs_nanos = nanos.unsigned_abs();
            let sign = if nanos < 0 { "-" } else { "+" };

            if abs_nanos >= 1_000_000_000 {
                format!("{}{}s", sign, abs_nanos / 1_000_000_000)
            } else if abs_nanos >= 1_000_000 {
                format!("{}{}ms", sign, abs_nanos / 1_000_000)
            } else if abs_nanos >= 1_000 {
                format!("{}{}us", sign, abs_nanos / 1_000)
            } else {
                format!("{}{}ns", sign, abs_nanos)
            }
        })
    }

    /// Converts to a compact u32 representation (fault type only).
    ///
    /// This is useful for storage-efficient serialization where only
    /// the fault type needs to be preserved.
    #[inline]
    pub fn to_compact(&self) -> u32 {
        self.fault_type.to_compact()
    }
}

impl fmt::Display for TimeFaultContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} on {} at {}",
            self.severity, self.fault_type, self.affected_node, self.detection_time
        )?;

        if let Some(mag) = self.magnitude_display() {
            write!(f, " (magnitude: {})", mag)?;
        }

        if self.injected {
            write!(f, " [INJECTED]")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fault_type_description() {
        assert_eq!(
            TimeFaultType::ClockSkewBackward.description(),
            "Clock jumped backward"
        );
        assert!(TimeFaultType::ClockSkewBackward.is_backward_jump());
        assert!(!TimeFaultType::ClockSkewBackward.is_forward_jump());
    }

    #[test]
    fn test_fault_type_compact() {
        for i in 0..=13 {
            let fault_type = TimeFaultType::from_compact(i);
            assert_eq!(fault_type.to_compact(), i);
        }

        assert_eq!(TimeFaultType::from_compact(999), TimeFaultType::Unknown);
    }

    #[test]
    fn test_fault_context_creation() {
        let fault = TimeFaultContext::new(
            TimeFaultType::ClockSkewBackward,
            NodeId::new(1),
            ControllerTime::from_secs(100),
        );

        assert_eq!(fault.fault_type(), TimeFaultType::ClockSkewBackward);
        assert_eq!(fault.affected_node(), NodeId::new(1));
        assert!(!fault.is_injected());
    }

    #[test]
    fn test_fault_context_injected() {
        let fault = TimeFaultContext::injected(
            TimeFaultType::ClockSkewForward,
            NodeId::new(2),
            ControllerTime::from_secs(50),
        );

        assert!(fault.is_injected());
    }

    #[test]
    fn test_fault_context_builder() {
        let fault = TimeFaultContext::new(
            TimeFaultType::ClockDriftFast,
            NodeId::new(1),
            ControllerTime::from_secs(100),
        )
        .with_magnitude_nanos(5_000_000_000)
        .with_severity(FaultSeverity::Error)
        .with_description("Test fault")
        .with_correlation_id(12345);

        assert_eq!(fault.magnitude_nanos(), Some(5_000_000_000));
        assert_eq!(fault.severity(), FaultSeverity::Error);
        assert_eq!(fault.description(), Some("Test fault"));
        assert_eq!(fault.correlation_id(), Some(12345));
    }

    #[test]
    fn test_magnitude_display() {
        let fault = TimeFaultContext::new(
            TimeFaultType::ClockSkewForward,
            NodeId::new(1),
            ControllerTime::from_secs(0),
        )
        .with_magnitude_nanos(5_500_000_000);

        assert_eq!(fault.magnitude_display(), Some("+5s".to_string()));

        let fault_back = fault.clone().with_magnitude_nanos(-100_000_000);
        assert_eq!(fault_back.magnitude_display(), Some("-100ms".to_string()));
    }

    #[test]
    fn test_severity_ordering() {
        assert!(FaultSeverity::Info < FaultSeverity::Warning);
        assert!(FaultSeverity::Warning < FaultSeverity::Error);
        assert!(FaultSeverity::Error < FaultSeverity::Critical);
    }

    #[test]
    fn test_display() {
        let fault = TimeFaultContext::new(
            TimeFaultType::NtpSyncFailure,
            NodeId::new(3),
            ControllerTime::from_secs(1000),
        )
        .with_severity(FaultSeverity::Warning);

        let display = format!("{}", fault);
        assert!(display.contains("WARN"));
        assert!(display.contains("NTP"));
        assert!(display.contains("node:3"));
    }

    #[test]
    fn test_serde() {
        let fault = TimeFaultContext::new(
            TimeFaultType::ClockFrozen,
            NodeId::new(5),
            ControllerTime::from_secs(500),
        )
        .with_magnitude_nanos(0)
        .with_severity(FaultSeverity::Critical);

        let json = serde_json::to_string(&fault).unwrap();
        let fault2: TimeFaultContext = serde_json::from_str(&json).unwrap();

        assert_eq!(fault, fault2);
    }
}
