//! Generator combinators for composing complex generation strategies

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

use mamut_core::ProcessId;

use crate::context::GeneratorContext;
use crate::generated::GeneratedOp;
use crate::traits::Generator;

/// Chain combinator: runs generators sequentially
///
/// When the first generator is exhausted, switches to the next one.
pub struct Chain<V> {
    generators: Vec<Box<dyn Generator<Value = V>>>,
    current: usize,
    name: String,
}

impl<V> Chain<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + Debug + 'static,
{
    /// Create a new chain combinator
    pub fn new(generators: Vec<Box<dyn Generator<Value = V>>>) -> Self {
        Self {
            generators,
            current: 0,
            name: "Chain".to_string(),
        }
    }

    /// Create from two generators
    pub fn pair(first: impl Generator<Value = V> + 'static, second: impl Generator<Value = V> + 'static) -> Self {
        Self::new(vec![Box::new(first), Box::new(second)])
    }

    /// Set the name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add another generator to the chain
    pub fn then(mut self, generator: impl Generator<Value = V> + 'static) -> Self {
        self.generators.push(Box::new(generator));
        self
    }

    /// Get the current generator index
    pub fn current_index(&self) -> usize {
        self.current
    }

    /// Get the number of generators in the chain
    pub fn len(&self) -> usize {
        self.generators.len()
    }

    /// Check if the chain is empty
    pub fn is_empty(&self) -> bool {
        self.generators.is_empty()
    }
}

#[async_trait]
impl<V> Generator for Chain<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + Debug + 'static,
{
    type Value = V;

    async fn next(&mut self, process: ProcessId, ctx: &GeneratorContext) -> Option<GeneratedOp<Self::Value>> {
        while self.current < self.generators.len() {
            let inner_gen = &mut self.generators[self.current];
            if !inner_gen.is_exhausted() {
                if let Some(op) = inner_gen.next(process, ctx).await {
                    return Some(op.from_generator(format!("{}[{}]", self.name, self.current)));
                }
            }
            self.current += 1;
        }
        None
    }

    fn update(&mut self, value: &Self::Value, process: ProcessId, success: bool, result: Option<&str>) {
        if self.current < self.generators.len() {
            self.generators[self.current].update(value, process, success, result);
        }
    }

    fn is_exhausted(&self) -> bool {
        self.current >= self.generators.len()
            || (self.current == self.generators.len() - 1
                && self.generators[self.current].is_exhausted())
    }

    fn reset(&mut self, seed: u64) {
        self.current = 0;
        for (i, inner_gen) in self.generators.iter_mut().enumerate() {
            inner_gen.reset(seed.wrapping_add(i as u64));
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn remaining(&self) -> Option<usize> {
        let mut total = 0;
        for inner_gen in &self.generators[self.current..] {
            if let Some(r) = inner_gen.remaining() {
                total += r;
            } else {
                return None;
            }
        }
        Some(total)
    }
}

/// Interleave combinator: alternates between generators
///
/// Cycles through generators, taking one operation from each in turn.
pub struct Interleave<V> {
    generators: Vec<Box<dyn Generator<Value = V>>>,
    current: usize,
    exhausted_count: usize,
    name: String,
}

impl<V> Interleave<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + Debug + 'static,
{
    /// Create a new interleave combinator
    pub fn new(generators: Vec<Box<dyn Generator<Value = V>>>) -> Self {
        Self {
            generators,
            current: 0,
            exhausted_count: 0,
            name: "Interleave".to_string(),
        }
    }

    /// Create from two generators
    pub fn pair(first: impl Generator<Value = V> + 'static, second: impl Generator<Value = V> + 'static) -> Self {
        Self::new(vec![Box::new(first), Box::new(second)])
    }

    /// Set the name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add another generator to interleave
    pub fn and(mut self, generator: impl Generator<Value = V> + 'static) -> Self {
        self.generators.push(Box::new(generator));
        self
    }
}

#[async_trait]
impl<V> Generator for Interleave<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + Debug + 'static,
{
    type Value = V;

    async fn next(&mut self, process: ProcessId, ctx: &GeneratorContext) -> Option<GeneratedOp<Self::Value>> {
        let gen_count = self.generators.len();
        if gen_count == 0 {
            return None;
        }

        // Recount exhausted generators in case state changed
        self.exhausted_count = self.generators.iter().filter(|g| g.is_exhausted()).count();
        if self.exhausted_count >= gen_count {
            return None;
        }

        let start = self.current;
        loop {
            let idx = self.current;
            self.current = (self.current + 1) % gen_count;

            let inner_gen = &mut self.generators[idx];
            if !inner_gen.is_exhausted() {
                if let Some(op) = inner_gen.next(process, ctx).await {
                    return Some(op.from_generator(format!("{}[{}]", self.name, idx)));
                }
            }

            if self.current == start {
                // Full cycle completed, check exhaustion again
                self.exhausted_count = self.generators.iter().filter(|g| g.is_exhausted()).count();
                break;
            }
        }
        None
    }

    fn update(&mut self, value: &Self::Value, process: ProcessId, success: bool, result: Option<&str>) {
        // Update all generators (they may share state)
        for inner_gen in &mut self.generators {
            inner_gen.update(value, process, success, result);
        }
    }

    fn is_exhausted(&self) -> bool {
        self.generators.is_empty() || self.exhausted_count >= self.generators.len()
    }

    fn reset(&mut self, seed: u64) {
        self.current = 0;
        self.exhausted_count = 0;
        for (i, inner_gen) in self.generators.iter_mut().enumerate() {
            inner_gen.reset(seed.wrapping_add(i as u64));
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Filter combinator: filters operations based on a predicate
pub struct Filter<G, F, V> {
    inner: G,
    predicate: F,
    name: String,
    max_retries: usize,
    _phantom: PhantomData<V>,
}

impl<G, F, V> Filter<G, F, V>
where
    G: Generator<Value = V>,
    F: Fn(&GeneratedOp<V>) -> bool + Send + Sync,
    V: Serialize + DeserializeOwned + Send + Sync + Clone + Debug,
{
    /// Create a new filter combinator
    pub fn new(inner: G, predicate: F) -> Self {
        Self {
            inner,
            predicate,
            name: "Filter".to_string(),
            max_retries: 100,
            _phantom: PhantomData,
        }
    }

    /// Set the name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the maximum retry count
    pub fn with_max_retries(mut self, max: usize) -> Self {
        self.max_retries = max;
        self
    }

    /// Get a reference to the inner generator
    pub fn inner(&self) -> &G {
        &self.inner
    }

    /// Get a mutable reference to the inner generator
    pub fn inner_mut(&mut self) -> &mut G {
        &mut self.inner
    }
}

#[async_trait]
impl<G, F, V> Generator for Filter<G, F, V>
where
    G: Generator<Value = V>,
    F: Fn(&GeneratedOp<V>) -> bool + Send + Sync,
    V: Serialize + DeserializeOwned + Send + Sync + Clone + Debug + 'static,
{
    type Value = V;

    async fn next(&mut self, process: ProcessId, ctx: &GeneratorContext) -> Option<GeneratedOp<Self::Value>> {
        for _ in 0..self.max_retries {
            if self.inner.is_exhausted() {
                return None;
            }
            if let Some(op) = self.inner.next(process, ctx).await {
                if (self.predicate)(&op) {
                    return Some(op);
                }
            }
        }
        None
    }

    fn update(&mut self, value: &Self::Value, process: ProcessId, success: bool, result: Option<&str>) {
        self.inner.update(value, process, success, result);
    }

    fn is_exhausted(&self) -> bool {
        self.inner.is_exhausted()
    }

    fn reset(&mut self, seed: u64) {
        self.inner.reset(seed);
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Take combinator: limits the number of operations
pub struct Take<G> {
    inner: G,
    limit: u64,
    count: u64,
    name: String,
}

impl<G> Take<G>
where
    G: Generator,
{
    /// Create a new take combinator
    pub fn new(inner: G, limit: u64) -> Self {
        Self {
            inner,
            limit,
            count: 0,
            name: "Take".to_string(),
        }
    }

    /// Set the name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Get the number of operations taken so far
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Get the limit
    pub fn limit(&self) -> u64 {
        self.limit
    }

    /// Get a reference to the inner generator
    pub fn inner(&self) -> &G {
        &self.inner
    }

    /// Get a mutable reference to the inner generator
    pub fn inner_mut(&mut self) -> &mut G {
        &mut self.inner
    }
}

#[async_trait]
impl<G> Generator for Take<G>
where
    G: Generator,
    G::Value: 'static,
{
    type Value = G::Value;

    async fn next(&mut self, process: ProcessId, ctx: &GeneratorContext) -> Option<GeneratedOp<Self::Value>> {
        if self.count >= self.limit {
            return None;
        }
        if let Some(op) = self.inner.next(process, ctx).await {
            self.count += 1;
            Some(op)
        } else {
            None
        }
    }

    fn update(&mut self, value: &Self::Value, process: ProcessId, success: bool, result: Option<&str>) {
        self.inner.update(value, process, success, result);
    }

    fn is_exhausted(&self) -> bool {
        self.count >= self.limit || self.inner.is_exhausted()
    }

    fn reset(&mut self, seed: u64) {
        self.count = 0;
        self.inner.reset(seed);
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn remaining(&self) -> Option<usize> {
        let take_remaining = (self.limit - self.count) as usize;
        match self.inner.remaining() {
            Some(inner_remaining) => Some(take_remaining.min(inner_remaining)),
            None => Some(take_remaining),
        }
    }
}

/// WithTimeout combinator: adds timeout constraints to generation
pub struct WithTimeout<G> {
    inner: G,
    timeout: Duration,
    deadline: Option<Instant>,
    name: String,
}

impl<G> WithTimeout<G>
where
    G: Generator,
{
    /// Create a new timeout combinator
    pub fn new(inner: G, timeout: Duration) -> Self {
        Self {
            inner,
            timeout,
            deadline: None,
            name: "WithTimeout".to_string(),
        }
    }

    /// Set the name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Start the timer
    pub fn start(&mut self) {
        self.deadline = Some(Instant::now() + self.timeout);
    }

    /// Get remaining time
    pub fn remaining_time(&self) -> Option<Duration> {
        self.deadline.map(|d| d.saturating_duration_since(Instant::now()))
    }

    /// Check if timed out
    pub fn is_timed_out(&self) -> bool {
        self.deadline.map_or(false, |d| Instant::now() >= d)
    }

    /// Get a reference to the inner generator
    pub fn inner(&self) -> &G {
        &self.inner
    }

    /// Get a mutable reference to the inner generator
    pub fn inner_mut(&mut self) -> &mut G {
        &mut self.inner
    }
}

#[async_trait]
impl<G> Generator for WithTimeout<G>
where
    G: Generator,
    G::Value: 'static,
{
    type Value = G::Value;

    async fn next(&mut self, process: ProcessId, ctx: &GeneratorContext) -> Option<GeneratedOp<Self::Value>> {
        // Auto-start on first call
        if self.deadline.is_none() {
            self.start();
        }

        if self.is_timed_out() {
            return None;
        }

        // Check remaining time
        let remaining = self.remaining_time()?;
        if remaining.is_zero() {
            return None;
        }

        // Use tokio timeout for the actual generation
        match tokio::time::timeout(remaining, self.inner.next(process, ctx)).await {
            Ok(result) => result,
            Err(_) => None, // Timeout
        }
    }

    fn update(&mut self, value: &Self::Value, process: ProcessId, success: bool, result: Option<&str>) {
        self.inner.update(value, process, success, result);
    }

    fn is_exhausted(&self) -> bool {
        self.is_timed_out() || self.inner.is_exhausted()
    }

    fn reset(&mut self, seed: u64) {
        self.deadline = None;
        self.inner.reset(seed);
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl<V: Debug> std::fmt::Debug for Chain<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chain")
            .field("name", &self.name)
            .field("current", &self.current)
            .field("total", &self.generators.len())
            .finish()
    }
}

impl<V: Debug> std::fmt::Debug for Interleave<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Interleave")
            .field("name", &self.name)
            .field("current", &self.current)
            .field("total", &self.generators.len())
            .field("exhausted", &self.exhausted_count)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::{RandomGenerator, WeightedOperation};

    #[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq)]
    struct TestOp(i32);

    fn make_generator(id: i32, count: u64) -> RandomGenerator<TestOp> {
        RandomGenerator::new(id as u64)
            .with_max_operations(count)
            .with_operation(WeightedOperation::constant(TestOp(id), 1.0, "test"))
    }

    #[tokio::test]
    async fn test_chain() {
        let mut chain = Chain::pair(make_generator(1, 2), make_generator(2, 2));

        let ctx = GeneratorContext::new();
        let process = ProcessId::new(1);

        let mut values = vec![];
        while let Some(op) = chain.next(process, &ctx).await {
            values.push(op.value.0);
        }

        assert_eq!(values, vec![1, 1, 2, 2]);
        assert!(chain.is_exhausted());
    }

    #[tokio::test]
    async fn test_interleave() {
        let mut interleave = Interleave::pair(make_generator(1, 2), make_generator(2, 2));

        let ctx = GeneratorContext::new();
        let process = ProcessId::new(1);

        let mut values = vec![];
        while let Some(op) = interleave.next(process, &ctx).await {
            values.push(op.value.0);
        }

        assert_eq!(values, vec![1, 2, 1, 2]);
        assert!(interleave.is_exhausted());
    }

    #[tokio::test]
    async fn test_filter() {
        let inner_gen = make_generator(1, 10);
        let mut filtered = Filter::new(inner_gen, |op| op.value.0 % 2 == 1);

        let ctx = GeneratorContext::new();
        let process = ProcessId::new(1);

        // Should still produce operations (all are 1, which is odd)
        let op = filtered.next(process, &ctx).await;
        assert!(op.is_some());
    }

    #[tokio::test]
    async fn test_take() {
        let inner_gen = make_generator(1, 100);
        let mut taken = Take::new(inner_gen, 5);

        let ctx = GeneratorContext::new();
        let process = ProcessId::new(1);

        let mut count = 0;
        while taken.next(process, &ctx).await.is_some() {
            count += 1;
        }

        assert_eq!(count, 5);
        assert!(taken.is_exhausted());
        assert_eq!(taken.remaining(), Some(0));
    }

    #[tokio::test]
    async fn test_with_timeout() {
        let inner_gen = make_generator(1, 1000);
        let mut timed = WithTimeout::new(inner_gen, Duration::from_millis(100));

        let ctx = GeneratorContext::new();
        let process = ProcessId::new(1);

        // Should produce at least one operation quickly
        let op = timed.next(process, &ctx).await;
        assert!(op.is_some());

        // Timer should have started
        assert!(timed.remaining_time().is_some());
    }

    #[tokio::test]
    async fn test_chain_reset() {
        let mut chain = Chain::pair(make_generator(1, 2), make_generator(2, 2));

        let ctx = GeneratorContext::new();
        let process = ProcessId::new(1);

        // Exhaust the chain
        while chain.next(process, &ctx).await.is_some() {}
        assert!(chain.is_exhausted());

        // Reset
        chain.reset(99);
        assert!(!chain.is_exhausted());
        assert_eq!(chain.current_index(), 0);

        // Should produce again
        let op = chain.next(process, &ctx).await;
        assert!(op.is_some());
    }
}
