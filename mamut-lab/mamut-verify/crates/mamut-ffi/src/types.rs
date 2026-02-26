//! FFI type definitions matching the C header.
//!
//! All types in this module use `#[repr(C)]` to ensure C-compatible memory layout.
//! These types are used for data exchange across the FFI boundary.

use std::ffi::{c_char, c_void};

// ============================================================================
// Opaque Handle Types
// ============================================================================

/// Opaque handle to the verification engine.
///
/// This is a placeholder type used for type safety at the FFI boundary.
/// The actual implementation is in `handles::VerifierInner`.
#[repr(C)]
pub struct VerifierHandle {
    _private: [u8; 0],
}

/// Opaque handle to a test execution context.
///
/// This is a placeholder type used for type safety at the FFI boundary.
/// The actual implementation is in `handles::TestInner`.
#[repr(C)]
pub struct TestHandle {
    _private: [u8; 0],
}

/// Opaque handle to a check result.
///
/// This is a placeholder type used for type safety at the FFI boundary.
/// The actual implementation is in `handles::CheckResultInner`.
#[repr(C)]
pub struct CheckResultHandle {
    _private: [u8; 0],
}

/// Opaque handle to a verification report.
///
/// This is a placeholder type used for type safety at the FFI boundary.
/// The actual implementation is in `handles::ReportInner`.
#[repr(C)]
pub struct ReportHandle {
    _private: [u8; 0],
}

// ============================================================================
// Slice Types
// ============================================================================

/// A borrowed, non-null-terminated string slice.
///
/// # Memory
///
/// This type borrows data; the underlying memory must remain valid for the
/// duration of any function call using this type.
///
/// # Null Safety
///
/// - `data` may be NULL only if `len` is 0.
/// - The string is NOT null-terminated; use `len` for bounds.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StringSlice {
    /// Pointer to UTF-8 encoded string data. May be NULL if len == 0.
    pub data: *const c_char,
    /// Length in bytes (not characters).
    pub len: usize,
}

impl Default for StringSlice {
    fn default() -> Self {
        Self {
            data: std::ptr::null(),
            len: 0,
        }
    }
}

impl StringSlice {
    /// Create a new StringSlice from a Rust string slice.
    pub fn from_str(s: &str) -> Self {
        Self {
            data: s.as_ptr() as *const c_char,
            len: s.len(),
        }
    }

    /// Check if the slice is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// A borrowed byte slice.
///
/// # Memory
///
/// This type borrows data; the underlying memory must remain valid for the
/// duration of any function call using this type.
///
/// # Null Safety
///
/// `data` may be NULL only if `len` is 0.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ByteSlice {
    /// Pointer to byte data. May be NULL if len == 0.
    pub data: *const u8,
    /// Length in bytes.
    pub len: usize,
}

impl Default for ByteSlice {
    fn default() -> Self {
        Self {
            data: std::ptr::null(),
            len: 0,
        }
    }
}

impl ByteSlice {
    /// Create a new ByteSlice from a Rust byte slice.
    pub fn from_slice(s: &[u8]) -> Self {
        Self {
            data: s.as_ptr(),
            len: s.len(),
        }
    }

    /// Check if the slice is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

// ============================================================================
// Error Types
// ============================================================================

/// Error codes returned by verifier operations.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VerifierErrorCode {
    /// Operation completed successfully.
    Ok = 0,
    /// A null pointer was passed where a valid pointer was required.
    NullPointer = 1,
    /// The provided handle is invalid or has been freed.
    InvalidHandle = 2,
    /// A string argument contained invalid UTF-8.
    InvalidUtf8 = 3,
    /// The requested operation is not valid in the current state.
    InvalidState = 4,
    /// Memory allocation failed.
    OutOfMemory = 5,
    /// An I/O error occurred.
    Io = 6,
    /// A timeout occurred during the operation.
    Timeout = 7,
    /// The operation was cancelled.
    Cancelled = 8,
    /// A verification invariant was violated.
    InvariantViolation = 9,
    /// An internal error occurred.
    Internal = 10,
    /// The configuration is invalid.
    InvalidConfig = 11,
    /// The operation failed due to an injected fault.
    FaultInjected = 12,
    /// An unknown or unspecified error occurred.
    Unknown = 255,
}

impl std::fmt::Display for VerifierErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ok => write!(f, "OK"),
            Self::NullPointer => write!(f, "Null pointer"),
            Self::InvalidHandle => write!(f, "Invalid handle"),
            Self::InvalidUtf8 => write!(f, "Invalid UTF-8"),
            Self::InvalidState => write!(f, "Invalid state"),
            Self::OutOfMemory => write!(f, "Out of memory"),
            Self::Io => write!(f, "I/O error"),
            Self::Timeout => write!(f, "Timeout"),
            Self::Cancelled => write!(f, "Cancelled"),
            Self::InvariantViolation => write!(f, "Invariant violation"),
            Self::Internal => write!(f, "Internal error"),
            Self::InvalidConfig => write!(f, "Invalid configuration"),
            Self::FaultInjected => write!(f, "Fault injected"),
            Self::Unknown => write!(f, "Unknown error"),
        }
    }
}

impl std::error::Error for VerifierErrorCode {}

/// Extended error information.
///
/// # Memory
///
/// The `message` field is owned and must be freed as part of the containing
/// `VerifierResult` using `verifier_result_free()`.
#[repr(C)]
#[derive(Debug)]
pub struct VerifierError {
    /// The error code.
    pub code: VerifierErrorCode,
    /// Human-readable error message. Owned, may be NULL.
    pub message: *mut c_char,
}

/// Result type tag for VerifierResult discriminated union.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifierResultTag {
    /// The result contains a success value.
    Ok = 0,
    /// The result contains an error.
    Err = 1,
}

/// Union payload for VerifierResult.
#[repr(C)]
pub union VerifierResultValue {
    /// Valid when tag == Ok. Contains a handle or NULL.
    pub ok: *mut c_void,
    /// Valid when tag == Err. Contains error details.
    pub err: VerifierError,
}

/// Discriminated union for operation results.
///
/// # Usage
///
/// Check the `tag` field to determine which union member is valid:
/// - `VerifierResultTag::Ok`: Access `value.ok` for the success value.
/// - `VerifierResultTag::Err`: Access `value.err` for error details.
///
/// # Memory
///
/// Must be freed with `verifier_result_free()` when no longer needed.
#[repr(C)]
pub struct VerifierResult {
    /// Discriminant indicating which union field is valid.
    pub tag: VerifierResultTag,
    /// The result value.
    pub value: VerifierResultValue,
}

// ============================================================================
// Configuration Types
// ============================================================================

/// Execution mode for tests.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ExecutionMode {
    /// Execute operations sequentially.
    #[default]
    Sequential = 0,
    /// Execute operations with controlled concurrency.
    Concurrent = 1,
    /// Execute with deterministic scheduling for reproducibility.
    Deterministic = 2,
}

/// Configuration for creating a new test.
///
/// # Memory
///
/// String fields (`name`, `description`) are borrowed and must remain valid
/// during the call to `verifier_create_test()`.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TestConfig {
    /// Test name. Borrowed.
    pub name: StringSlice,
    /// Optional test description. Borrowed.
    pub description: StringSlice,
    /// Maximum number of operations to execute. 0 for unlimited.
    pub max_operations: u64,
    /// Timeout in milliseconds. 0 for no timeout.
    pub timeout_ms: u64,
    /// Random seed for deterministic execution. 0 for random.
    pub seed: u64,
    /// Execution mode.
    pub mode: ExecutionMode,
    /// Enable verbose logging.
    pub verbose: bool,
    /// Enable invariant checking after each operation.
    pub check_invariants: bool,
    /// Enable fault injection.
    pub enable_fault_injection: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            name: StringSlice::default(),
            description: StringSlice::default(),
            max_operations: 0,
            timeout_ms: 0,
            seed: 0,
            mode: ExecutionMode::Sequential,
            verbose: false,
            check_invariants: true,
            enable_fault_injection: false,
        }
    }
}

// ============================================================================
// Operation Types
// ============================================================================

/// Type of operation being performed.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationType {
    /// Read operation.
    Read = 0,
    /// Write operation.
    Write = 1,
    /// Delete operation.
    Delete = 2,
    /// Transaction begin.
    TxnBegin = 3,
    /// Transaction commit.
    TxnCommit = 4,
    /// Transaction abort.
    TxnAbort = 5,
    /// Barrier/fence operation.
    Barrier = 6,
    /// Custom operation type.
    Custom = 255,
}

impl Default for OperationType {
    fn default() -> Self {
        Self::Read
    }
}

/// An operation to be executed by the system under test.
///
/// # Memory
///
/// - `key`, `value`, and `custom_name` are borrowed slices.
/// - `user_data` is an opaque pointer managed by the caller.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Operation {
    /// Unique operation ID within the test.
    pub id: u64,
    /// Type of operation.
    pub op_type: OperationType,
    /// Key for the operation. Borrowed.
    pub key: ByteSlice,
    /// Value for write operations. Borrowed.
    pub value: ByteSlice,
    /// Custom operation name for Custom type. Borrowed.
    pub custom_name: StringSlice,
    /// Opaque user data pointer.
    pub user_data: *mut c_void,
}

impl Default for Operation {
    fn default() -> Self {
        Self {
            id: 0,
            op_type: OperationType::Read,
            key: ByteSlice::default(),
            value: ByteSlice::default(),
            custom_name: StringSlice::default(),
            user_data: std::ptr::null_mut(),
        }
    }
}

// Safety: Operation can be sent between threads as it only contains raw pointers
// that the caller is responsible for synchronizing.
unsafe impl Send for Operation {}

/// Status of an operation result.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationStatus {
    /// Operation succeeded.
    Success = 0,
    /// Operation failed with an error.
    Failure = 1,
    /// Operation was skipped.
    Skipped = 2,
    /// Operation timed out.
    Timeout = 3,
    /// Operation was aborted due to fault injection.
    FaultInjected = 4,
}

impl Default for OperationStatus {
    fn default() -> Self {
        Self::Success
    }
}

/// Result of executing an operation.
///
/// # Memory
///
/// - `value_data` is owned; free with the containing result.
/// - `error_message` is owned; free with the containing result.
/// - Use `verifier_operation_result_free()` to free all owned memory.
#[repr(C)]
#[derive(Debug)]
pub struct OperationResult {
    /// The operation ID this result corresponds to.
    pub operation_id: u64,
    /// Status of the operation.
    pub status: OperationStatus,
    /// Returned value for read operations. Owned.
    pub value_data: *mut u8,
    /// Length of the returned value.
    pub value_len: usize,
    /// Error message if status is Failure. Owned.
    pub error_message: *mut c_char,
    /// Duration in nanoseconds.
    pub duration_ns: u64,
}

impl Default for OperationResult {
    fn default() -> Self {
        Self {
            operation_id: 0,
            status: OperationStatus::Success,
            value_data: std::ptr::null_mut(),
            value_len: 0,
            error_message: std::ptr::null_mut(),
            duration_ns: 0,
        }
    }
}

// ============================================================================
// Fault Injection Types
// ============================================================================

/// Type of fault to inject.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FaultType {
    /// No fault.
    None = 0,
    /// Simulate a crash/process termination.
    Crash = 1,
    /// Simulate a network partition.
    NetworkPartition = 2,
    /// Simulate disk I/O errors.
    DiskError = 3,
    /// Simulate memory allocation failure.
    Oom = 4,
    /// Simulate CPU contention/slowdown.
    SlowCpu = 5,
    /// Simulate clock skew.
    ClockSkew = 6,
    /// Corrupt data in transit or at rest.
    DataCorruption = 7,
    /// Drop messages/packets.
    MessageDrop = 8,
    /// Reorder messages/operations.
    MessageReorder = 9,
    /// Duplicate messages/operations.
    MessageDuplicate = 10,
    /// Delay messages/operations.
    Delay = 11,
    /// Custom fault type.
    Custom = 255,
}

impl Default for FaultType {
    fn default() -> Self {
        Self::None
    }
}

/// Target for fault injection.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FaultTarget {
    /// Target all components.
    All = 0,
    /// Target storage layer.
    Storage = 1,
    /// Target network layer.
    Network = 2,
    /// Target compute layer.
    Compute = 3,
    /// Target memory subsystem.
    Memory = 4,
    /// Target specific node by ID.
    Node = 5,
    /// Target specific operation by ID.
    Operation = 6,
}

impl Default for FaultTarget {
    fn default() -> Self {
        Self::All
    }
}

/// Configuration for a fault injection.
///
/// # Memory
///
/// `params` is a borrowed byte slice.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FaultConfig {
    /// Type of fault to inject.
    pub fault_type: FaultType,
    /// Target of the fault.
    pub target: FaultTarget,
    /// Target ID when target is specific.
    pub target_id: u64,
    /// Probability of fault occurring (0.0 - 1.0).
    pub probability: f64,
    /// Duration of the fault in milliseconds. 0 for instantaneous.
    pub duration_ms: u64,
    /// Additional fault-specific parameters. Borrowed.
    pub params: ByteSlice,
}

impl Default for FaultConfig {
    fn default() -> Self {
        Self {
            fault_type: FaultType::None,
            target: FaultTarget::All,
            target_id: 0,
            probability: 1.0,
            duration_ms: 0,
            params: ByteSlice::default(),
        }
    }
}

/// Information about an injected fault.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FaultInfo {
    /// Unique fault ID.
    pub id: u64,
    /// Type of fault that was injected.
    pub fault_type: FaultType,
    /// Target of the fault.
    pub target: FaultTarget,
    /// When the fault was injected (Unix timestamp ms).
    pub injected_at_ms: u64,
    /// When the fault will clear (Unix timestamp ms). 0 if permanent.
    pub clear_at_ms: u64,
    /// Whether the fault is currently active.
    pub active: bool,
}

impl Default for FaultInfo {
    fn default() -> Self {
        Self {
            id: 0,
            fault_type: FaultType::None,
            target: FaultTarget::All,
            injected_at_ms: 0,
            clear_at_ms: 0,
            active: false,
        }
    }
}

// ============================================================================
// Callback Types
// ============================================================================

/// Callback invoked when an operation is about to be executed.
///
/// # Parameters
///
/// - `user_data`: Opaque pointer passed during registration.
/// - `operation`: The operation about to be executed.
///
/// # Returns
///
/// `true` to allow the operation, `false` to skip it.
///
/// # Safety
///
/// Must not panic. Panics will be caught but may leave the system in an
/// inconsistent state.
pub type OperationCallback =
    Option<unsafe extern "C" fn(user_data: *mut c_void, operation: *const Operation) -> bool>;

/// Callback to execute an operation on the system under test.
///
/// # Parameters
///
/// - `user_data`: Opaque pointer passed during registration.
/// - `operation`: The operation to execute.
/// - `result`: Output parameter for the result. The callback is responsible
///   for allocating `value_data` and `error_message` using `verifier_alloc()`.
///
/// # Returns
///
/// `VerifierErrorCode::Ok` on success, error code on failure.
///
/// # Safety
///
/// - Must not panic.
/// - Must properly initialize the `result` output parameter.
/// - Allocated memory in `result` must use `verifier_alloc()`.
pub type ExecuteOperationCallback = Option<
    unsafe extern "C" fn(
        user_data: *mut c_void,
        operation: *const Operation,
        result: *mut OperationResult,
    ) -> VerifierErrorCode,
>;

/// Callback invoked when a fault is injected.
///
/// # Parameters
///
/// - `user_data`: Opaque pointer passed during registration.
/// - `fault`: Information about the injected fault.
///
/// # Safety
///
/// Must not panic.
pub type FaultInjectedCallback =
    Option<unsafe extern "C" fn(user_data: *mut c_void, fault: *const FaultInfo)>;

/// Callback invoked when a fault is cleared.
///
/// # Parameters
///
/// - `user_data`: Opaque pointer passed during registration.
/// - `fault`: Information about the cleared fault.
///
/// # Safety
///
/// Must not panic.
pub type FaultClearedCallback =
    Option<unsafe extern "C" fn(user_data: *mut c_void, fault: *const FaultInfo)>;

/// Callback for progress reporting.
///
/// # Parameters
///
/// - `user_data`: Opaque pointer passed during registration.
/// - `completed`: Number of operations completed.
/// - `total`: Total number of operations (0 if unknown).
///
/// # Safety
///
/// Must not panic.
pub type ProgressCallback =
    Option<unsafe extern "C" fn(user_data: *mut c_void, completed: u64, total: u64)>;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn test_type_sizes() {
        // Ensure types have expected sizes for FFI compatibility
        assert_eq!(mem::size_of::<StringSlice>(), mem::size_of::<usize>() * 2);
        assert_eq!(mem::size_of::<ByteSlice>(), mem::size_of::<usize>() * 2);
        assert_eq!(mem::size_of::<VerifierErrorCode>(), 4); // C enum size
        assert_eq!(mem::size_of::<ExecutionMode>(), 4);
        assert_eq!(mem::size_of::<OperationType>(), 4);
        assert_eq!(mem::size_of::<OperationStatus>(), 4);
        assert_eq!(mem::size_of::<FaultType>(), 4);
        assert_eq!(mem::size_of::<FaultTarget>(), 4);
    }

    #[test]
    fn test_string_slice_from_str() {
        let s = "hello";
        let slice = StringSlice::from_str(s);
        assert_eq!(slice.len, 5);
        assert!(!slice.data.is_null());
    }

    #[test]
    fn test_byte_slice_from_slice() {
        let bytes = [1u8, 2, 3, 4, 5];
        let slice = ByteSlice::from_slice(&bytes);
        assert_eq!(slice.len, 5);
        assert!(!slice.data.is_null());
    }

    #[test]
    fn test_default_values() {
        let config = TestConfig::default();
        assert!(config.name.is_empty());
        assert_eq!(config.max_operations, 0);
        assert_eq!(config.mode, ExecutionMode::Sequential);

        let op = Operation::default();
        assert_eq!(op.id, 0);
        assert_eq!(op.op_type, OperationType::Read);

        let result = OperationResult::default();
        assert_eq!(result.status, OperationStatus::Success);
        assert!(result.value_data.is_null());
    }

    #[test]
    fn test_error_code_display() {
        assert_eq!(format!("{}", VerifierErrorCode::Ok), "OK");
        assert_eq!(format!("{}", VerifierErrorCode::NullPointer), "Null pointer");
        assert_eq!(format!("{}", VerifierErrorCode::Internal), "Internal error");
    }
}
