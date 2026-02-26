//! Core Generator trait and associated types

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

use mamut_core::ProcessId;

use crate::context::GeneratorContext;
use crate::generated::GeneratedOp;

/// Result type for generator operations
pub type GeneratorResult<T> = Result<T, GeneratorError>;

/// Errors that can occur during generation
#[derive(Debug, Clone, thiserror::Error)]
pub enum GeneratorError {
    /// Generator is exhausted
    #[error("generator exhausted")]
    Exhausted,

    /// No valid operation could be generated
    #[error("no valid operation: {0}")]
    NoValidOperation(String),

    /// Generation timed out
    #[error("generation timed out")]
    Timeout,

    /// State error
    #[error("state error: {0}")]
    StateError(String),

    /// Configuration error
    #[error("configuration error: {0}")]
    ConfigError(String),
}

/// The core generator trait for producing operation sequences
///
/// Generators are responsible for producing a stream of operations to be
/// executed against the system under test. They can be stateful, tracking
/// the results of previous operations to inform future generation.
///
/// # Type Parameters
///
/// * `Value` - The type of operation values produced by this generator
///
/// # Determinism
///
/// Generators should be deterministic given the same seed. Use the `reset`
/// method to reinitialize the generator with a specific seed for replay.
#[async_trait]
pub trait Generator: Send + Sync {
    /// The type of values produced by this generator
    type Value: Serialize + DeserializeOwned + Send + Sync + Clone + Debug;

    /// Generate the next operation for the given process
    ///
    /// Returns `None` when the generator is exhausted or no valid operation
    /// can be generated for the given process.
    ///
    /// # Arguments
    ///
    /// * `process` - The process ID that will execute the operation
    /// * `ctx` - The current generator context with model state and history
    async fn next(
        &mut self,
        process: ProcessId,
        ctx: &GeneratorContext,
    ) -> Option<GeneratedOp<Self::Value>>;

    /// Update the generator with feedback from an executed operation
    ///
    /// This allows generators to track the effects of operations and adjust
    /// their generation strategy accordingly.
    ///
    /// # Arguments
    ///
    /// * `value` - The operation value that was executed
    /// * `process` - The process that executed the operation
    /// * `success` - Whether the operation succeeded
    /// * `result` - Optional result or error message
    fn update(&mut self, value: &Self::Value, process: ProcessId, success: bool, result: Option<&str>);

    /// Check if the generator has been exhausted
    ///
    /// Returns `true` if the generator can no longer produce operations.
    fn is_exhausted(&self) -> bool;

    /// Reset the generator with a new seed
    ///
    /// This reinitializes the generator's random state for deterministic replay.
    ///
    /// # Arguments
    ///
    /// * `seed` - The seed value for the random number generator
    fn reset(&mut self, seed: u64);

    /// Get the name of this generator for debugging/logging
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    /// Estimate the number of operations remaining
    ///
    /// Returns `None` if the count is unknown or infinite.
    fn remaining(&self) -> Option<usize> {
        None
    }
}

/// Extension trait for boxed generators
impl<V: Serialize + DeserializeOwned + Send + Sync + Clone + Debug + 'static>
    dyn Generator<Value = V>
{
    /// Create a boxed generator from a concrete implementation
    pub fn boxed<G: Generator<Value = V> + 'static>(generator: G) -> Box<dyn Generator<Value = V>> {
        Box::new(generator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_id_display() {
        let pid = ProcessId::new(42);
        assert_eq!(format!("{}", pid), "Process(42)");
    }

    #[test]
    fn test_generator_error() {
        let err = GeneratorError::Exhausted;
        assert_eq!(format!("{}", err), "generator exhausted");

        let err = GeneratorError::NoValidOperation("no rules match".to_string());
        assert!(format!("{}", err).contains("no valid operation"));
    }
}
