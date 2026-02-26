//! Mamut Generator - Operation generators for property-based testing
//!
//! This crate provides the `Generator` trait and various implementations for
//! generating sequences of operations during verification runs.
//!
//! # Core Concepts
//!
//! - [`Generator`]: The core trait for operation generators
//! - [`GeneratorContext`]: Runtime context providing model state and history
//! - [`GeneratedOp`]: A generated operation with metadata
//!
//! # Generator Implementations
//!
//! - [`RandomGenerator`]: Configurable random operation generation
//! - [`StatefulGenerator`]: State-aware generation based on model state
//! - [`ReplayGenerator`]: Deterministic replay of recorded histories
//! - [`AdversarialGenerator`]: Targeted generation to find invariant violations
//!
//! # Combinators
//!
//! Generators can be combined using combinators:
//! - [`Chain`]: Run generators sequentially
//! - [`Interleave`]: Alternate between generators
//! - [`Filter`]: Filter generated operations
//! - [`Take`]: Limit number of operations
//! - [`WithTimeout`]: Add timeout constraints

mod adversarial;
mod combinators;
mod context;
mod generated;
mod random;
mod replay;
mod stateful;
mod traits;

// Re-export core types for convenience
pub use mamut_core::ProcessId;

pub use adversarial::{AdversarialGenerator, InvariantTarget, TargetingStrategy};
pub use combinators::{Chain, Filter, Interleave, Take, WithTimeout};
pub use context::{ActiveFault, FaultType, GeneratorContext, HistorySummary};
pub use generated::{GeneratedOp, OperationMetadata};
pub use random::{DistributionConfig, RandomGenerator, WeightedOperation};
pub use replay::{RecordedOp, ReplayGenerator, ReplayMode};
pub use stateful::{StateCondition, StatefulGenerator, TransitionRule};
pub use traits::{Generator, GeneratorResult};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::adversarial::{AdversarialGenerator, InvariantTarget, TargetingStrategy};
    pub use crate::combinators::{Chain, Filter, Interleave, Take, WithTimeout};
    pub use crate::context::{ActiveFault, FaultType, GeneratorContext, HistorySummary};
    pub use crate::generated::{GeneratedOp, OperationMetadata};
    pub use crate::random::{DistributionConfig, RandomGenerator, WeightedOperation};
    pub use crate::replay::{RecordedOp, ReplayGenerator, ReplayMode};
    pub use crate::stateful::{StateCondition, StatefulGenerator, TransitionRule};
    pub use crate::traits::{Generator, GeneratorResult};
    pub use mamut_core::ProcessId;
}
