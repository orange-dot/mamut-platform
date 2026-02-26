//! Core traits for consistency checkers and models.

use async_trait::async_trait;
use mamut_core::{History, Operation};
use serde::{de::DeserializeOwned, Serialize};

use crate::{CheckResult, ExpectedResult};

/// A consistency checker that validates histories against some consistency model.
///
/// Checkers are designed to be composable and can check histories incrementally
/// for streaming use cases.
///
/// # Type Parameters
///
/// * `Value` - The type of values in the operations being checked. Must be
///   serializable for persistence and debugging purposes.
///
/// # Example
///
/// ```rust,ignore
/// use mamut_checker::{Checker, CheckResult};
/// use mamut_core::History;
///
/// struct MyChecker;
///
/// #[async_trait]
/// impl Checker for MyChecker {
///     type Value = serde_json::Value;
///
///     fn name(&self) -> &str {
///         "my-checker"
///     }
///
///     async fn check(&self, history: &History<Self::Value>) -> CheckResult {
///         // Check consistency...
///         CheckResult::pass()
///     }
/// }
/// ```
#[async_trait]
pub trait Checker: Send + Sync {
    /// The type of values in operations that this checker handles.
    type Value: Serialize + DeserializeOwned + Send + Sync + Clone + 'static;

    /// Returns the name of this checker for logging and debugging.
    fn name(&self) -> &str;

    /// Check the entire history for consistency violations.
    ///
    /// This is the primary checking method. It should return a `CheckResult`
    /// indicating whether the history satisfies the consistency model.
    ///
    /// # Arguments
    ///
    /// * `history` - The complete history of operations to check.
    ///
    /// # Returns
    ///
    /// A `CheckResult` indicating success or failure with counterexample.
    async fn check(&self, history: &History<Self::Value>) -> CheckResult;

    /// Incrementally check a new operation as it arrives.
    ///
    /// This optional method allows checkers to maintain state and check
    /// operations as they are observed, enabling early violation detection.
    ///
    /// # Arguments
    ///
    /// * `op` - The new operation to check.
    ///
    /// # Returns
    ///
    /// `Some(CheckResult)` if a definitive result can be determined from
    /// this operation alone, `None` if more operations are needed.
    ///
    /// # Default Implementation
    ///
    /// Returns `None`, meaning incremental checking is not supported.
    fn check_incremental(&mut self, _op: &Operation<Self::Value>) -> Option<CheckResult> {
        None
    }

    /// Reset any internal state for incremental checking.
    ///
    /// This should be called before starting a new incremental check session.
    fn reset(&mut self) {}

    /// Get a description of what this checker validates.
    fn description(&self) -> &str {
        "No description available"
    }
}

/// A sequential specification model for a data structure.
///
/// The model defines the expected behavior of the data structure when operations
/// are executed sequentially. It is used by consistency checkers to determine
/// what constitutes valid behavior.
///
/// # Type Parameters
///
/// * `Op` - The type of operations the model understands.
/// * `State` - The internal state type of the model.
///
/// # Example
///
/// ```rust,ignore
/// use mamut_checker::{Model, ExpectedResult};
///
/// #[derive(Clone)]
/// struct RegisterModel;
///
/// #[derive(Clone, PartialEq)]
/// struct RegisterState(Option<i32>);
///
/// enum RegisterOp {
///     Read,
///     Write(i32),
/// }
///
/// impl Model for RegisterModel {
///     type Op = RegisterOp;
///     type State = RegisterState;
///
///     fn init(&self) -> Self::State {
///         RegisterState(None)
///     }
///
///     fn step(&self, state: &Self::State, op: &Self::Op) -> (Self::State, ExpectedResult) {
///         match op {
///             RegisterOp::Read => (state.clone(), ExpectedResult::Ok),
///             RegisterOp::Write(v) => (RegisterState(Some(*v)), ExpectedResult::Ok),
///         }
///     }
///
///     fn equivalent(&self, s1: &Self::State, s2: &Self::State) -> bool {
///         s1 == s2
///     }
/// }
/// ```
pub trait Model: Send + Sync + Clone {
    /// The type of operations this model can execute.
    type Op: Send + Sync + Clone;

    /// The state type maintained by this model.
    type State: Clone + Send + Sync;

    /// Create the initial state of the model.
    ///
    /// This represents the state of the data structure before any operations
    /// have been performed.
    fn init(&self) -> Self::State;

    /// Execute a single operation on the model state.
    ///
    /// This is the core method that defines the sequential semantics of the
    /// data structure. Given a state and an operation, it returns the new
    /// state and the expected result of the operation.
    ///
    /// # Arguments
    ///
    /// * `state` - The current state of the model.
    /// * `op` - The operation to execute.
    ///
    /// # Returns
    ///
    /// A tuple of (new_state, expected_result).
    fn step(&self, state: &Self::State, op: &Self::Op) -> (Self::State, ExpectedResult);

    /// Check if two states are equivalent.
    ///
    /// This is used to detect cycles during search (state space exploration)
    /// and to determine if different operation orderings lead to the same
    /// logical state.
    ///
    /// # Arguments
    ///
    /// * `s1` - The first state.
    /// * `s2` - The second state.
    ///
    /// # Returns
    ///
    /// `true` if the states are equivalent, `false` otherwise.
    fn equivalent(&self, s1: &Self::State, s2: &Self::State) -> bool;

    /// Check if an operation can be executed in the given state.
    ///
    /// This is used to prune the search space by identifying operations
    /// that cannot possibly succeed from a given state.
    ///
    /// # Default Implementation
    ///
    /// Returns `true` for all operations.
    fn can_execute(&self, _state: &Self::State, _op: &Self::Op) -> bool {
        true
    }

    /// Get the name of this model for debugging purposes.
    fn name(&self) -> &str {
        "unnamed-model"
    }
}

/// A simplified model trait for common data structures.
///
/// This trait provides a simpler interface for implementing models where
/// state comparison is just equality and all operations are always executable.
pub trait SimpleModel: Send + Sync + Clone {
    /// The type of operations this model can execute.
    type Op: Send + Sync + Clone;

    /// The state type maintained by this model.
    type State: Clone + Send + Sync + PartialEq;

    /// Create the initial state.
    fn init(&self) -> Self::State;

    /// Execute an operation.
    fn step(&self, state: &Self::State, op: &Self::Op) -> (Self::State, ExpectedResult);
}

// Blanket implementation of Model for SimpleModel
impl<T: SimpleModel> Model for T {
    type Op = <T as SimpleModel>::Op;
    type State = <T as SimpleModel>::State;

    fn init(&self) -> Self::State {
        <T as SimpleModel>::init(self)
    }

    fn step(&self, state: &Self::State, op: &Self::Op) -> (Self::State, ExpectedResult) {
        <T as SimpleModel>::step(self, state, op)
    }

    fn equivalent(&self, s1: &Self::State, s2: &Self::State) -> bool {
        s1 == s2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Simple register model for testing
    #[derive(Clone)]
    struct TestRegisterModel;

    #[derive(Clone, PartialEq)]
    struct TestState(i32);

    #[derive(Clone)]
    enum TestOp {
        Read(i32),
        Write(i32),
    }

    impl SimpleModel for TestRegisterModel {
        type Op = TestOp;
        type State = TestState;

        fn init(&self) -> Self::State {
            TestState(0)
        }

        fn step(&self, state: &Self::State, op: &Self::Op) -> (Self::State, ExpectedResult) {
            match op {
                TestOp::Read(v) => {
                    if *v == state.0 {
                        (state.clone(), ExpectedResult::Ok)
                    } else {
                        (state.clone(), ExpectedResult::Err("value mismatch".into()))
                    }
                }
                TestOp::Write(v) => (TestState(*v), ExpectedResult::Ok),
            }
        }
    }

    #[test]
    fn test_simple_model() {
        let model = TestRegisterModel;
        let state = model.init();
        assert_eq!(state.0, 0);

        let (new_state, result) = model.step(&state, &TestOp::Write(42));
        assert_eq!(new_state.0, 42);
        assert_eq!(result, ExpectedResult::Ok);

        let (_, result) = model.step(&new_state, &TestOp::Read(42));
        assert_eq!(result, ExpectedResult::Ok);

        let (_, result) = model.step(&new_state, &TestOp::Read(0));
        assert!(matches!(result, ExpectedResult::Err(_)));
    }

    #[test]
    fn test_state_equivalence() {
        let model = TestRegisterModel;
        let s1 = TestState(42);
        let s2 = TestState(42);
        let s3 = TestState(0);

        assert!(model.equivalent(&s1, &s2));
        assert!(!model.equivalent(&s1, &s3));
    }
}
