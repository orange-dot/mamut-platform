//! Fault scheduling strategies for coordinated fault injection.
//!
//! The scheduler determines when and which faults to inject based on
//! configurable strategies.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use rand::prelude::*;
use tracing::{debug, info, warn};

use crate::context::{FaultContext, FaultHandle};
use crate::error::{NemesisError, Result};
use crate::traits::Fault;

/// Strategy for selecting faults to inject.
#[derive(Debug, Clone)]
pub enum SchedulingStrategy {
    /// Select faults randomly with uniform distribution.
    Random {
        /// Optional seed for reproducibility.
        seed: Option<u64>,
    },

    /// Cycle through faults in order.
    RoundRobin,

    /// Apply multiple strategies in composition.
    Composed {
        /// Strategies to compose.
        strategies: Vec<Box<SchedulingStrategy>>,
        /// How to combine strategy selections.
        mode: CompositionMode,
    },

    /// Weight-based random selection.
    Weighted {
        /// Weights for each fault by name.
        weights: std::collections::HashMap<String, f64>,
    },

    /// Time-based scheduling.
    TimeBased {
        /// Schedule entries.
        schedule: Vec<ScheduleEntry>,
    },
}

/// How to combine multiple scheduling strategies.
#[derive(Debug, Clone, Copy)]
pub enum CompositionMode {
    /// Use the first strategy that returns a fault.
    FirstMatch,
    /// Select randomly from all strategy results.
    RandomSelect,
    /// Apply all strategies (for composed faults).
    All,
}

/// Entry in a time-based schedule.
#[derive(Debug, Clone)]
pub struct ScheduleEntry {
    /// Fault name to inject.
    pub fault_name: String,
    /// Time offset from start.
    pub offset: Duration,
    /// How long to keep the fault active.
    pub duration: Duration,
}

/// Scheduler for coordinating fault injection.
pub struct NemesisScheduler {
    /// Available faults.
    faults: Vec<Arc<dyn Fault>>,
    /// Scheduling strategy.
    strategy: SchedulingStrategy,
    /// Current position for round-robin.
    current_index: std::sync::atomic::AtomicUsize,
    /// Random number generator.
    rng: std::sync::Mutex<rand::rngs::StdRng>,
    /// Active fault handles.
    active_handles: std::sync::Mutex<Vec<FaultHandle>>,
    /// Configuration.
    config: SchedulerConfig,
}

/// Configuration for the scheduler.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Maximum number of concurrent faults.
    pub max_concurrent_faults: usize,
    /// Minimum interval between fault injections.
    pub min_interval: Duration,
    /// Maximum interval between fault injections.
    pub max_interval: Duration,
    /// Whether to auto-recover expired faults.
    pub auto_recover: bool,
    /// Default fault duration.
    pub default_duration: Duration,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_faults: 3,
            min_interval: Duration::from_secs(5),
            max_interval: Duration::from_secs(30),
            auto_recover: true,
            default_duration: Duration::from_secs(30),
        }
    }
}

impl NemesisScheduler {
    /// Creates a new scheduler with the given strategy.
    pub fn new(strategy: SchedulingStrategy) -> Self {
        let seed = match &strategy {
            SchedulingStrategy::Random { seed: Some(s) } => *s,
            _ => rand::thread_rng().r#gen(),
        };

        Self {
            faults: Vec::new(),
            strategy,
            current_index: std::sync::atomic::AtomicUsize::new(0),
            rng: std::sync::Mutex::new(rand::rngs::StdRng::seed_from_u64(seed)),
            active_handles: std::sync::Mutex::new(Vec::new()),
            config: SchedulerConfig::default(),
        }
    }

    /// Creates a scheduler with random strategy.
    pub fn random() -> Self {
        Self::new(SchedulingStrategy::Random { seed: None })
    }

    /// Creates a scheduler with random strategy and a specific seed.
    pub fn random_with_seed(seed: u64) -> Self {
        Self::new(SchedulingStrategy::Random { seed: Some(seed) })
    }

    /// Creates a scheduler with round-robin strategy.
    pub fn round_robin() -> Self {
        Self::new(SchedulingStrategy::RoundRobin)
    }

    /// Creates a composed scheduler.
    pub fn composed(strategies: Vec<SchedulingStrategy>, mode: CompositionMode) -> Self {
        Self::new(SchedulingStrategy::Composed {
            strategies: strategies.into_iter().map(Box::new).collect(),
            mode,
        })
    }

    /// Sets the scheduler configuration.
    pub fn with_config(mut self, config: SchedulerConfig) -> Self {
        self.config = config;
        self
    }

    /// Registers a fault with the scheduler.
    pub fn register_fault(&mut self, fault: Arc<dyn Fault>) {
        info!(fault_name = %fault.name(), "Registering fault with scheduler");
        self.faults.push(fault);
    }

    /// Registers multiple faults.
    pub fn register_faults(&mut self, faults: impl IntoIterator<Item = Arc<dyn Fault>>) {
        for fault in faults {
            self.register_fault(fault);
        }
    }

    /// Selects the next fault to inject based on the strategy.
    pub fn select_next(&self) -> Option<Arc<dyn Fault>> {
        if self.faults.is_empty() {
            return None;
        }

        match &self.strategy {
            SchedulingStrategy::Random { .. } => self.select_random(),
            SchedulingStrategy::RoundRobin => self.select_round_robin(),
            SchedulingStrategy::Weighted { weights } => self.select_weighted(weights),
            SchedulingStrategy::Composed { strategies, mode } => {
                self.select_composed(strategies, *mode)
            }
            SchedulingStrategy::TimeBased { .. } => {
                // Time-based selection requires knowing current time offset
                // Fall back to random for simple selection
                self.select_random()
            }
        }
    }

    /// Selects a fault randomly.
    fn select_random(&self) -> Option<Arc<dyn Fault>> {
        let mut rng = self.rng.lock().unwrap();
        self.faults.choose(&mut *rng).cloned()
    }

    /// Selects the next fault in round-robin order.
    fn select_round_robin(&self) -> Option<Arc<dyn Fault>> {
        let index = self
            .current_index
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let actual_index = index % self.faults.len();
        self.faults.get(actual_index).cloned()
    }

    /// Selects a fault based on weights.
    fn select_weighted(
        &self,
        weights: &std::collections::HashMap<String, f64>,
    ) -> Option<Arc<dyn Fault>> {
        let mut rng = self.rng.lock().unwrap();

        // Build weighted list
        let weighted_faults: Vec<_> = self
            .faults
            .iter()
            .map(|f| {
                let weight = weights.get(f.name()).copied().unwrap_or(1.0);
                (f.clone(), weight)
            })
            .collect();

        // Calculate total weight
        let total_weight: f64 = weighted_faults.iter().map(|(_, w)| w).sum();
        if total_weight <= 0.0 {
            return None;
        }

        // Select based on weight
        let mut selection = rng.r#gen::<f64>() * total_weight;
        for (fault, weight) in weighted_faults {
            selection -= weight;
            if selection <= 0.0 {
                return Some(fault);
            }
        }

        self.faults.last().cloned()
    }

    /// Selects using composed strategies.
    fn select_composed(
        &self,
        _strategies: &[Box<SchedulingStrategy>],
        mode: CompositionMode,
    ) -> Option<Arc<dyn Fault>> {
        // For simplicity, fall back to the underlying selection based on mode
        match mode {
            CompositionMode::FirstMatch | CompositionMode::RandomSelect => self.select_random(),
            CompositionMode::All => {
                // Return the first fault for "all" mode
                // The actual injection would handle multiple faults
                self.faults.first().cloned()
            }
        }
    }

    /// Injects a selected fault.
    pub async fn inject(&self, ctx: &FaultContext) -> Result<FaultHandle> {
        // Check concurrent fault limit
        {
            let handles = self.active_handles.lock().unwrap();
            if handles.len() >= self.config.max_concurrent_faults {
                return Err(NemesisError::SchedulerError(format!(
                    "Maximum concurrent faults ({}) reached",
                    self.config.max_concurrent_faults
                )));
            }
        }

        let fault = self
            .select_next()
            .ok_or_else(|| NemesisError::SchedulerError("No faults registered".to_string()))?;

        debug!(fault_name = %fault.name(), "Injecting fault");

        // Validate fault requirements
        fault.validate(ctx)?;

        // Inject the fault
        let handle = fault.inject(ctx).await?;

        // Track the handle
        {
            let mut handles = self.active_handles.lock().unwrap();
            handles.push(handle.clone());
        }

        info!(
            fault_name = %fault.name(),
            handle_id = %handle.id,
            "Fault injected successfully"
        );

        Ok(handle)
    }

    /// Injects a specific fault by name.
    pub async fn inject_by_name(&self, name: &str, ctx: &FaultContext) -> Result<FaultHandle> {
        let fault = self
            .faults
            .iter()
            .find(|f| f.name() == name)
            .ok_or_else(|| NemesisError::TargetNotFound(format!("Fault not found: {}", name)))?;

        fault.validate(ctx)?;
        let handle = fault.inject(ctx).await?;

        {
            let mut handles = self.active_handles.lock().unwrap();
            handles.push(handle.clone());
        }

        Ok(handle)
    }

    /// Recovers from a specific fault.
    pub async fn recover(&self, handle: FaultHandle) -> Result<()> {
        let fault = self
            .faults
            .iter()
            .find(|f| f.name() == handle.fault_name)
            .ok_or_else(|| {
                NemesisError::InvalidHandle(format!("Unknown fault type: {}", handle.fault_name))
            })?;

        debug!(fault_name = %fault.name(), handle_id = %handle.id, "Recovering from fault");

        fault.recover(handle.clone()).await?;

        // Remove from active handles
        {
            let mut handles = self.active_handles.lock().unwrap();
            handles.retain(|h| h.id != handle.id);
        }

        info!(fault_name = %fault.name(), "Fault recovered successfully");

        Ok(())
    }

    /// Recovers from all active faults.
    pub async fn recover_all(&self) -> Result<()> {
        let handles: Vec<FaultHandle> = {
            let handles = self.active_handles.lock().unwrap();
            handles.clone()
        };

        let mut errors = Vec::new();

        for handle in handles {
            if let Err(e) = self.recover(handle).await {
                warn!(error = %e, "Failed to recover fault");
                errors.push(e);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(NemesisError::RecoveryFailed(format!(
                "Failed to recover {} faults",
                errors.len()
            )))
        }
    }

    /// Cleans up expired faults.
    pub async fn cleanup_expired(&self) -> Result<Vec<FaultHandle>> {
        let expired: Vec<FaultHandle> = {
            let handles = self.active_handles.lock().unwrap();
            handles.iter().filter(|h| h.is_expired()).cloned().collect()
        };

        let mut recovered = Vec::new();

        for handle in expired {
            if self.config.auto_recover {
                if let Err(e) = self.recover(handle.clone()).await {
                    warn!(handle_id = %handle.id, error = %e, "Failed to auto-recover expired fault");
                } else {
                    recovered.push(handle);
                }
            }
        }

        Ok(recovered)
    }

    /// Returns the number of active faults.
    pub fn active_count(&self) -> usize {
        self.active_handles.lock().unwrap().len()
    }

    /// Returns all active fault handles.
    pub fn active_handles(&self) -> Vec<FaultHandle> {
        self.active_handles.lock().unwrap().clone()
    }

    /// Returns the number of registered faults.
    pub fn fault_count(&self) -> usize {
        self.faults.len()
    }

    /// Gets a random interval between min and max.
    pub fn random_interval(&self) -> Duration {
        let mut rng = self.rng.lock().unwrap();
        let min = self.config.min_interval.as_millis() as u64;
        let max = self.config.max_interval.as_millis() as u64;
        Duration::from_millis(rng.gen_range(min..=max))
    }
}

/// Builder for creating schedulers.
#[derive(Default)]
pub struct SchedulerBuilder {
    strategy: Option<SchedulingStrategy>,
    config: SchedulerConfig,
    faults: Vec<Arc<dyn Fault>>,
}

impl SchedulerBuilder {
    /// Creates a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the scheduling strategy.
    pub fn strategy(mut self, strategy: SchedulingStrategy) -> Self {
        self.strategy = Some(strategy);
        self
    }

    /// Sets the configuration.
    pub fn config(mut self, config: SchedulerConfig) -> Self {
        self.config = config;
        self
    }

    /// Sets the maximum concurrent faults.
    pub fn max_concurrent(mut self, max: usize) -> Self {
        self.config.max_concurrent_faults = max;
        self
    }

    /// Sets the interval range.
    pub fn interval(mut self, min: Duration, max: Duration) -> Self {
        self.config.min_interval = min;
        self.config.max_interval = max;
        self
    }

    /// Adds a fault.
    pub fn fault(mut self, fault: Arc<dyn Fault>) -> Self {
        self.faults.push(fault);
        self
    }

    /// Builds the scheduler.
    pub fn build(self) -> NemesisScheduler {
        let strategy = self.strategy.unwrap_or(SchedulingStrategy::Random { seed: None });
        let mut scheduler = NemesisScheduler::new(strategy).with_config(self.config);
        scheduler.register_faults(self.faults);
        scheduler
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::LinuxCapability;
    use crate::context::{FaultTarget, NetworkTarget};

    struct MockFault {
        name: String,
    }

    #[async_trait]
    impl Fault for MockFault {
        fn name(&self) -> &str {
            &self.name
        }

        fn required_capabilities(&self) -> Vec<LinuxCapability> {
            vec![]
        }

        async fn inject(&self, _ctx: &FaultContext) -> Result<FaultHandle> {
            Ok(FaultHandle::new(&self.name))
        }

        async fn recover(&self, _handle: FaultHandle) -> Result<()> {
            Ok(())
        }

        async fn is_active(&self, _handle: &FaultHandle) -> Result<bool> {
            Ok(true)
        }
    }

    #[test]
    fn test_random_scheduler() {
        let mut scheduler = NemesisScheduler::random_with_seed(42);
        scheduler.register_fault(Arc::new(MockFault {
            name: "fault1".to_string(),
        }));
        scheduler.register_fault(Arc::new(MockFault {
            name: "fault2".to_string(),
        }));

        let selected = scheduler.select_next();
        assert!(selected.is_some());
    }

    #[test]
    fn test_round_robin_scheduler() {
        let mut scheduler = NemesisScheduler::round_robin();
        scheduler.register_fault(Arc::new(MockFault {
            name: "fault1".to_string(),
        }));
        scheduler.register_fault(Arc::new(MockFault {
            name: "fault2".to_string(),
        }));
        scheduler.register_fault(Arc::new(MockFault {
            name: "fault3".to_string(),
        }));

        assert_eq!(scheduler.select_next().unwrap().name(), "fault1");
        assert_eq!(scheduler.select_next().unwrap().name(), "fault2");
        assert_eq!(scheduler.select_next().unwrap().name(), "fault3");
        assert_eq!(scheduler.select_next().unwrap().name(), "fault1");
    }

    #[tokio::test]
    async fn test_inject_and_recover() {
        let mut scheduler = NemesisScheduler::random();
        scheduler.register_fault(Arc::new(MockFault {
            name: "test".to_string(),
        }));

        let ctx = FaultContext::builder()
            .target(FaultTarget::Network(NetworkTarget::new()))
            .build();

        let handle = scheduler.inject(&ctx).await.unwrap();
        assert_eq!(scheduler.active_count(), 1);

        scheduler.recover(handle).await.unwrap();
        assert_eq!(scheduler.active_count(), 0);
    }

    #[test]
    fn test_scheduler_builder() {
        let scheduler = SchedulerBuilder::new()
            .strategy(SchedulingStrategy::RoundRobin)
            .max_concurrent(5)
            .interval(Duration::from_secs(1), Duration::from_secs(10))
            .build();

        assert_eq!(scheduler.config.max_concurrent_faults, 5);
    }
}
