//! Stateful operation generator that tracks model state

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

/// A condition on the model state
pub struct StateCondition<S> {
    /// Name of this condition
    pub name: String,
    /// The predicate function
    predicate: Box<dyn Fn(&S) -> bool + Send + Sync>,
}

impl<S> std::fmt::Debug for StateCondition<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateCondition")
            .field("name", &self.name)
            .finish()
    }
}

impl<S> StateCondition<S> {
    /// Create a new state condition
    pub fn new<F>(name: impl Into<String>, predicate: F) -> Self
    where
        F: Fn(&S) -> bool + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            predicate: Box::new(predicate),
        }
    }

    /// Evaluate the condition
    pub fn evaluate(&self, state: &S) -> bool {
        (self.predicate)(state)
    }
}

/// A transition rule that maps state conditions to operations
pub struct TransitionRule<S, V> {
    /// Name of this rule
    pub name: String,
    /// Precondition that must hold
    pub precondition: Option<StateCondition<S>>,
    /// The operation factory
    factory: Box<dyn Fn(&S, &mut ChaCha8Rng) -> V + Send + Sync>,
    /// Weight for random selection among applicable rules
    pub weight: f64,
    /// Metadata template
    pub metadata: OperationMetadata,
    /// Postcondition description (for documentation)
    pub postcondition_desc: Option<String>,
}

impl<S, V> std::fmt::Debug for TransitionRule<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransitionRule")
            .field("name", &self.name)
            .field("weight", &self.weight)
            .field("has_precondition", &self.precondition.is_some())
            .finish()
    }
}

impl<S, V> TransitionRule<S, V> {
    /// Create a new transition rule
    pub fn new<F>(name: impl Into<String>, factory: F) -> Self
    where
        F: Fn(&S, &mut ChaCha8Rng) -> V + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            precondition: None,
            factory: Box::new(factory),
            weight: 1.0,
            metadata: OperationMetadata::new(),
            postcondition_desc: None,
        }
    }

    /// Add a precondition
    pub fn when(mut self, condition: StateCondition<S>) -> Self {
        self.precondition = Some(condition);
        self
    }

    /// Set the weight
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    /// Set the metadata
    pub fn with_metadata(mut self, metadata: OperationMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Add postcondition description
    pub fn ensures(mut self, desc: impl Into<String>) -> Self {
        self.postcondition_desc = Some(desc.into());
        self
    }

    /// Check if the rule is applicable
    pub fn is_applicable(&self, state: &S) -> bool {
        self.precondition
            .as_ref()
            .map_or(true, |c| c.evaluate(state))
    }

    /// Generate an operation value
    pub fn generate(&self, state: &S, rng: &mut ChaCha8Rng) -> V {
        (self.factory)(state, rng)
    }
}

/// State update function type
pub type StateUpdater<S, V> = Box<dyn Fn(&mut S, &V, ProcessId, bool) + Send + Sync>;

/// Stateful generator that tracks model state and selects operations accordingly
///
/// This generator maintains an internal model of the system state and uses
/// transition rules to select appropriate operations based on the current state.
pub struct StatefulGenerator<S, V> {
    /// Random number generator
    rng: ChaCha8Rng,
    /// Initial seed
    initial_seed: u64,
    /// Current model state
    state: S,
    /// Initial state for reset
    initial_state: S,
    /// Transition rules
    rules: Vec<TransitionRule<S, V>>,
    /// State update function
    updater: Option<StateUpdater<S, V>>,
    /// Maximum operations
    max_operations: Option<u64>,
    /// Generated count
    generated_count: u64,
    /// Sequence counter
    sequence: u64,
    /// Generator name
    name: String,
    /// Per-process state tracking
    process_states: HashMap<ProcessId, S>,
    /// Use per-process state
    per_process: bool,
    /// Phantom
    _phantom: PhantomData<V>,
}

impl<S, V> StatefulGenerator<S, V>
where
    S: Clone + Send + Sync + 'static,
    V: Serialize + DeserializeOwned + Send + Sync + Clone + Debug + 'static,
{
    /// Create a new stateful generator
    pub fn new(seed: u64, initial_state: S) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            initial_seed: seed,
            state: initial_state.clone(),
            initial_state,
            rules: Vec::new(),
            updater: None,
            max_operations: None,
            generated_count: 0,
            sequence: 0,
            name: "StatefulGenerator".to_string(),
            process_states: HashMap::new(),
            per_process: false,
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

    /// Add a transition rule
    pub fn with_rule(mut self, rule: TransitionRule<S, V>) -> Self {
        self.rules.push(rule);
        self
    }

    /// Add multiple transition rules
    pub fn with_rules(mut self, rules: impl IntoIterator<Item = TransitionRule<S, V>>) -> Self {
        self.rules.extend(rules);
        self
    }

    /// Set the state update function
    pub fn with_updater<F>(mut self, updater: F) -> Self
    where
        F: Fn(&mut S, &V, ProcessId, bool) + Send + Sync + 'static,
    {
        self.updater = Some(Box::new(updater));
        self
    }

    /// Enable per-process state tracking
    pub fn per_process(mut self) -> Self {
        self.per_process = true;
        self
    }

    /// Get the current state
    pub fn state(&self) -> &S {
        &self.state
    }

    /// Get state for a specific process
    pub fn process_state(&self, process: ProcessId) -> &S {
        self.process_states.get(&process).unwrap_or(&self.state)
    }

    /// Get indices of applicable rules for the current state
    fn applicable_rule_indices(&self, state: &S) -> Vec<usize> {
        self.rules
            .iter()
            .enumerate()
            .filter(|(_, r)| r.is_applicable(state))
            .map(|(i, _)| i)
            .collect()
    }

    /// Select a rule index based on weights
    fn select_rule_index(&mut self, rule_indices: &[usize]) -> Option<usize> {
        if rule_indices.is_empty() {
            return None;
        }

        let total_weight: f64 = rule_indices.iter().map(|&i| self.rules[i].weight).sum();
        if total_weight <= 0.0 {
            return rule_indices.first().copied();
        }

        let mut roll = self.rng.r#gen::<f64>() * total_weight;
        for &idx in rule_indices {
            roll -= self.rules[idx].weight;
            if roll <= 0.0 {
                return Some(idx);
            }
        }

        rule_indices.last().copied()
    }
}

#[async_trait]
impl<S, V> Generator for StatefulGenerator<S, V>
where
    S: Clone + Send + Sync + 'static,
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

        // Get the relevant state
        let state = if self.per_process {
            self.process_states
                .entry(process)
                .or_insert_with(|| self.initial_state.clone())
                .clone()
        } else {
            self.state.clone()
        };

        // Find applicable rules
        let applicable = self.applicable_rule_indices(&state);
        if applicable.is_empty() {
            tracing::debug!(
                generator = %self.name,
                process = %process,
                "No applicable rules for current state"
            );
            return None;
        }

        // Select a rule
        let rule_idx = self.select_rule_index(&applicable)?;
        let rule = &self.rules[rule_idx];
        let value = rule.generate(&state, &mut self.rng);
        let metadata = rule.metadata.clone();
        let source = format!("{}:{}", self.name, rule.name);

        self.generated_count += 1;
        self.sequence += 1;

        Some(
            GeneratedOp::new(value)
                .with_target(process)
                .with_metadata(metadata)
                .from_generator(source)
                .with_sequence(self.sequence),
        )
    }

    fn update(&mut self, value: &Self::Value, process: ProcessId, success: bool, _result: Option<&str>) {
        if let Some(ref updater) = self.updater {
            if self.per_process {
                let state = self
                    .process_states
                    .entry(process)
                    .or_insert_with(|| self.initial_state.clone());
                updater(state, value, process, success);
            } else {
                updater(&mut self.state, value, process, success);
            }
        }
    }

    fn is_exhausted(&self) -> bool {
        self.max_operations
            .map_or(false, |max| self.generated_count >= max)
    }

    fn reset(&mut self, seed: u64) {
        self.rng = ChaCha8Rng::seed_from_u64(seed);
        self.initial_seed = seed;
        self.state = self.initial_state.clone();
        self.process_states.clear();
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

impl<S, V> std::fmt::Debug for StatefulGenerator<S, V>
where
    S: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatefulGenerator")
            .field("name", &self.name)
            .field("seed", &self.initial_seed)
            .field("rules", &self.rules.len())
            .field("generated", &self.generated_count)
            .field("state", &self.state)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Clone, Default)]
    struct CounterState {
        count: i32,
        max: i32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    enum CounterOp {
        Increment,
        Decrement,
        Reset,
    }

    #[tokio::test]
    async fn test_stateful_generator() {
        let generator = StatefulGenerator::<CounterState, CounterOp>::new(
            42,
            CounterState { count: 0, max: 10 },
        )
        .with_rule(
            TransitionRule::new("increment", |_, _| CounterOp::Increment)
                .when(StateCondition::new("not_at_max", |s: &CounterState| {
                    s.count < s.max
                }))
                .with_weight(2.0),
        )
        .with_rule(
            TransitionRule::new("decrement", |_, _| CounterOp::Decrement)
                .when(StateCondition::new("not_at_zero", |s: &CounterState| {
                    s.count > 0
                })),
        )
        .with_rule(TransitionRule::new("reset", |_, _| CounterOp::Reset).with_weight(0.5))
        .with_updater(|state, op, _process, _success| match op {
            CounterOp::Increment => state.count += 1,
            CounterOp::Decrement => state.count -= 1,
            CounterOp::Reset => state.count = 0,
        });

        assert_eq!(generator.state().count, 0);
    }

    #[test]
    fn test_state_condition() {
        let cond = StateCondition::new("positive", |x: &i32| *x > 0);
        assert!(cond.evaluate(&5));
        assert!(!cond.evaluate(&0));
        assert!(!cond.evaluate(&-1));
    }

    #[tokio::test]
    async fn test_applicable_rules() {
        let mut generator = StatefulGenerator::<i32, String>::new(42, 5)
            .with_rule(
                TransitionRule::new("always", |_, _| "always".to_string())
            )
            .with_rule(
                TransitionRule::new("positive", |_, _| "positive".to_string())
                    .when(StateCondition::new("pos", |x: &i32| *x > 0))
            )
            .with_rule(
                TransitionRule::new("negative", |_, _| "negative".to_string())
                    .when(StateCondition::new("neg", |x: &i32| *x < 0))
            );

        let ctx = GeneratorContext::new();

        // With state = 5, "always" and "positive" should be applicable
        let op = generator.next(ProcessId::new(1), &ctx).await;
        assert!(op.is_some());
    }
}
