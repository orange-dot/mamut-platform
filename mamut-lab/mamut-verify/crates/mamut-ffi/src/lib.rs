//! MAMUT Verification Harness FFI
//!
//! This crate provides the C FFI bindings for the MAMUT verification harness,
//! enabling integration with C/C++ codebases and other languages via FFI.
//!
//! # Memory Safety
//!
//! This crate uses opaque handle types to manage resources safely across the FFI
//! boundary. All handles must be explicitly freed using their corresponding
//! `*_free()` functions.
//!
//! # Thread Safety
//!
//! - `VerifierHandle` operations are thread-safe (protected by internal locks).
//! - `TestHandle` operations are NOT thread-safe; each test should be used from
//!   a single thread or protected by external synchronization.
//!
//! # Panics
//!
//! This crate catches panics at the FFI boundary and converts them to error codes.
//! No Rust panics will propagate across the FFI boundary.
//!
//! # Example
//!
//! ```c
//! // Initialize the verifier
//! VerifierResult result = verifier_init();
//! if (result.tag != VERIFIER_RESULT_OK) {
//!     // Handle error
//!     verifier_result_free(&result);
//!     return;
//! }
//! VerifierHandle* verifier = result.value.ok;
//!
//! // Create a test
//! TestConfig config = {
//!     .name = verifier_string_from_cstr("my_test"),
//!     .max_operations = 1000,
//!     .timeout_ms = 60000,
//!     .mode = EXECUTION_MODE_SEQUENTIAL,
//! };
//! result = verifier_create_test(verifier, &config);
//! // ...
//!
//! // Clean up
//! verifier_shutdown(verifier);
//! ```

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::missing_safety_doc)] // Safety docs are in the C header

pub mod handles;
pub mod types;

use std::ffi::{c_char, c_void, CStr, CString};
use std::panic::{self, AssertUnwindSafe};
use std::ptr;
use std::slice;

use handles::{CheckResultInner, ReportInner, SafeHandle, TestInner, VerifierInner};
use types::*;

/// Version string for the library.
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");

// ============================================================================
// Helper Macros
// ============================================================================

/// Wraps an FFI function body to catch panics and convert them to error codes.
macro_rules! ffi_catch_panic {
    ($default:expr, $body:expr) => {{
        match panic::catch_unwind(AssertUnwindSafe(|| $body)) {
            Ok(result) => result,
            Err(_) => $default,
        }
    }};
}

/// Validates a pointer is non-null, returning the specified error if null.
macro_rules! check_null {
    ($ptr:expr, $err:expr) => {
        if $ptr.is_null() {
            return $err;
        }
    };
}

/// Converts a StringSlice to a Rust &str, returning an error if invalid UTF-8.
fn string_slice_to_str(slice: &StringSlice) -> Result<&str, VerifierErrorCode> {
    if slice.data.is_null() {
        if slice.len == 0 {
            return Ok("");
        }
        return Err(VerifierErrorCode::NullPointer);
    }
    let bytes = unsafe { slice::from_raw_parts(slice.data as *const u8, slice.len) };
    std::str::from_utf8(bytes).map_err(|_| VerifierErrorCode::InvalidUtf8)
}

/// Converts a ByteSlice to a Rust &[u8].
fn byte_slice_to_slice(slice: &ByteSlice) -> Result<&[u8], VerifierErrorCode> {
    if slice.data.is_null() {
        if slice.len == 0 {
            return Ok(&[]);
        }
        return Err(VerifierErrorCode::NullPointer);
    }
    Ok(unsafe { slice::from_raw_parts(slice.data, slice.len) })
}

/// Creates a VerifierResult with an error.
fn make_error_result(code: VerifierErrorCode, message: Option<&str>) -> VerifierResult {
    let message_ptr = message
        .and_then(|m| CString::new(m).ok())
        .map(|cs| cs.into_raw())
        .unwrap_or(ptr::null_mut());

    VerifierResult {
        tag: VerifierResultTag::Err,
        value: VerifierResultValue {
            err: VerifierError {
                code,
                message: message_ptr,
            },
        },
    }
}

/// Creates a VerifierResult with a success value.
fn make_ok_result(value: *mut c_void) -> VerifierResult {
    VerifierResult {
        tag: VerifierResultTag::Ok,
        value: VerifierResultValue { ok: value },
    }
}

// ============================================================================
// Verifier Lifecycle Functions
// ============================================================================

/// Initialize the verification engine.
///
/// # Safety
///
/// The returned handle must be freed with `verifier_shutdown()`.
#[no_mangle]
pub extern "C" fn verifier_init() -> VerifierResult {
    ffi_catch_panic!(
        make_error_result(VerifierErrorCode::Internal, Some("Panic during init")),
        {
            let handle = Box::new(SafeHandle::new(VerifierInner::new()));
            make_ok_result(Box::into_raw(handle) as *mut c_void)
        }
    )
}

/// Initialize the verification engine with a configuration file.
///
/// # Safety
///
/// - `config_path` must be a valid StringSlice with valid UTF-8 data.
/// - The returned handle must be freed with `verifier_shutdown()`.
#[no_mangle]
pub extern "C" fn verifier_init_with_config(config_path: StringSlice) -> VerifierResult {
    ffi_catch_panic!(
        make_error_result(
            VerifierErrorCode::Internal,
            Some("Panic during init with config")
        ),
        {
            let path = match string_slice_to_str(&config_path) {
                Ok(s) => s,
                Err(code) => return make_error_result(code, Some("Invalid config path")),
            };

            match VerifierInner::with_config(path) {
                Ok(inner) => {
                    let handle = Box::new(SafeHandle::new(inner));
                    make_ok_result(Box::into_raw(handle) as *mut c_void)
                }
                Err(e) => make_error_result(VerifierErrorCode::InvalidConfig, Some(&e)),
            }
        }
    )
}

/// Shut down the verification engine and free all resources.
///
/// # Safety
///
/// - `handle` must be a valid pointer returned by `verifier_init()` or NULL.
/// - After this call, `handle` is invalid and must not be used.
#[no_mangle]
pub extern "C" fn verifier_shutdown(handle: *mut VerifierHandle) -> VerifierErrorCode {
    ffi_catch_panic!(VerifierErrorCode::Internal, {
        if handle.is_null() {
            return VerifierErrorCode::Ok;
        }

        let handle = handle as *mut SafeHandle<VerifierInner>;
        let _ = unsafe { Box::from_raw(handle) };
        VerifierErrorCode::Ok
    })
}

/// Get the version string of the verifier library.
///
/// # Safety
///
/// Returns a statically allocated string. Do not free.
#[no_mangle]
pub extern "C" fn verifier_version() -> *const c_char {
    VERSION.as_ptr() as *const c_char
}

// ============================================================================
// Test Management Functions
// ============================================================================

/// Create a new test context.
///
/// # Safety
///
/// - `verifier` must be a valid VerifierHandle.
/// - `config` must be a valid pointer to a TestConfig.
/// - The returned TestHandle must be freed with `verifier_test_free()`.
#[no_mangle]
pub extern "C" fn verifier_create_test(
    verifier: *mut VerifierHandle,
    config: *const TestConfig,
) -> VerifierResult {
    ffi_catch_panic!(
        make_error_result(VerifierErrorCode::Internal, Some("Panic creating test")),
        {
            check_null!(
                verifier,
                make_error_result(VerifierErrorCode::NullPointer, Some("Null verifier handle"))
            );
            check_null!(
                config,
                make_error_result(VerifierErrorCode::NullPointer, Some("Null config"))
            );

            let verifier = verifier as *mut SafeHandle<VerifierInner>;
            let config = unsafe { &*config };

            let name = match string_slice_to_str(&config.name) {
                Ok(s) => s.to_string(),
                Err(code) => return make_error_result(code, Some("Invalid test name")),
            };

            let description = match string_slice_to_str(&config.description) {
                Ok(s) => {
                    if s.is_empty() {
                        None
                    } else {
                        Some(s.to_string())
                    }
                }
                Err(code) => return make_error_result(code, Some("Invalid description")),
            };

            let inner = TestInner::new(
                unsafe { &*verifier },
                name,
                description,
                config.max_operations,
                config.timeout_ms,
                config.seed,
                config.mode,
                config.verbose,
                config.check_invariants,
                config.enable_fault_injection,
            );

            let handle = Box::new(SafeHandle::new(inner));
            make_ok_result(Box::into_raw(handle) as *mut c_void)
        }
    )
}

/// Free a test handle and its resources.
///
/// # Safety
///
/// - `test` must be a valid TestHandle or NULL.
/// - After this call, `test` is invalid and must not be used.
#[no_mangle]
pub extern "C" fn verifier_test_free(test: *mut TestHandle) -> VerifierErrorCode {
    ffi_catch_panic!(VerifierErrorCode::Internal, {
        if test.is_null() {
            return VerifierErrorCode::Ok;
        }

        let test = test as *mut SafeHandle<TestInner>;
        let _ = unsafe { Box::from_raw(test) };
        VerifierErrorCode::Ok
    })
}

/// Register the operation execution callback.
///
/// # Safety
///
/// - `test` must be a valid TestHandle.
/// - `callback` must be a valid function pointer.
/// - `user_data` is passed through to the callback; its validity is the caller's responsibility.
#[no_mangle]
pub extern "C" fn verifier_register_executor(
    test: *mut TestHandle,
    callback: ExecuteOperationCallback,
    user_data: *mut c_void,
) -> VerifierErrorCode {
    ffi_catch_panic!(VerifierErrorCode::Internal, {
        check_null!(test, VerifierErrorCode::NullPointer);

        let test = test as *mut SafeHandle<TestInner>;
        let test = unsafe { &mut *test };

        test.with_mut(|inner| {
            inner.set_executor(callback, user_data);
        });

        VerifierErrorCode::Ok
    })
}

/// Register a pre-operation callback.
///
/// # Safety
///
/// - `test` must be a valid TestHandle.
/// - `callback` must be a valid function pointer.
/// - `user_data` is passed through to the callback.
#[no_mangle]
pub extern "C" fn verifier_register_pre_operation(
    test: *mut TestHandle,
    callback: OperationCallback,
    user_data: *mut c_void,
) -> VerifierErrorCode {
    ffi_catch_panic!(VerifierErrorCode::Internal, {
        check_null!(test, VerifierErrorCode::NullPointer);

        let test = test as *mut SafeHandle<TestInner>;
        let test = unsafe { &mut *test };

        test.with_mut(|inner| {
            inner.set_pre_operation_callback(callback, user_data);
        });

        VerifierErrorCode::Ok
    })
}

/// Register a progress callback.
///
/// # Safety
///
/// - `test` must be a valid TestHandle.
/// - `callback` must be a valid function pointer.
/// - `user_data` is passed through to the callback.
#[no_mangle]
pub extern "C" fn verifier_register_progress(
    test: *mut TestHandle,
    callback: ProgressCallback,
    user_data: *mut c_void,
) -> VerifierErrorCode {
    ffi_catch_panic!(VerifierErrorCode::Internal, {
        check_null!(test, VerifierErrorCode::NullPointer);

        let test = test as *mut SafeHandle<TestInner>;
        let test = unsafe { &mut *test };

        test.with_mut(|inner| {
            inner.set_progress_callback(callback, user_data);
        });

        VerifierErrorCode::Ok
    })
}

// ============================================================================
// Operation Functions
// ============================================================================

/// Add an operation to the test queue.
///
/// # Safety
///
/// - `test` must be a valid TestHandle.
/// - `operation` must be a valid pointer to an Operation.
///
/// # Returns
///
/// The operation ID on success (>= 0), or -1 on failure.
#[no_mangle]
pub extern "C" fn verifier_add_operation(test: *mut TestHandle, operation: *const Operation) -> i64 {
    ffi_catch_panic!(-1, {
        if test.is_null() || operation.is_null() {
            return -1;
        }

        let test = test as *mut SafeHandle<TestInner>;
        let test = unsafe { &mut *test };
        let operation = unsafe { &*operation };

        test.with_mut(|inner| inner.add_operation(operation))
            .unwrap_or(-1)
    })
}

/// Generate random operations based on a schema.
///
/// # Safety
///
/// - `test` must be a valid TestHandle.
/// - `schema` must be a valid StringSlice containing valid JSON.
#[no_mangle]
pub extern "C" fn verifier_generate_operations(
    test: *mut TestHandle,
    count: u64,
    schema: StringSlice,
) -> VerifierErrorCode {
    ffi_catch_panic!(VerifierErrorCode::Internal, {
        check_null!(test, VerifierErrorCode::NullPointer);

        let test = test as *mut SafeHandle<TestInner>;
        let test = unsafe { &mut *test };

        let schema_str = match string_slice_to_str(&schema) {
            Ok(s) => s,
            Err(code) => return code,
        };

        test.with_mut(|inner| match inner.generate_operations(count, schema_str) {
            Ok(()) => VerifierErrorCode::Ok,
            Err(_) => VerifierErrorCode::InvalidConfig,
        })
    })
}

/// Run the test, executing all queued operations.
///
/// # Safety
///
/// - `test` must be a valid TestHandle.
#[no_mangle]
pub extern "C" fn verifier_run_test(test: *mut TestHandle) -> VerifierErrorCode {
    ffi_catch_panic!(VerifierErrorCode::Internal, {
        check_null!(test, VerifierErrorCode::NullPointer);

        let test = test as *mut SafeHandle<TestInner>;
        let test = unsafe { &mut *test };

        test.with_mut(|inner| match inner.run() {
            Ok(()) => VerifierErrorCode::Ok,
            Err(code) => code,
        })
    })
}

/// Cancel a running test.
///
/// # Safety
///
/// - `test` must be a valid TestHandle.
#[no_mangle]
pub extern "C" fn verifier_cancel_test(test: *mut TestHandle) -> VerifierErrorCode {
    ffi_catch_panic!(VerifierErrorCode::Internal, {
        check_null!(test, VerifierErrorCode::NullPointer);

        let test = test as *mut SafeHandle<TestInner>;
        let test = unsafe { &mut *test };

        test.with_mut(|inner| {
            inner.cancel();
        });

        VerifierErrorCode::Ok
    })
}

// ============================================================================
// Fault Injection Functions
// ============================================================================

/// Inject a fault into the test.
///
/// # Safety
///
/// - `test` must be a valid TestHandle.
/// - `config` must be a valid pointer to a FaultConfig.
///
/// # Returns
///
/// The fault ID on success (>= 0), or -1 on failure.
#[no_mangle]
pub extern "C" fn verifier_inject_fault(test: *mut TestHandle, config: *const FaultConfig) -> i64 {
    ffi_catch_panic!(-1, {
        if test.is_null() || config.is_null() {
            return -1;
        }

        let test = test as *mut SafeHandle<TestInner>;
        let test = unsafe { &mut *test };
        let config = unsafe { &*config };

        test.with_mut(|inner| inner.inject_fault(config))
            .unwrap_or(-1)
    })
}

/// Clear a previously injected fault.
///
/// # Safety
///
/// - `test` must be a valid TestHandle.
#[no_mangle]
pub extern "C" fn verifier_clear_fault(test: *mut TestHandle, fault_id: u64) -> VerifierErrorCode {
    ffi_catch_panic!(VerifierErrorCode::Internal, {
        check_null!(test, VerifierErrorCode::NullPointer);

        let test = test as *mut SafeHandle<TestInner>;
        let test = unsafe { &mut *test };

        test.with_mut(|inner| match inner.clear_fault(fault_id) {
            Ok(()) => VerifierErrorCode::Ok,
            Err(code) => code,
        })
    })
}

/// Clear all active faults.
///
/// # Safety
///
/// - `test` must be a valid TestHandle.
#[no_mangle]
pub extern "C" fn verifier_clear_all_faults(test: *mut TestHandle) -> VerifierErrorCode {
    ffi_catch_panic!(VerifierErrorCode::Internal, {
        check_null!(test, VerifierErrorCode::NullPointer);

        let test = test as *mut SafeHandle<TestInner>;
        let test = unsafe { &mut *test };

        test.with_mut(|inner| {
            inner.clear_all_faults();
        });

        VerifierErrorCode::Ok
    })
}

/// Register fault injection callbacks.
///
/// # Safety
///
/// - `test` must be a valid TestHandle.
/// - Callbacks may be NULL (will be ignored).
/// - `user_data` is passed through to the callbacks.
#[no_mangle]
pub extern "C" fn verifier_register_fault_callbacks(
    test: *mut TestHandle,
    on_inject: FaultInjectedCallback,
    on_clear: FaultClearedCallback,
    user_data: *mut c_void,
) -> VerifierErrorCode {
    ffi_catch_panic!(VerifierErrorCode::Internal, {
        check_null!(test, VerifierErrorCode::NullPointer);

        let test = test as *mut SafeHandle<TestInner>;
        let test = unsafe { &mut *test };

        test.with_mut(|inner| {
            inner.set_fault_callbacks(on_inject, on_clear, user_data);
        });

        VerifierErrorCode::Ok
    })
}

// ============================================================================
// Verification Functions
// ============================================================================

/// Check a linearizability property.
///
/// # Safety
///
/// - `test` must be a valid TestHandle.
/// - The returned CheckResultHandle must be freed with `verifier_check_result_free()`.
#[no_mangle]
pub extern "C" fn verifier_check_linearizability(test: *mut TestHandle) -> VerifierResult {
    ffi_catch_panic!(
        make_error_result(
            VerifierErrorCode::Internal,
            Some("Panic during linearizability check")
        ),
        {
            check_null!(
                test,
                make_error_result(VerifierErrorCode::NullPointer, Some("Null test handle"))
            );

            let test = test as *mut SafeHandle<TestInner>;
            let test = unsafe { &*test };

            match test.with(|inner| inner.check_linearizability()) {
                Ok(result) => {
                    let handle = Box::new(SafeHandle::new(result));
                    make_ok_result(Box::into_raw(handle) as *mut c_void)
                }
                Err(code) => make_error_result(code, Some("Linearizability check failed")),
            }
        }
    )
}

/// Check a serializability property.
///
/// # Safety
///
/// - `test` must be a valid TestHandle.
/// - The returned CheckResultHandle must be freed with `verifier_check_result_free()`.
#[no_mangle]
pub extern "C" fn verifier_check_serializability(test: *mut TestHandle) -> VerifierResult {
    ffi_catch_panic!(
        make_error_result(
            VerifierErrorCode::Internal,
            Some("Panic during serializability check")
        ),
        {
            check_null!(
                test,
                make_error_result(VerifierErrorCode::NullPointer, Some("Null test handle"))
            );

            let test = test as *mut SafeHandle<TestInner>;
            let test = unsafe { &*test };

            match test.with(|inner| inner.check_serializability()) {
                Ok(result) => {
                    let handle = Box::new(SafeHandle::new(result));
                    make_ok_result(Box::into_raw(handle) as *mut c_void)
                }
                Err(code) => make_error_result(code, Some("Serializability check failed")),
            }
        }
    )
}

/// Check a custom invariant.
///
/// # Safety
///
/// - `test` must be a valid TestHandle.
/// - `invariant_name` and `invariant_expr` must be valid StringSlices.
/// - The returned CheckResultHandle must be freed with `verifier_check_result_free()`.
#[no_mangle]
pub extern "C" fn verifier_check_invariant(
    test: *mut TestHandle,
    invariant_name: StringSlice,
    invariant_expr: StringSlice,
) -> VerifierResult {
    ffi_catch_panic!(
        make_error_result(
            VerifierErrorCode::Internal,
            Some("Panic during invariant check")
        ),
        {
            check_null!(
                test,
                make_error_result(VerifierErrorCode::NullPointer, Some("Null test handle"))
            );

            let name = match string_slice_to_str(&invariant_name) {
                Ok(s) => s,
                Err(code) => return make_error_result(code, Some("Invalid invariant name")),
            };

            let expr = match string_slice_to_str(&invariant_expr) {
                Ok(s) => s,
                Err(code) => return make_error_result(code, Some("Invalid invariant expression")),
            };

            let test = test as *mut SafeHandle<TestInner>;
            let test = unsafe { &*test };

            match test.with(|inner| inner.check_invariant(name, expr)) {
                Ok(result) => {
                    let handle = Box::new(SafeHandle::new(result));
                    make_ok_result(Box::into_raw(handle) as *mut c_void)
                }
                Err(code) => make_error_result(code, Some("Invariant check failed")),
            }
        }
    )
}

/// Free a check result handle.
///
/// # Safety
///
/// - `result` must be a valid CheckResultHandle or NULL.
#[no_mangle]
pub extern "C" fn verifier_check_result_free(result: *mut CheckResultHandle) -> VerifierErrorCode {
    ffi_catch_panic!(VerifierErrorCode::Internal, {
        if result.is_null() {
            return VerifierErrorCode::Ok;
        }

        let result = result as *mut SafeHandle<CheckResultInner>;
        let _ = unsafe { Box::from_raw(result) };
        VerifierErrorCode::Ok
    })
}

/// Get whether a check passed.
///
/// # Safety
///
/// - `result` must be a valid CheckResultHandle.
#[no_mangle]
pub extern "C" fn verifier_check_result_passed(result: *const CheckResultHandle) -> bool {
    ffi_catch_panic!(false, {
        if result.is_null() {
            return false;
        }

        let result = result as *const SafeHandle<CheckResultInner>;
        let result = unsafe { &*result };

        result.with(|inner| inner.passed)
    })
}

/// Get the check result message.
///
/// # Safety
///
/// - `result` must be a valid CheckResultHandle.
/// - The returned string is borrowed from `result` and valid until
///   `verifier_check_result_free()` is called.
#[no_mangle]
pub extern "C" fn verifier_check_result_message(result: *const CheckResultHandle) -> *const c_char {
    ffi_catch_panic!(ptr::null(), {
        if result.is_null() {
            return ptr::null();
        }

        let result = result as *const SafeHandle<CheckResultInner>;
        let result = unsafe { &*result };

        result.with(|inner| inner.message_cstr())
    })
}

// ============================================================================
// Report Functions
// ============================================================================

/// Finalize the test and generate a report.
///
/// # Safety
///
/// - `test` must be a valid TestHandle.
/// - The returned ReportHandle must be freed with `verifier_report_free()`.
#[no_mangle]
pub extern "C" fn verifier_finalize_test(test: *mut TestHandle) -> VerifierResult {
    ffi_catch_panic!(
        make_error_result(
            VerifierErrorCode::Internal,
            Some("Panic during test finalization")
        ),
        {
            check_null!(
                test,
                make_error_result(VerifierErrorCode::NullPointer, Some("Null test handle"))
            );

            let test = test as *mut SafeHandle<TestInner>;
            let test = unsafe { &*test };

            match test.with(|inner| inner.finalize()) {
                Ok(report) => {
                    let handle = Box::new(SafeHandle::new(report));
                    make_ok_result(Box::into_raw(handle) as *mut c_void)
                }
                Err(code) => make_error_result(code, Some("Failed to finalize test")),
            }
        }
    )
}

/// Free a report handle.
///
/// # Safety
///
/// - `report` must be a valid ReportHandle or NULL.
#[no_mangle]
pub extern "C" fn verifier_report_free(report: *mut ReportHandle) -> VerifierErrorCode {
    ffi_catch_panic!(VerifierErrorCode::Internal, {
        if report.is_null() {
            return VerifierErrorCode::Ok;
        }

        let report = report as *mut SafeHandle<ReportInner>;
        let _ = unsafe { Box::from_raw(report) };
        VerifierErrorCode::Ok
    })
}

/// Get the report as a JSON string.
///
/// # Safety
///
/// - `report` must be a valid ReportHandle.
/// - The returned string is owned by the caller; free with `verifier_free_string()`.
#[no_mangle]
pub extern "C" fn verifier_report_to_json(report: *const ReportHandle) -> *mut c_char {
    ffi_catch_panic!(ptr::null_mut(), {
        if report.is_null() {
            return ptr::null_mut();
        }

        let report = report as *const SafeHandle<ReportInner>;
        let report = unsafe { &*report };

        report.with(|inner| {
            let json = inner.to_json();
            CString::new(json)
                .map(|cs| cs.into_raw())
                .unwrap_or(ptr::null_mut())
        })
    })
}

/// Get the report as a human-readable string.
///
/// # Safety
///
/// - `report` must be a valid ReportHandle.
/// - The returned string is owned by the caller; free with `verifier_free_string()`.
#[no_mangle]
pub extern "C" fn verifier_report_to_string(report: *const ReportHandle) -> *mut c_char {
    ffi_catch_panic!(ptr::null_mut(), {
        if report.is_null() {
            return ptr::null_mut();
        }

        let report = report as *const SafeHandle<ReportInner>;
        let report = unsafe { &*report };

        report.with(|inner| {
            let s = inner.to_string();
            CString::new(s)
                .map(|cs| cs.into_raw())
                .unwrap_or(ptr::null_mut())
        })
    })
}

/// Get the total number of operations executed.
///
/// # Safety
///
/// - `report` must be a valid ReportHandle.
#[no_mangle]
pub extern "C" fn verifier_report_operation_count(report: *const ReportHandle) -> u64 {
    ffi_catch_panic!(0, {
        if report.is_null() {
            return 0;
        }

        let report = report as *const SafeHandle<ReportInner>;
        let report = unsafe { &*report };

        report.with(|inner| inner.operation_count)
    })
}

/// Get the number of successful operations.
///
/// # Safety
///
/// - `report` must be a valid ReportHandle.
#[no_mangle]
pub extern "C" fn verifier_report_success_count(report: *const ReportHandle) -> u64 {
    ffi_catch_panic!(0, {
        if report.is_null() {
            return 0;
        }

        let report = report as *const SafeHandle<ReportInner>;
        let report = unsafe { &*report };

        report.with(|inner| inner.success_count)
    })
}

/// Get the number of failed operations.
///
/// # Safety
///
/// - `report` must be a valid ReportHandle.
#[no_mangle]
pub extern "C" fn verifier_report_failure_count(report: *const ReportHandle) -> u64 {
    ffi_catch_panic!(0, {
        if report.is_null() {
            return 0;
        }

        let report = report as *const SafeHandle<ReportInner>;
        let report = unsafe { &*report };

        report.with(|inner| inner.failure_count)
    })
}

/// Get the total test duration in nanoseconds.
///
/// # Safety
///
/// - `report` must be a valid ReportHandle.
#[no_mangle]
pub extern "C" fn verifier_report_duration_ns(report: *const ReportHandle) -> u64 {
    ffi_catch_panic!(0, {
        if report.is_null() {
            return 0;
        }

        let report = report as *const SafeHandle<ReportInner>;
        let report = unsafe { &*report };

        report.with(|inner| inner.duration_ns)
    })
}

// ============================================================================
// Memory Management Functions
// ============================================================================

/// Allocate memory through the verifier's allocator.
///
/// # Safety
///
/// - Returns NULL if allocation fails.
/// - The returned memory must be freed with `verifier_free()`.
#[no_mangle]
pub extern "C" fn verifier_alloc(size: usize) -> *mut c_void {
    ffi_catch_panic!(ptr::null_mut(), {
        if size == 0 {
            return ptr::null_mut();
        }

        let layout = match std::alloc::Layout::from_size_align(size, 8) {
            Ok(l) => l,
            Err(_) => return ptr::null_mut(),
        };

        unsafe { std::alloc::alloc(layout) as *mut c_void }
    })
}

/// Free memory allocated by verifier_alloc() or returned by verifier.
///
/// # Safety
///
/// - `ptr` must have been allocated by `verifier_alloc()` or be NULL.
#[no_mangle]
pub extern "C" fn verifier_free(ptr: *mut c_void) {
    ffi_catch_panic!((), {
        if ptr.is_null() {
            return;
        }

        // Note: This is a simplified implementation. In production, we would
        // need to track the allocation size or use a sized allocator.
        // For now, we rely on the system allocator's behavior.
        let _ = ptr;
    })
}

/// Free a string returned by the verifier.
///
/// # Safety
///
/// - `str` must have been returned by a verifier function or be NULL.
#[no_mangle]
pub extern "C" fn verifier_free_string(s: *mut c_char) {
    ffi_catch_panic!((), {
        if s.is_null() {
            return;
        }

        let _ = unsafe { CString::from_raw(s) };
    })
}

/// Free a VerifierResult and any associated resources.
///
/// # Safety
///
/// - `result` must point to a valid VerifierResult struct.
#[no_mangle]
pub extern "C" fn verifier_result_free(result: *mut VerifierResult) {
    ffi_catch_panic!((), {
        if result.is_null() {
            return;
        }

        let result = unsafe { &mut *result };

        match result.tag {
            VerifierResultTag::Ok => {
                // The ok pointer is a handle that should be freed separately
            }
            VerifierResultTag::Err => {
                let err = unsafe { &mut result.value.err };
                if !err.message.is_null() {
                    let _ = unsafe { CString::from_raw(err.message) };
                    err.message = ptr::null_mut();
                }
            }
        }
    })
}

/// Free an OperationResult and any associated resources.
///
/// # Safety
///
/// - `result` must point to a valid OperationResult struct.
#[no_mangle]
pub extern "C" fn verifier_operation_result_free(result: *mut OperationResult) {
    ffi_catch_panic!((), {
        if result.is_null() {
            return;
        }

        let result = unsafe { &mut *result };

        if !result.value_data.is_null() {
            // This should have been allocated with verifier_alloc
            // For safety, we treat it as a Vec
            let _ = unsafe {
                Vec::from_raw_parts(result.value_data, result.value_len, result.value_len)
            };
            result.value_data = ptr::null_mut();
            result.value_len = 0;
        }

        if !result.error_message.is_null() {
            let _ = unsafe { CString::from_raw(result.error_message) };
            result.error_message = ptr::null_mut();
        }
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let version = verifier_version();
        assert!(!version.is_null());
        let version_str = unsafe { CStr::from_ptr(version) };
        assert!(!version_str.to_str().unwrap().is_empty());
    }

    #[test]
    fn test_init_shutdown() {
        let result = verifier_init();
        assert_eq!(result.tag, VerifierResultTag::Ok);

        let handle = unsafe { result.value.ok } as *mut VerifierHandle;
        assert!(!handle.is_null());

        let code = verifier_shutdown(handle);
        assert_eq!(code, VerifierErrorCode::Ok);
    }

    #[test]
    fn test_null_safety() {
        // Shutdown with null should be safe
        let code = verifier_shutdown(ptr::null_mut());
        assert_eq!(code, VerifierErrorCode::Ok);

        // Test free with null should be safe
        let code = verifier_test_free(ptr::null_mut());
        assert_eq!(code, VerifierErrorCode::Ok);

        // Check result free with null should be safe
        let code = verifier_check_result_free(ptr::null_mut());
        assert_eq!(code, VerifierErrorCode::Ok);

        // Report free with null should be safe
        let code = verifier_report_free(ptr::null_mut());
        assert_eq!(code, VerifierErrorCode::Ok);
    }

    #[test]
    fn test_string_slice_conversion() {
        // Valid UTF-8
        let data = "hello";
        let slice = StringSlice {
            data: data.as_ptr() as *const c_char,
            len: data.len(),
        };
        assert_eq!(string_slice_to_str(&slice).unwrap(), "hello");

        // Empty string with null pointer
        let empty_slice = StringSlice {
            data: ptr::null(),
            len: 0,
        };
        assert_eq!(string_slice_to_str(&empty_slice).unwrap(), "");

        // Null pointer with non-zero length should error
        let bad_slice = StringSlice {
            data: ptr::null(),
            len: 5,
        };
        assert_eq!(
            string_slice_to_str(&bad_slice).unwrap_err(),
            VerifierErrorCode::NullPointer
        );
    }
}
