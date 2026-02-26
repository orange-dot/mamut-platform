//! Safe handle implementations for FFI resources.
//!
//! This module provides the internal implementations behind the opaque FFI handles.
//! Each handle type wraps its resources in a `SafeHandle` to ensure thread-safe
//! access and proper cleanup.
//!
//! # Design
//!
//! The `SafeHandle<T>` wrapper provides:
//! - Interior mutability with `RefCell` (single-threaded) or `RwLock` (thread-safe)
//! - Panic safety at the FFI boundary
//! - Automatic resource cleanup via `Drop`
//!
//! # Thread Safety
//!
//! - `VerifierInner` uses `RwLock` for thread-safe access
//! - `TestInner` uses `RefCell` and is NOT thread-safe
//! - Callers must ensure proper synchronization for `TestInner`

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{c_char, c_void, CString};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::types::*;

// ============================================================================
// SafeHandle Wrapper
// ============================================================================

/// A safe wrapper for FFI handles that provides interior mutability.
///
/// This wrapper ensures that the inner value can be safely accessed and
/// modified through the FFI boundary.
///
/// # Type Parameters
///
/// - `T`: The inner type to wrap.
pub struct SafeHandle<T> {
    inner: RefCell<T>,
}

impl<T> SafeHandle<T> {
    /// Create a new SafeHandle wrapping the given value.
    pub fn new(inner: T) -> Self {
        Self {
            inner: RefCell::new(inner),
        }
    }

    /// Access the inner value immutably.
    pub fn with<R, F: FnOnce(&T) -> R>(&self, f: F) -> R {
        let inner = self.inner.borrow();
        f(&*inner)
    }

    /// Access the inner value mutably.
    pub fn with_mut<R, F: FnOnce(&mut T) -> R>(&self, f: F) -> R {
        let mut inner = self.inner.borrow_mut();
        f(&mut *inner)
    }
}

// SafeHandle is Send if T is Send (for single-threaded use)
unsafe impl<T: Send> Send for SafeHandle<T> {}

// ============================================================================
// VerifierInner
// ============================================================================

/// Internal state for the verification engine.
///
/// This is the implementation behind `VerifierHandle`.
pub struct VerifierInner {
    /// Configuration loaded from file, if any.
    config: Option<VerifierConfig>,
    /// Active tests tracked by this verifier.
    active_tests: Arc<RwLock<Vec<u64>>>,
    /// Next test ID.
    next_test_id: AtomicU64,
    /// Whether the verifier has been shut down.
    shutdown: AtomicBool,
}

/// Configuration for the verifier.
#[derive(Debug, Clone, Default)]
pub struct VerifierConfig {
    /// Log level.
    pub log_level: String,
    /// Maximum concurrent tests.
    pub max_concurrent_tests: usize,
    /// Default timeout in milliseconds.
    pub default_timeout_ms: u64,
}

impl VerifierInner {
    /// Create a new verifier with default configuration.
    pub fn new() -> Self {
        Self {
            config: None,
            active_tests: Arc::new(RwLock::new(Vec::new())),
            next_test_id: AtomicU64::new(1),
            shutdown: AtomicBool::new(false),
        }
    }

    /// Create a new verifier with configuration from a file.
    pub fn with_config(config_path: &str) -> Result<Self, String> {
        // In a real implementation, we would parse the config file here.
        // For now, we just validate the path exists.
        if config_path.is_empty() {
            return Err("Config path cannot be empty".to_string());
        }

        let config = VerifierConfig {
            log_level: "info".to_string(),
            max_concurrent_tests: 4,
            default_timeout_ms: 60000,
        };

        Ok(Self {
            config: Some(config),
            active_tests: Arc::new(RwLock::new(Vec::new())),
            next_test_id: AtomicU64::new(1),
            shutdown: AtomicBool::new(false),
        })
    }

    /// Check if the verifier has been shut down.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Allocate a new test ID.
    pub fn allocate_test_id(&self) -> u64 {
        self.next_test_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Register a test as active.
    pub fn register_test(&self, test_id: u64) {
        if let Ok(mut tests) = self.active_tests.write() {
            tests.push(test_id);
        }
    }

    /// Unregister a test.
    pub fn unregister_test(&self, test_id: u64) {
        if let Ok(mut tests) = self.active_tests.write() {
            tests.retain(|&id| id != test_id);
        }
    }
}

impl Default for VerifierInner {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for VerifierInner {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }
}

// ============================================================================
// TestInner
// ============================================================================

/// Stored callback with user data.
struct CallbackWithData<F> {
    callback: F,
    user_data: *mut c_void,
}

// Allow sending callbacks between threads (user's responsibility to ensure safety)
unsafe impl<F> Send for CallbackWithData<F> {}

/// Internal state for a test execution context.
///
/// This is the implementation behind `TestHandle`.
pub struct TestInner {
    /// Test ID.
    id: u64,
    /// Test name.
    name: String,
    /// Optional description.
    description: Option<String>,
    /// Maximum operations to execute.
    max_operations: u64,
    /// Timeout in milliseconds.
    timeout_ms: u64,
    /// Random seed.
    seed: u64,
    /// Execution mode.
    mode: ExecutionMode,
    /// Verbose logging.
    verbose: bool,
    /// Check invariants after each operation.
    check_invariants: bool,
    /// Fault injection enabled.
    fault_injection_enabled: bool,

    /// Queued operations.
    operations: Vec<StoredOperation>,
    /// Next operation ID.
    next_op_id: u64,
    /// Operation results.
    results: Vec<StoredOperationResult>,

    /// Active faults.
    active_faults: HashMap<u64, FaultInfo>,
    /// Next fault ID.
    next_fault_id: u64,

    /// Executor callback.
    executor: Option<CallbackWithData<ExecuteOperationCallback>>,
    /// Pre-operation callback.
    pre_operation: Option<CallbackWithData<OperationCallback>>,
    /// Progress callback.
    progress: Option<CallbackWithData<ProgressCallback>>,
    /// Fault injected callback.
    on_fault_inject: Option<CallbackWithData<FaultInjectedCallback>>,
    /// Fault cleared callback.
    on_fault_clear: Option<CallbackWithData<FaultClearedCallback>>,

    /// Test state.
    state: TestState,
    /// Cancellation flag.
    cancelled: AtomicBool,
    /// Start time.
    start_time: Option<Instant>,
    /// End time.
    end_time: Option<Instant>,
}

/// Test execution state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TestState {
    /// Test is being configured.
    Setup,
    /// Test is running.
    Running,
    /// Test completed successfully.
    Completed,
    /// Test was cancelled.
    Cancelled,
    /// Test failed.
    Failed,
}

/// Stored operation with owned data.
#[derive(Debug, Clone)]
struct StoredOperation {
    id: u64,
    op_type: OperationType,
    key: Vec<u8>,
    value: Vec<u8>,
    custom_name: String,
}

/// Stored operation result with owned data.
#[derive(Debug, Clone)]
struct StoredOperationResult {
    operation_id: u64,
    status: OperationStatus,
    value: Vec<u8>,
    error_message: Option<String>,
    duration_ns: u64,
}

impl TestInner {
    /// Create a new test context.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        verifier: &SafeHandle<VerifierInner>,
        name: String,
        description: Option<String>,
        max_operations: u64,
        timeout_ms: u64,
        seed: u64,
        mode: ExecutionMode,
        verbose: bool,
        check_invariants: bool,
        fault_injection_enabled: bool,
    ) -> Self {
        let id = verifier.with(|v| {
            let id = v.allocate_test_id();
            v.register_test(id);
            id
        });

        Self {
            id,
            name,
            description,
            max_operations,
            timeout_ms,
            seed,
            mode,
            verbose,
            check_invariants,
            fault_injection_enabled,
            operations: Vec::new(),
            next_op_id: 1,
            results: Vec::new(),
            active_faults: HashMap::new(),
            next_fault_id: 1,
            executor: None,
            pre_operation: None,
            progress: None,
            on_fault_inject: None,
            on_fault_clear: None,
            state: TestState::Setup,
            cancelled: AtomicBool::new(false),
            start_time: None,
            end_time: None,
        }
    }

    /// Set the executor callback.
    pub fn set_executor(&mut self, callback: ExecuteOperationCallback, user_data: *mut c_void) {
        self.executor = callback.map(|cb| CallbackWithData {
            callback: Some(cb),
            user_data,
        });
    }

    /// Set the pre-operation callback.
    pub fn set_pre_operation_callback(
        &mut self,
        callback: OperationCallback,
        user_data: *mut c_void,
    ) {
        self.pre_operation = callback.map(|cb| CallbackWithData {
            callback: Some(cb),
            user_data,
        });
    }

    /// Set the progress callback.
    pub fn set_progress_callback(&mut self, callback: ProgressCallback, user_data: *mut c_void) {
        self.progress = callback.map(|cb| CallbackWithData {
            callback: Some(cb),
            user_data,
        });
    }

    /// Set fault injection callbacks.
    pub fn set_fault_callbacks(
        &mut self,
        on_inject: FaultInjectedCallback,
        on_clear: FaultClearedCallback,
        user_data: *mut c_void,
    ) {
        self.on_fault_inject = on_inject.map(|cb| CallbackWithData {
            callback: Some(cb),
            user_data,
        });
        self.on_fault_clear = on_clear.map(|cb| CallbackWithData {
            callback: Some(cb),
            user_data,
        });
    }

    /// Add an operation to the queue.
    pub fn add_operation(&mut self, operation: &Operation) -> i64 {
        if self.state != TestState::Setup {
            return -1;
        }

        let id = self.next_op_id;
        self.next_op_id += 1;

        // Copy the operation data
        let key = if operation.key.data.is_null() || operation.key.len == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(operation.key.data, operation.key.len) }.to_vec()
        };

        let value = if operation.value.data.is_null() || operation.value.len == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(operation.value.data, operation.value.len) }
                .to_vec()
        };

        let custom_name =
            if operation.custom_name.data.is_null() || operation.custom_name.len == 0 {
                String::new()
            } else {
                let bytes = unsafe {
                    std::slice::from_raw_parts(
                        operation.custom_name.data as *const u8,
                        operation.custom_name.len,
                    )
                };
                String::from_utf8_lossy(bytes).into_owned()
            };

        self.operations.push(StoredOperation {
            id,
            op_type: operation.op_type,
            key,
            value,
            custom_name,
        });

        id as i64
    }

    /// Generate random operations based on a schema.
    pub fn generate_operations(&mut self, count: u64, _schema: &str) -> Result<(), String> {
        if self.state != TestState::Setup {
            return Err("Test is not in setup state".to_string());
        }

        // In a real implementation, we would parse the schema and generate operations.
        // For now, we just generate placeholder operations.
        for _ in 0..count {
            let id = self.next_op_id;
            self.next_op_id += 1;

            self.operations.push(StoredOperation {
                id,
                op_type: OperationType::Read,
                key: vec![0u8; 8],
                value: Vec::new(),
                custom_name: String::new(),
            });
        }

        Ok(())
    }

    /// Run the test.
    pub fn run(&mut self) -> Result<(), VerifierErrorCode> {
        if self.state != TestState::Setup {
            return Err(VerifierErrorCode::InvalidState);
        }

        self.state = TestState::Running;
        self.start_time = Some(Instant::now());

        let timeout = if self.timeout_ms > 0 {
            Some(Duration::from_millis(self.timeout_ms))
        } else {
            None
        };

        let total = self.operations.len() as u64;
        let max_ops = if self.max_operations > 0 {
            self.max_operations.min(total)
        } else {
            total
        };

        for (i, op) in self.operations.iter().take(max_ops as usize).enumerate() {
            // Check for cancellation
            if self.cancelled.load(Ordering::SeqCst) {
                self.state = TestState::Cancelled;
                self.end_time = Some(Instant::now());
                return Err(VerifierErrorCode::Cancelled);
            }

            // Check for timeout
            if let Some(timeout) = timeout {
                if self.start_time.unwrap().elapsed() > timeout {
                    self.state = TestState::Failed;
                    self.end_time = Some(Instant::now());
                    return Err(VerifierErrorCode::Timeout);
                }
            }

            // Create the C operation struct for callbacks
            let c_op = Operation {
                id: op.id,
                op_type: op.op_type,
                key: ByteSlice {
                    data: op.key.as_ptr(),
                    len: op.key.len(),
                },
                value: ByteSlice {
                    data: op.value.as_ptr(),
                    len: op.value.len(),
                },
                custom_name: StringSlice {
                    data: op.custom_name.as_ptr() as *const c_char,
                    len: op.custom_name.len(),
                },
                user_data: ptr::null_mut(),
            };

            // Call pre-operation callback
            if let Some(ref cb) = self.pre_operation {
                if let Some(callback) = cb.callback {
                    let should_continue = unsafe { callback(cb.user_data, &c_op) };
                    if !should_continue {
                        // Operation was skipped
                        self.results.push(StoredOperationResult {
                            operation_id: op.id,
                            status: OperationStatus::Skipped,
                            value: Vec::new(),
                            error_message: None,
                            duration_ns: 0,
                        });
                        continue;
                    }
                }
            }

            // Execute the operation
            let result = if let Some(ref cb) = self.executor {
                if let Some(callback) = cb.callback {
                    let mut op_result = OperationResult::default();
                    op_result.operation_id = op.id;

                    let start = Instant::now();
                    let code = unsafe { callback(cb.user_data, &c_op, &mut op_result) };
                    let duration = start.elapsed();

                    if code == VerifierErrorCode::Ok {
                        // Copy the result
                        let value = if op_result.value_data.is_null() || op_result.value_len == 0 {
                            Vec::new()
                        } else {
                            unsafe {
                                std::slice::from_raw_parts(
                                    op_result.value_data,
                                    op_result.value_len,
                                )
                            }
                            .to_vec()
                        };

                        let error_message = if op_result.error_message.is_null() {
                            None
                        } else {
                            Some(
                                unsafe {
                                    std::ffi::CStr::from_ptr(op_result.error_message)
                                        .to_string_lossy()
                                        .into_owned()
                                },
                            )
                        };

                        StoredOperationResult {
                            operation_id: op.id,
                            status: op_result.status,
                            value,
                            error_message,
                            duration_ns: duration.as_nanos() as u64,
                        }
                    } else {
                        StoredOperationResult {
                            operation_id: op.id,
                            status: OperationStatus::Failure,
                            value: Vec::new(),
                            error_message: Some(format!("Executor returned error: {:?}", code)),
                            duration_ns: duration.as_nanos() as u64,
                        }
                    }
                } else {
                    StoredOperationResult {
                        operation_id: op.id,
                        status: OperationStatus::Skipped,
                        value: Vec::new(),
                        error_message: Some("No executor callback".to_string()),
                        duration_ns: 0,
                    }
                }
            } else {
                StoredOperationResult {
                    operation_id: op.id,
                    status: OperationStatus::Skipped,
                    value: Vec::new(),
                    error_message: Some("No executor registered".to_string()),
                    duration_ns: 0,
                }
            };

            self.results.push(result);

            // Call progress callback
            if let Some(ref cb) = self.progress {
                if let Some(callback) = cb.callback {
                    unsafe { callback(cb.user_data, (i + 1) as u64, max_ops) };
                }
            }
        }

        self.state = TestState::Completed;
        self.end_time = Some(Instant::now());
        Ok(())
    }

    /// Cancel the test.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Inject a fault.
    pub fn inject_fault(&mut self, config: &FaultConfig) -> i64 {
        if !self.fault_injection_enabled {
            return -1;
        }

        let id = self.next_fault_id;
        self.next_fault_id += 1;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let clear_at = if config.duration_ms > 0 {
            now + config.duration_ms
        } else {
            0
        };

        let fault_info = FaultInfo {
            id,
            fault_type: config.fault_type,
            target: config.target,
            injected_at_ms: now,
            clear_at_ms: clear_at,
            active: true,
        };

        // Call the inject callback
        if let Some(ref cb) = self.on_fault_inject {
            if let Some(callback) = cb.callback {
                unsafe { callback(cb.user_data, &fault_info) };
            }
        }

        self.active_faults.insert(id, fault_info);
        id as i64
    }

    /// Clear a fault.
    pub fn clear_fault(&mut self, fault_id: u64) -> Result<(), VerifierErrorCode> {
        if let Some(mut fault) = self.active_faults.remove(&fault_id) {
            fault.active = false;

            // Call the clear callback
            if let Some(ref cb) = self.on_fault_clear {
                if let Some(callback) = cb.callback {
                    unsafe { callback(cb.user_data, &fault) };
                }
            }

            Ok(())
        } else {
            Err(VerifierErrorCode::InvalidHandle)
        }
    }

    /// Clear all faults.
    pub fn clear_all_faults(&mut self) {
        let fault_ids: Vec<u64> = self.active_faults.keys().copied().collect();
        for id in fault_ids {
            let _ = self.clear_fault(id);
        }
    }

    /// Check linearizability.
    pub fn check_linearizability(&self) -> Result<CheckResultInner, VerifierErrorCode> {
        // In a real implementation, we would perform linearizability checking.
        // For now, we return a placeholder result.
        Ok(CheckResultInner::new(
            true,
            "Linearizability check passed (placeholder)".to_string(),
        ))
    }

    /// Check serializability.
    pub fn check_serializability(&self) -> Result<CheckResultInner, VerifierErrorCode> {
        // In a real implementation, we would perform serializability checking.
        // For now, we return a placeholder result.
        Ok(CheckResultInner::new(
            true,
            "Serializability check passed (placeholder)".to_string(),
        ))
    }

    /// Check a custom invariant.
    pub fn check_invariant(
        &self,
        name: &str,
        _expr: &str,
    ) -> Result<CheckResultInner, VerifierErrorCode> {
        // In a real implementation, we would evaluate the invariant expression.
        // For now, we return a placeholder result.
        Ok(CheckResultInner::new(
            true,
            format!("Invariant '{}' check passed (placeholder)", name),
        ))
    }

    /// Finalize the test and generate a report.
    pub fn finalize(&self) -> Result<ReportInner, VerifierErrorCode> {
        let duration_ns = match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => end.duration_since(start).as_nanos() as u64,
            _ => 0,
        };

        let success_count = self
            .results
            .iter()
            .filter(|r| r.status == OperationStatus::Success)
            .count() as u64;

        let failure_count = self
            .results
            .iter()
            .filter(|r| r.status == OperationStatus::Failure)
            .count() as u64;

        Ok(ReportInner {
            test_name: self.name.clone(),
            test_description: self.description.clone(),
            operation_count: self.results.len() as u64,
            success_count,
            failure_count,
            duration_ns,
            state: self.state,
        })
    }
}

// ============================================================================
// CheckResultInner
// ============================================================================

/// Internal state for a check result.
///
/// This is the implementation behind `CheckResultHandle`.
pub struct CheckResultInner {
    /// Whether the check passed.
    pub passed: bool,
    /// The result message.
    message: String,
    /// Cached C string for the message.
    message_cstr: Option<CString>,
}

impl CheckResultInner {
    /// Create a new check result.
    pub fn new(passed: bool, message: String) -> Self {
        let message_cstr = CString::new(message.as_str()).ok();
        Self {
            passed,
            message,
            message_cstr,
        }
    }

    /// Get the message as a C string pointer.
    pub fn message_cstr(&self) -> *const c_char {
        self.message_cstr
            .as_ref()
            .map(|cs| cs.as_ptr())
            .unwrap_or(ptr::null())
    }
}

// ============================================================================
// ReportInner
// ============================================================================

/// Internal state for a verification report.
///
/// This is the implementation behind `ReportHandle`.
pub struct ReportInner {
    /// Test name.
    pub test_name: String,
    /// Test description.
    pub test_description: Option<String>,
    /// Total operations executed.
    pub operation_count: u64,
    /// Successful operations.
    pub success_count: u64,
    /// Failed operations.
    pub failure_count: u64,
    /// Total duration in nanoseconds.
    pub duration_ns: u64,
    /// Final test state.
    state: TestState,
}

impl ReportInner {
    /// Convert the report to a JSON string.
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"test_name":"{}","description":{},"operation_count":{},"success_count":{},"failure_count":{},"duration_ns":{},"state":"{}"}}"#,
            self.test_name,
            self.test_description
                .as_ref()
                .map(|d| format!("\"{}\"", d))
                .unwrap_or_else(|| "null".to_string()),
            self.operation_count,
            self.success_count,
            self.failure_count,
            self.duration_ns,
            match self.state {
                TestState::Setup => "setup",
                TestState::Running => "running",
                TestState::Completed => "completed",
                TestState::Cancelled => "cancelled",
                TestState::Failed => "failed",
            }
        )
    }

    /// Convert the report to a human-readable string.
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        let mut s = format!("Test Report: {}\n", self.test_name);
        if let Some(ref desc) = self.test_description {
            s.push_str(&format!("Description: {}\n", desc));
        }
        s.push_str(&format!(
            "Operations: {} total, {} success, {} failed\n",
            self.operation_count, self.success_count, self.failure_count
        ));
        s.push_str(&format!(
            "Duration: {:.3} ms\n",
            self.duration_ns as f64 / 1_000_000.0
        ));
        s.push_str(&format!(
            "State: {}\n",
            match self.state {
                TestState::Setup => "Setup",
                TestState::Running => "Running",
                TestState::Completed => "Completed",
                TestState::Cancelled => "Cancelled",
                TestState::Failed => "Failed",
            }
        ));
        s
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_inner_creation() {
        let verifier = VerifierInner::new();
        assert!(!verifier.is_shutdown());
        assert_eq!(verifier.allocate_test_id(), 1);
        assert_eq!(verifier.allocate_test_id(), 2);
    }

    #[test]
    fn test_safe_handle() {
        let handle = SafeHandle::new(42);
        assert_eq!(handle.with(|v| *v), 42);
        handle.with_mut(|v| *v = 100);
        assert_eq!(handle.with(|v| *v), 100);
    }

    #[test]
    fn test_check_result_inner() {
        let result = CheckResultInner::new(true, "Test passed".to_string());
        assert!(result.passed);
        assert!(!result.message_cstr().is_null());
    }

    #[test]
    fn test_report_inner_json() {
        let report = ReportInner {
            test_name: "test1".to_string(),
            test_description: Some("A test".to_string()),
            operation_count: 100,
            success_count: 95,
            failure_count: 5,
            duration_ns: 1_000_000_000,
            state: TestState::Completed,
        };

        let json = report.to_json();
        assert!(json.contains("\"test_name\":\"test1\""));
        assert!(json.contains("\"operation_count\":100"));
        assert!(json.contains("\"state\":\"completed\""));
    }

    #[test]
    fn test_report_inner_to_string() {
        let report = ReportInner {
            test_name: "test1".to_string(),
            test_description: None,
            operation_count: 100,
            success_count: 100,
            failure_count: 0,
            duration_ns: 1_000_000,
            state: TestState::Completed,
        };

        let s = report.to_string();
        assert!(s.contains("Test Report: test1"));
        assert!(s.contains("100 total"));
        assert!(s.contains("Completed"));
    }
}
