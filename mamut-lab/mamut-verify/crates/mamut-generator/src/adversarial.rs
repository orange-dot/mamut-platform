//! Adversarial generator targeting specific invariants

use async_trait::async_trait;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;

use mamut_core::ProcessId;

use crate::context::GeneratorContext;
use crate::generated::{GeneratedOp, OperationMetadata};
use crate::traits::Generator;

/// Target invariant for adversarial generation
pub struct InvariantTarget<V> {
    /// Name of the invariant
    pub name: String,
    /// Description of what the invariant checks
    pub description: String,
    /// Operations that might violate this invariant
    pub challenging_ops: Vec<Box<dyn Fn(&mut ChaCha8Rng) -> V + Send + Sync>>,
    /// Weight for this invariant
    pub weight: f64,
    /// Historical violation count
    pub violations: u64,
    /// Historical attempt count
    pub attempts: u64,
}

impl<V> std::fmt::Debug for InvariantTarget<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InvariantTarget")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("weight", &self.weight)
            .field("violations", &self.violations)
            .field("attempts", &self.attempts)
            .finish()
    }
}

impl<V> InvariantTarget<V> {
    /// Create a new invariant target
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            challenging_ops: Vec::new(),
            weight: 1.0,
            violations: 0,
            attempts: 0,
        }
    }

    /// Add a challenging operation
    pub fn with_challenger<F>(mut self, factory: F) -> Self
    where
        F: Fn(&mut ChaCha8Rng) -> V + Send + Sync + 'static,
    {
        self.challenging_ops.push(Box::new(factory));
        self
    }

    /// Set the weight
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    /// Get the violation rate
    pub fn violation_rate(&self) -> f64 {
        if self.attempts == 0 {
            0.0
        } else {
            self.violations as f64 / self.attempts as f64
        }
    }

    /// Record a violation attempt
    pub fn record_attempt(&mut self, violated: bool) {
        self.attempts += 1;
        if violated {
            self.violations += 1;
        }
    }

    /// Generate a challenging operation
    pub fn generate(&self, rng: &mut ChaCha8Rng) -> Option<V> {
        if self.challenging_ops.is_empty() {
            return None;
        }
        let idx = rng.gen_range(0..self.challenging_ops.len());
        Some((self.challenging_ops[idx])(rng))
    }
}

/// Strategy for targeting invariants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TargetingStrategy {
    /// Random selection among invariants
    #[default]
    Random,
    /// Focus on invariants with highest violation rate
    HighViolation,
    /// Focus on invariants with lowest violation rate (exploration)
    LowViolation,
    /// Round-robin through invariants
    RoundRobin,
    /// Adaptive based on recent violations
    Adaptive,
}

/// Adversarial generator that targets specific invariants
///
/// This generator produces operations designed to challenge system invariants
/// and find edge cases. It learns from previous violations to focus on
/// promising attack vectors.
pub struct AdversarialGenerator<V> {
    /// Random number generator
    rng: ChaCha8Rng,
    /// Initial seed
    initial_seed: u64,
    /// Target invariants
    targets: Vec<InvariantTarget<V>>,
    /// Targeting strategy
    strategy: TargetingStrategy,
    /// Current round-robin index
    round_robin_idx: usize,
    /// Recent violation history (for adaptive)
    recent_violations: HashMap<String, Vec<bool>>,
    /// Window size for adaptive strategy
    adaptive_window: usize,
    /// Maximum operations
    max_operations: Option<u64>,
    /// Generated count
    generated_count: u64,
    /// Sequence counter
    sequence: u64,
    /// Generator name
    name: String,
    /// Fallback generator for when no targets apply
    fallback: Option<Box<dyn Fn(&mut ChaCha8Rng) -> V + Send + Sync>>,
    /// Exploration probability (random operation instead of targeted)
    exploration_rate: f64,
    /// Phantom
    _phantom: PhantomData<V>,
}

impl<V> AdversarialGenerator<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + Debug + 'static,
{
    /// Create a new adversarial generator
    pub fn new(seed: u64) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            initial_seed: seed,
            targets: Vec::new(),
            strategy: TargetingStrategy::default(),
            round_robin_idx: 0,
            recent_violations: HashMap::new(),
            adaptive_window: 10,
            max_operations: None,
            generated_count: 0,
            sequence: 0,
            name: "AdversarialGenerator".to_string(),
            fallback: None,
            exploration_rate: 0.1,
            _phantom: PhantomData,
        }
    }

    /// Set the generator name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set maximum operations
    pub fn with_max_operations(mut self, max: u64) -> Self {
        self.max_operations = Some(max);
        self
    }

    /// Add a target invariant
    pub fn with_target(mut self, target: InvariantTarget<V>) -> Self {
        self.recent_violations.insert(target.name.clone(), Vec::new());
        self.targets.push(target);
        self
    }

    /// Add multiple target invariants
    pub fn with_targets(mut self, targets: impl IntoIterator<Item = InvariantTarget<V>>) -> Self {
        for target in targets {
            self.recent_violations.insert(target.name.clone(), Vec::new());
            self.targets.push(target);
        }
        self
    }

    /// Set the targeting strategy
    pub fn with_strategy(mut self, strategy: TargetingStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set the adaptive window size
    pub fn with_adaptive_window(mut self, window: usize) -> Self {
        self.adaptive_window = window;
        self
    }

    /// Set the exploration rate
    pub fn with_exploration_rate(mut self, rate: f64) -> Self {
        self.exploration_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Set a fallback operation generator
    pub fn with_fallback<F>(mut self, fallback: F) -> Self
    where
        F: Fn(&mut ChaCha8Rng) -> V + Send + Sync + 'static,
    {
        self.fallback = Some(Box::new(fallback));
        self
    }

    /// Get statistics for a target
    pub fn target_stats(&self, name: &str) -> Option<(u64, u64, f64)> {
        self.targets.iter().find(|t| t.name == name).map(|t| {
            (t.attempts, t.violations, t.violation_rate())
        })
    }

    /// Get all target statistics
    pub fn all_stats(&self) -> Vec<(&str, u64, u64, f64)> {
        self.targets
            .iter()
            .map(|t| (t.name.as_str(), t.attempts, t.violations, t.violation_rate()))
            .collect()
    }

    /// Select a target based on the current strategy
    fn select_target(&mut self) -> Option<usize> {
        if self.targets.is_empty() {
            return None;
        }

        let idx = match self.strategy {
            TargetingStrategy::Random => self.rng.gen_range(0..self.targets.len()),

            TargetingStrategy::HighViolation => {
                self.targets
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| {
                        a.violation_rate()
                            .partial_cmp(&b.violation_rate())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            }

            TargetingStrategy::LowViolation => {
                // Prefer unexplored targets
                if let Some((i, _)) = self.targets.iter().enumerate().find(|(_, t)| t.attempts == 0)
                {
                    i
                } else {
                    self.targets
                        .iter()
                        .enumerate()
                        .min_by(|(_, a), (_, b)| {
                            a.violation_rate()
                                .partial_cmp(&b.violation_rate())
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .map(|(i, _)| i)
                        .unwrap_or(0)
                }
            }

            TargetingStrategy::RoundRobin => {
                let idx = self.round_robin_idx;
                self.round_robin_idx = (self.round_robin_idx + 1) % self.targets.len();
                idx
            }

            TargetingStrategy::Adaptive => {
                // Calculate adaptive weights based on recent violations
                let weights: Vec<f64> = self
                    .targets
                    .iter()
                    .map(|t| {
                        let recent = self.recent_violations.get(&t.name);
                        let recent_rate = recent
                            .map(|v| {
                                if v.is_empty() {
                                    0.5 // Unknown = medium priority
                                } else {
                                    v.iter().filter(|&&x| x).count() as f64 / v.len() as f64
                                }
                            })
                            .unwrap_or(0.5);

                        // Higher weight for moderate violation rates (more interesting)
                        let interest = 4.0 * recent_rate * (1.0 - recent_rate);
                        (interest + 0.1) * t.weight
                    })
                    .collect();

                let total: f64 = weights.iter().sum();
                if total <= 0.0 {
                    self.rng.gen_range(0..self.targets.len())
                } else {
                    let mut roll = self.rng.r#gen::<f64>() * total;
                    let mut selected = 0;
                    for (i, w) in weights.iter().enumerate() {
                        roll -= w;
                        if roll <= 0.0 {
                            selected = i;
                            break;
                        }
                    }
                    selected
                }
            }
        };

        Some(idx)
    }

    /// Record a violation for adaptive strategy
    pub fn record_violation(&mut self, target_name: &str, violated: bool) {
        if let Some(target) = self.targets.iter_mut().find(|t| t.name == target_name) {
            target.record_attempt(violated);
        }

        if let Some(history) = self.recent_violations.get_mut(target_name) {
            history.push(violated);
            if history.len() > self.adaptive_window {
                history.remove(0);
            }
        }
    }
}

#[async_trait]
impl<V> Generator for AdversarialGenerator<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + Debug + 'static,
{
    type Value = V;

    async fn next(
        &mut self,
        process: ProcessId,
        _ctx: &GeneratorContext,
    ) -> Option<GeneratedOp<Self::Value>> {
        if self.is_exhausted() {
            return None;
        }

        // Exploration: occasionally use fallback
        if self.rng.r#gen::<f64>() < self.exploration_rate {
            if let Some(ref fallback) = self.fallback {
                let value = fallback(&mut self.rng);
                self.generated_count += 1;
                self.sequence += 1;

                return Some(
                    GeneratedOp::new(value)
                        .with_target(process)
                        .with_metadata(OperationMetadata::new().with_tag("exploration"))
                        .from_generator(format!("{}:exploration", self.name))
                        .with_sequence(self.sequence),
                );
            }
        }

        // Select a target and generate a challenging operation
        let target_idx = self.select_target()?;
        let target = &self.targets[target_idx];
        let value = target.generate(&mut self.rng)?;
        let target_name = target.name.clone();

        self.generated_count += 1;
        self.sequence += 1;

        Some(
            GeneratedOp::new(value)
                .with_target(process)
                .with_metadata(
                    OperationMetadata::new()
                        .with_tag("adversarial")
                        .with_custom("target_invariant", &target_name),
                )
                .from_generator(format!("{}:{}", self.name, target_name))
                .with_sequence(self.sequence),
        )
    }

    fn update(&mut self, _value: &Self::Value, _process: ProcessId, _success: bool, _result: Option<&str>) {
        // Adversarial generator updates via record_violation
    }

    fn is_exhausted(&self) -> bool {
        self.max_operations
            .map_or(false, |max| self.generated_count >= max)
    }

    fn reset(&mut self, seed: u64) {
        self.rng = ChaCha8Rng::seed_from_u64(seed);
        self.initial_seed = seed;
        self.round_robin_idx = 0;
        self.generated_count = 0;
        self.sequence = 0;
        for target in &mut self.targets {
            target.attempts = 0;
            target.violations = 0;
        }
        for history in self.recent_violations.values_mut() {
            history.clear();
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn remaining(&self) -> Option<usize> {
        self.max_operations
            .map(|max| (max - self.generated_count) as usize)
    }
}

impl<V> std::fmt::Debug for AdversarialGenerator<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdversarialGenerator")
            .field("name", &self.name)
            .field("seed", &self.initial_seed)
            .field("strategy", &self.strategy)
            .field("targets", &self.targets.len())
            .field("generated", &self.generated_count)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    enum TestOp {
        Normal(i32),
        EdgeCase(i32),
        Boundary(i32),
    }

    #[tokio::test]
    async fn test_adversarial_generator() {
        let mut generator = AdversarialGenerator::<TestOp>::new(42)
            .with_name("test")
            .with_target(
                InvariantTarget::new("bounds", "Check value bounds")
                    .with_challenger(|rng| TestOp::Boundary(rng.gen_range(-100..100)))
                    .with_weight(2.0),
            )
            .with_target(
                InvariantTarget::new("overflow", "Check overflow")
                    .with_challenger(|_| TestOp::EdgeCase(i32::MAX)),
            )
            .with_fallback(|rng| TestOp::Normal(rng.gen_range(0..10)));

        let ctx = GeneratorContext::new();
        let process = ProcessId::new(1);

        // Should generate operations
        let op = generator.next(process, &ctx).await;
        assert!(op.is_some());
    }

    #[test]
    fn test_invariant_target() {
        let mut target = InvariantTarget::<i32>::new("test", "Test invariant")
            .with_challenger(|rng| rng.gen_range(0..100))
            .with_weight(1.5);

        assert_eq!(target.violation_rate(), 0.0);

        target.record_attempt(false);
        target.record_attempt(true);
        target.record_attempt(false);

        assert!((target.violation_rate() - 0.333).abs() < 0.01);
        assert_eq!(target.attempts, 3);
        assert_eq!(target.violations, 1);
    }

    #[tokio::test]
    async fn test_round_robin_strategy() {
        let mut generator = AdversarialGenerator::<i32>::new(42)
            .with_strategy(TargetingStrategy::RoundRobin)
            .with_target(InvariantTarget::new("a", "A").with_challenger(|_| 1))
            .with_target(InvariantTarget::new("b", "B").with_challenger(|_| 2))
            .with_target(InvariantTarget::new("c", "C").with_challenger(|_| 3))
            .with_exploration_rate(0.0); // Disable exploration

        let ctx = GeneratorContext::new();
        let process = ProcessId::new(1);

        // Should cycle through targets
        let mut values = Vec::new();
        for _ in 0..6 {
            if let Some(op) = generator.next(process, &ctx).await {
                values.push(op.value);
            }
        }

        // With round-robin, we should see pattern 1,2,3,1,2,3
        assert_eq!(values, vec![1, 2, 3, 1, 2, 3]);
    }

    #[test]
    fn test_adaptive_strategy() {
        let mut generator = AdversarialGenerator::<i32>::new(42)
            .with_strategy(TargetingStrategy::Adaptive)
            .with_adaptive_window(5)
            .with_target(InvariantTarget::new("high", "High violations").with_challenger(|_| 1))
            .with_target(InvariantTarget::new("low", "Low violations").with_challenger(|_| 2));

        // Record some violations
        generator.record_violation("high", true);
        generator.record_violation("high", true);
        generator.record_violation("low", false);
        generator.record_violation("low", false);

        // Check stats
        let (attempts, violations, rate) = generator.target_stats("high").unwrap();
        assert_eq!(attempts, 2);
        assert_eq!(violations, 2);
        assert_eq!(rate, 1.0);
    }
}
