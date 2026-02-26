//! Random operation generator with configurable distributions

use async_trait::async_trait;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Debug;
use std::marker::PhantomData;

use mamut_core::ProcessId;

use crate::context::GeneratorContext;
use crate::generated::{GeneratedOp, OperationMetadata};
use crate::traits::Generator;

/// Configuration for value distributions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DistributionConfig {
    /// Uniform distribution between min and max
    Uniform { min: f64, max: f64 },
    /// Normal (Gaussian) distribution with mean and standard deviation
    Normal { mean: f64, std_dev: f64 },
    /// Exponential distribution with rate parameter (lambda)
    Exponential { rate: f64 },
    /// Constant value
    Constant { value: f64 },
    /// Custom weights for discrete choices
    Weighted { weights: Vec<f64> },
}

impl Default for DistributionConfig {
    fn default() -> Self {
        DistributionConfig::Uniform { min: 0.0, max: 1.0 }
    }
}

impl DistributionConfig {
    /// Sample a value from the distribution
    pub fn sample(&self, rng: &mut impl Rng) -> f64 {
        match self {
            DistributionConfig::Uniform { min, max } => rng.gen_range(*min..*max),
            DistributionConfig::Normal { mean, std_dev } => {
                // Box-Muller transform for normal distribution
                let u1: f64 = rng.r#gen();
                let u2: f64 = rng.r#gen();
                let z = (-2.0_f64 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                mean + std_dev * z
            }
            DistributionConfig::Exponential { rate } => {
                let u: f64 = rng.r#gen();
                -u.ln() / rate
            }
            DistributionConfig::Constant { value } => *value,
            DistributionConfig::Weighted { weights } => {
                if weights.is_empty() {
                    return 0.0;
                }
                let total: f64 = weights.iter().sum();
                if total <= 0.0 {
                    return 0.0;
                }
                let mut roll = rng.r#gen::<f64>() * total;
                for (i, w) in weights.iter().enumerate() {
                    roll -= w;
                    if roll <= 0.0 {
                        return i as f64;
                    }
                }
                (weights.len() - 1) as f64
            }
        }
    }

    /// Sample an integer index (useful for weighted selection)
    pub fn sample_index(&self, rng: &mut impl Rng, count: usize) -> usize {
        if count == 0 {
            return 0;
        }
        match self {
            DistributionConfig::Weighted { weights } => {
                let weights: Vec<f64> = weights.iter().take(count).copied().collect();
                if weights.is_empty() {
                    rng.gen_range(0..count)
                } else {
                    let total: f64 = weights.iter().sum();
                    if total <= 0.0 {
                        return rng.gen_range(0..count);
                    }
                    let mut roll = rng.r#gen::<f64>() * total;
                    for (i, w) in weights.iter().enumerate() {
                        roll -= w;
                        if roll <= 0.0 {
                            return i;
                        }
                    }
                    weights.len() - 1
                }
            }
            _ => {
                let idx = self.sample(rng).abs() as usize;
                idx % count
            }
        }
    }
}

/// A weighted operation for random selection
pub struct WeightedOperation<V> {
    /// The operation value factory
    pub factory: Box<dyn Fn(&mut ChaCha8Rng) -> V + Send + Sync>,
    /// Selection weight
    pub weight: f64,
    /// Operation name for debugging
    pub name: String,
    /// Metadata template
    pub metadata: OperationMetadata,
}

impl<V> std::fmt::Debug for WeightedOperation<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeightedOperation")
            .field("name", &self.name)
            .field("weight", &self.weight)
            .finish()
    }
}

impl<V> WeightedOperation<V> {
    /// Create a new weighted operation with a constant value
    pub fn constant(value: V, weight: f64, name: impl Into<String>) -> Self
    where
        V: Clone + Send + Sync + 'static,
    {
        Self {
            factory: Box::new(move |_| value.clone()),
            weight,
            name: name.into(),
            metadata: OperationMetadata::new(),
        }
    }

    /// Create a new weighted operation with a factory function
    pub fn with_factory<F>(factory: F, weight: f64, name: impl Into<String>) -> Self
    where
        F: Fn(&mut ChaCha8Rng) -> V + Send + Sync + 'static,
    {
        Self {
            factory: Box::new(factory),
            weight,
            name: name.into(),
            metadata: OperationMetadata::new(),
        }
    }

    /// Set the metadata for this operation
    pub fn with_metadata(mut self, metadata: OperationMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Generate a value using the factory
    pub fn generate(&self, rng: &mut ChaCha8Rng) -> V {
        (self.factory)(rng)
    }
}

/// Random operation generator with configurable distributions
///
/// This generator produces operations randomly according to configured
/// weights and distributions. It supports deterministic replay through
/// seeded random number generation.
pub struct RandomGenerator<V> {
    /// Random number generator
    rng: ChaCha8Rng,
    /// Initial seed for reset
    initial_seed: u64,
    /// Weighted operations to choose from
    operations: Vec<WeightedOperation<V>>,
    /// Maximum number of operations to generate
    max_operations: Option<u64>,
    /// Number of operations generated
    generated_count: u64,
    /// Sequence counter
    sequence: u64,
    /// Generator name
    name: String,
    /// Process selection distribution
    process_distribution: DistributionConfig,
    /// Phantom for type safety
    _phantom: PhantomData<V>,
}

impl<V> RandomGenerator<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + Debug,
{
    /// Create a new random generator with the given seed
    pub fn new(seed: u64) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            initial_seed: seed,
            operations: Vec::new(),
            max_operations: None,
            generated_count: 0,
            sequence: 0,
            name: "RandomGenerator".to_string(),
            process_distribution: DistributionConfig::Uniform { min: 0.0, max: 1.0 },
            _phantom: PhantomData,
        }
    }

    /// Set the generator name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the maximum number of operations
    pub fn with_max_operations(mut self, max: u64) -> Self {
        self.max_operations = Some(max);
        self
    }

    /// Add a weighted operation
    pub fn with_operation(mut self, op: WeightedOperation<V>) -> Self {
        self.operations.push(op);
        self
    }

    /// Add multiple weighted operations
    pub fn with_operations(mut self, ops: impl IntoIterator<Item = WeightedOperation<V>>) -> Self {
        self.operations.extend(ops);
        self
    }

    /// Set the process selection distribution
    pub fn with_process_distribution(mut self, dist: DistributionConfig) -> Self {
        self.process_distribution = dist;
        self
    }

    /// Select a random operation using weighted selection
    fn select_operation(&mut self) -> Option<usize> {
        if self.operations.is_empty() {
            return None;
        }

        let total_weight: f64 = self.operations.iter().map(|op| op.weight).sum();
        if total_weight <= 0.0 {
            return Some(0);
        }

        let mut roll = self.rng.r#gen::<f64>() * total_weight;
        for (i, op) in self.operations.iter().enumerate() {
            roll -= op.weight;
            if roll <= 0.0 {
                return Some(i);
            }
        }

        Some(self.operations.len() - 1)
    }

    /// Select a random process from the context
    fn select_process(&mut self, ctx: &GeneratorContext) -> Option<ProcessId> {
        let processes = ctx.available_processes();
        if processes.is_empty() {
            return None;
        }
        let idx = self.process_distribution.sample_index(&mut self.rng, processes.len());
        processes.get(idx).copied()
    }
}

#[async_trait]
impl<V> Generator for RandomGenerator<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + Debug + 'static,
{
    type Value = V;

    async fn next(
        &mut self,
        process: ProcessId,
        ctx: &GeneratorContext,
    ) -> Option<GeneratedOp<Self::Value>> {
        // Check if exhausted
        if self.is_exhausted() {
            return None;
        }

        // Select an operation
        let idx = self.select_operation()?;
        let op = &self.operations[idx];
        let value = op.generate(&mut self.rng);
        let metadata = op.metadata.clone();
        let source = format!("{}:{}", self.name, op.name);

        // Optionally redirect to a different process
        let target = if ctx.available_processes().len() > 1 && self.rng.gen_bool(0.1) {
            self.select_process(ctx)
        } else {
            Some(process)
        };

        self.generated_count += 1;
        self.sequence += 1;

        Some(
            GeneratedOp::new(value)
                .with_target(target.unwrap_or(process))
                .with_metadata(metadata)
                .from_generator(source)
                .with_sequence(self.sequence),
        )
    }

    fn update(&mut self, _value: &Self::Value, _process: ProcessId, _success: bool, _result: Option<&str>) {
        // Random generator doesn't track state
    }

    fn is_exhausted(&self) -> bool {
        self.max_operations
            .map_or(false, |max| self.generated_count >= max)
    }

    fn reset(&mut self, seed: u64) {
        self.rng = ChaCha8Rng::seed_from_u64(seed);
        self.initial_seed = seed;
        self.generated_count = 0;
        self.sequence = 0;
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn remaining(&self) -> Option<usize> {
        self.max_operations
            .map(|max| (max - self.generated_count) as usize)
    }
}

impl<V> std::fmt::Debug for RandomGenerator<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RandomGenerator")
            .field("name", &self.name)
            .field("seed", &self.initial_seed)
            .field("operations", &self.operations.len())
            .field("generated", &self.generated_count)
            .field("max", &self.max_operations)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    enum TestOp {
        Read(String),
        Write(String, i32),
        Delete(String),
    }

    #[tokio::test]
    async fn test_random_generator() {
        let generator = RandomGenerator::<TestOp>::new(42)
            .with_name("test")
            .with_max_operations(10)
            .with_operation(WeightedOperation::constant(
                TestOp::Read("key".to_string()),
                1.0,
                "read",
            ))
            .with_operation(WeightedOperation::constant(
                TestOp::Write("key".to_string(), 42),
                1.0,
                "write",
            ));

        assert!(!generator.is_exhausted());
        assert_eq!(generator.remaining(), Some(10));
    }

    #[tokio::test]
    async fn test_deterministic_generation() {
        let mut gen1 = RandomGenerator::<TestOp>::new(12345)
            .with_operation(WeightedOperation::with_factory(
                |rng| TestOp::Write("key".to_string(), rng.gen_range(0..100)),
                1.0,
                "write",
            ));

        let mut gen2 = RandomGenerator::<TestOp>::new(12345)
            .with_operation(WeightedOperation::with_factory(
                |rng| TestOp::Write("key".to_string(), rng.gen_range(0..100)),
                1.0,
                "write",
            ));

        let ctx = GeneratorContext::with_processes(vec![ProcessId::new(1)]);
        let process = ProcessId::new(1);

        // Same seed should produce same sequence
        let op1 = gen1.next(process, &ctx).await;
        let op2 = gen2.next(process, &ctx).await;

        assert_eq!(op1.map(|o| o.value), op2.map(|o| o.value));
    }

    #[test]
    fn test_distribution_config() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let uniform = DistributionConfig::Uniform { min: 0.0, max: 10.0 };
        let val = uniform.sample(&mut rng);
        assert!((0.0..10.0).contains(&val));

        let constant = DistributionConfig::Constant { value: 5.0 };
        assert_eq!(constant.sample(&mut rng), 5.0);

        let weighted = DistributionConfig::Weighted { weights: vec![1.0, 2.0, 1.0] };
        let idx = weighted.sample_index(&mut rng, 3);
        assert!(idx < 3);
    }
}
