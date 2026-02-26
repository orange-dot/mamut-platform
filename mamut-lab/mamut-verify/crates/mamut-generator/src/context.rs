//! Generator context providing runtime information for operation generation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use mamut_core::ProcessId;

/// The type of fault currently active in the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FaultType {
    /// Network partition between processes
    NetworkPartition,
    /// Process crash
    ProcessCrash,
    /// Message delay
    MessageDelay,
    /// Message loss
    MessageLoss,
    /// Clock skew
    ClockSkew,
    /// Disk failure
    DiskFailure,
    /// Custom fault type
    Custom(String),
}

impl std::fmt::Display for FaultType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FaultType::NetworkPartition => write!(f, "NetworkPartition"),
            FaultType::ProcessCrash => write!(f, "ProcessCrash"),
            FaultType::MessageDelay => write!(f, "MessageDelay"),
            FaultType::MessageLoss => write!(f, "MessageLoss"),
            FaultType::ClockSkew => write!(f, "ClockSkew"),
            FaultType::DiskFailure => write!(f, "DiskFailure"),
            FaultType::Custom(name) => write!(f, "Custom({})", name),
        }
    }
}

/// An active fault in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveFault {
    /// The type of fault
    pub fault_type: FaultType,
    /// Processes affected by this fault
    pub affected_processes: Vec<ProcessId>,
    /// When the fault started (logical timestamp)
    pub start_time: u64,
    /// When the fault will end (if known)
    pub end_time: Option<u64>,
    /// Additional fault parameters
    pub parameters: HashMap<String, String>,
}

impl ActiveFault {
    /// Create a new active fault
    pub fn new(fault_type: FaultType, affected: Vec<ProcessId>, start_time: u64) -> Self {
        Self {
            fault_type,
            affected_processes: affected,
            start_time,
            end_time: None,
            parameters: HashMap::new(),
        }
    }

    /// Set the end time for this fault
    pub fn with_end_time(mut self, end_time: u64) -> Self {
        self.end_time = Some(end_time);
        self
    }

    /// Add a parameter to this fault
    pub fn with_parameter(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.parameters.insert(key.into(), value.into());
        self
    }

    /// Check if a process is affected by this fault
    pub fn affects(&self, process: ProcessId) -> bool {
        self.affected_processes.contains(&process)
    }

    /// Check if the fault is still active at the given time
    pub fn is_active_at(&self, time: u64) -> bool {
        time >= self.start_time && self.end_time.map_or(true, |end| time < end)
    }
}

/// Summary statistics about the execution history
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistorySummary {
    /// Total number of operations executed
    pub total_operations: u64,
    /// Number of successful operations
    pub successful_operations: u64,
    /// Number of failed operations
    pub failed_operations: u64,
    /// Operations per process
    pub operations_per_process: HashMap<ProcessId, u64>,
    /// Operation type counts (if categorized)
    pub operation_type_counts: HashMap<String, u64>,
    /// Current logical timestamp
    pub current_timestamp: u64,
    /// Number of invariant checks performed
    pub invariant_checks: u64,
    /// Number of invariant violations found
    pub invariant_violations: u64,
}

impl HistorySummary {
    /// Create a new empty history summary
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an operation execution
    pub fn record_operation(&mut self, process: ProcessId, success: bool, op_type: Option<&str>) {
        self.total_operations += 1;
        if success {
            self.successful_operations += 1;
        } else {
            self.failed_operations += 1;
        }
        *self.operations_per_process.entry(process).or_insert(0) += 1;
        if let Some(op_type) = op_type {
            *self.operation_type_counts.entry(op_type.to_string()).or_insert(0) += 1;
        }
        self.current_timestamp += 1;
    }

    /// Record an invariant check
    pub fn record_invariant_check(&mut self, violated: bool) {
        self.invariant_checks += 1;
        if violated {
            self.invariant_violations += 1;
        }
    }

    /// Get the success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.total_operations == 0 {
            1.0
        } else {
            self.successful_operations as f64 / self.total_operations as f64
        }
    }

    /// Get the violation rate (0.0 to 1.0)
    pub fn violation_rate(&self) -> f64 {
        if self.invariant_checks == 0 {
            0.0
        } else {
            self.invariant_violations as f64 / self.invariant_checks as f64
        }
    }
}

/// Runtime context available to generators during operation generation
///
/// The context provides generators with information about the current state
/// of the verification run, including model state, history statistics, and
/// active faults.
#[derive(Debug, Clone)]
pub struct GeneratorContext {
    /// Opaque model state (serialized)
    model_state: Option<Vec<u8>>,
    /// Summary of the execution history
    history: HistorySummary,
    /// Currently active faults
    active_faults: Vec<ActiveFault>,
    /// Available process IDs
    available_processes: Vec<ProcessId>,
    /// Custom context data
    custom_data: HashMap<String, String>,
    /// Current verification round
    round: u64,
    /// Maximum operations limit
    max_operations: Option<u64>,
}

impl GeneratorContext {
    /// Create a new generator context
    pub fn new() -> Self {
        Self {
            model_state: None,
            history: HistorySummary::new(),
            active_faults: Vec::new(),
            available_processes: Vec::new(),
            custom_data: HashMap::new(),
            round: 0,
            max_operations: None,
        }
    }

    /// Create a context with the given processes
    pub fn with_processes(processes: impl IntoIterator<Item = ProcessId>) -> Self {
        Self {
            available_processes: processes.into_iter().collect(),
            ..Self::new()
        }
    }

    /// Set the model state
    pub fn set_model_state<S: Serialize>(&mut self, state: &S) -> Result<(), serde_json::Error> {
        self.model_state = Some(serde_json::to_vec(state)?);
        Ok(())
    }

    /// Get the model state
    pub fn get_model_state<S: serde::de::DeserializeOwned>(&self) -> Option<Result<S, serde_json::Error>> {
        self.model_state.as_ref().map(|data| serde_json::from_slice(data))
    }

    /// Get the raw model state bytes
    pub fn model_state_bytes(&self) -> Option<&[u8]> {
        self.model_state.as_deref()
    }

    /// Get the history summary
    pub fn history(&self) -> &HistorySummary {
        &self.history
    }

    /// Get a mutable reference to the history summary
    pub fn history_mut(&mut self) -> &mut HistorySummary {
        &mut self.history
    }

    /// Get the active faults
    pub fn active_faults(&self) -> &[ActiveFault] {
        &self.active_faults
    }

    /// Add an active fault
    pub fn add_fault(&mut self, fault: ActiveFault) {
        self.active_faults.push(fault);
    }

    /// Remove faults that have ended at the given time
    pub fn expire_faults(&mut self, time: u64) {
        self.active_faults.retain(|f| f.is_active_at(time));
    }

    /// Check if a process is currently affected by any fault
    pub fn is_process_faulted(&self, process: ProcessId) -> bool {
        self.active_faults.iter().any(|f| f.affects(process))
    }

    /// Get faults affecting a specific process
    pub fn faults_for_process(&self, process: ProcessId) -> Vec<&ActiveFault> {
        self.active_faults.iter().filter(|f| f.affects(process)).collect()
    }

    /// Get the available processes
    pub fn available_processes(&self) -> &[ProcessId] {
        &self.available_processes
    }

    /// Get non-faulted processes
    pub fn healthy_processes(&self) -> Vec<ProcessId> {
        self.available_processes
            .iter()
            .copied()
            .filter(|&p| !self.is_process_faulted(p))
            .collect()
    }

    /// Set custom data
    pub fn set_custom(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.custom_data.insert(key.into(), value.into());
    }

    /// Get custom data
    pub fn get_custom(&self, key: &str) -> Option<&str> {
        self.custom_data.get(key).map(|s| s.as_str())
    }

    /// Get the current round
    pub fn round(&self) -> u64 {
        self.round
    }

    /// Increment the round
    pub fn next_round(&mut self) {
        self.round += 1;
    }

    /// Set the maximum operations limit
    pub fn set_max_operations(&mut self, max: u64) {
        self.max_operations = Some(max);
    }

    /// Get the maximum operations limit
    pub fn max_operations(&self) -> Option<u64> {
        self.max_operations
    }

    /// Check if we've reached the operation limit
    pub fn is_at_limit(&self) -> bool {
        self.max_operations
            .map_or(false, |max| self.history.total_operations >= max)
    }

    /// Get remaining operations until limit
    pub fn remaining_operations(&self) -> Option<u64> {
        self.max_operations
            .map(|max| max.saturating_sub(self.history.total_operations))
    }
}

impl Default for GeneratorContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_fault() {
        let fault = ActiveFault::new(
            FaultType::NetworkPartition,
            vec![ProcessId::new(1), ProcessId::new(2)],
            100,
        )
        .with_end_time(200)
        .with_parameter("partition_id", "p1");

        assert!(fault.affects(ProcessId::new(1)));
        assert!(fault.affects(ProcessId::new(2)));
        assert!(!fault.affects(ProcessId::new(3)));

        assert!(fault.is_active_at(100));
        assert!(fault.is_active_at(150));
        assert!(!fault.is_active_at(200));
        assert!(!fault.is_active_at(99));
    }

    #[test]
    fn test_history_summary() {
        let mut summary = HistorySummary::new();

        summary.record_operation(ProcessId::new(1), true, Some("read"));
        summary.record_operation(ProcessId::new(1), true, Some("write"));
        summary.record_operation(ProcessId::new(2), false, Some("read"));

        assert_eq!(summary.total_operations, 3);
        assert_eq!(summary.successful_operations, 2);
        assert_eq!(summary.failed_operations, 1);
        assert_eq!(summary.operations_per_process.get(&ProcessId::new(1)), Some(&2));
        assert_eq!(summary.operation_type_counts.get("read"), Some(&2));

        assert!((summary.success_rate() - 0.666).abs() < 0.01);

        summary.record_invariant_check(false);
        summary.record_invariant_check(true);
        assert_eq!(summary.violation_rate(), 0.5);
    }

    #[test]
    fn test_generator_context() {
        let mut ctx = GeneratorContext::with_processes(vec![
            ProcessId::new(1),
            ProcessId::new(2),
            ProcessId::new(3),
        ]);

        ctx.add_fault(ActiveFault::new(
            FaultType::ProcessCrash,
            vec![ProcessId::new(2)],
            0,
        ));

        assert!(ctx.is_process_faulted(ProcessId::new(2)));
        assert!(!ctx.is_process_faulted(ProcessId::new(1)));

        let healthy = ctx.healthy_processes();
        assert_eq!(healthy.len(), 2);
        assert!(healthy.contains(&ProcessId::new(1)));
        assert!(healthy.contains(&ProcessId::new(3)));
    }

    #[test]
    fn test_model_state() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct TestState {
            counter: u32,
            name: String,
        }

        let mut ctx = GeneratorContext::new();
        let state = TestState {
            counter: 42,
            name: "test".to_string(),
        };

        ctx.set_model_state(&state).unwrap();
        let retrieved: TestState = ctx.get_model_state().unwrap().unwrap();
        assert_eq!(retrieved, state);
    }
}
