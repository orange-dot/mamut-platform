/**
 * @file verifier.h
 * @brief C FFI header for the MAMUT verification harness
 *
 * This header defines the C interface for the verification harness, enabling
 * integration with C/C++ codebases and other languages via FFI.
 *
 * Memory Ownership Rules:
 * - Handle types (VerifierHandle, TestHandle, etc.) are owned by the caller
 *   and must be explicitly freed using their corresponding _free() functions.
 * - StringSlice and ByteSlice are borrowed references; the data they point to
 *   must remain valid for the duration of the call.
 * - Functions returning heap-allocated strings transfer ownership to the caller;
 *   use verifier_free_string() to deallocate.
 * - VerifierResult unions may contain allocated data; use verifier_result_free().
 *
 * Thread Safety:
 * - VerifierHandle operations are thread-safe.
 * - TestHandle operations are NOT thread-safe; each test should be used from
 *   a single thread or protected by external synchronization.
 *
 * @copyright (c) 2025 MAMUT Project
 */

#ifndef MAMUT_VERIFIER_H
#define MAMUT_VERIFIER_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * Opaque Handle Types
 * ============================================================================
 * These handles represent owned resources. Each must be freed using its
 * corresponding _free() function when no longer needed.
 */

/**
 * @brief Opaque handle to the verification engine.
 *
 * Represents the main verifier instance. Create with verifier_init(),
 * destroy with verifier_shutdown(). Thread-safe for all operations.
 */
typedef struct VerifierHandle VerifierHandle;

/**
 * @brief Opaque handle to a test execution context.
 *
 * Represents an individual test case. Create with verifier_create_test(),
 * destroy with verifier_test_free(). NOT thread-safe.
 */
typedef struct TestHandle TestHandle;

/**
 * @brief Opaque handle to a check result.
 *
 * Contains the outcome of a verification check. Obtain from verification
 * operations, destroy with verifier_check_result_free().
 */
typedef struct CheckResultHandle CheckResultHandle;

/**
 * @brief Opaque handle to a verification report.
 *
 * Aggregated results from a test run. Obtain from verifier_finalize_test(),
 * destroy with verifier_report_free().
 */
typedef struct ReportHandle ReportHandle;

/* ============================================================================
 * Slice Types (Borrowed References)
 * ============================================================================
 * These types represent borrowed views into data. The underlying data must
 * remain valid for the duration of the function call.
 */

/**
 * @brief A borrowed, non-null-terminated string slice.
 *
 * The data pointer may be NULL only if len is 0. The string is NOT
 * null-terminated; use len for bounds.
 */
typedef struct StringSlice {
    /** Pointer to UTF-8 encoded string data. May be NULL if len == 0. */
    const char* data;
    /** Length in bytes (not characters). */
    size_t len;
} StringSlice;

/**
 * @brief A borrowed byte slice.
 *
 * The data pointer may be NULL only if len is 0.
 */
typedef struct ByteSlice {
    /** Pointer to byte data. May be NULL if len == 0. */
    const uint8_t* data;
    /** Length in bytes. */
    size_t len;
} ByteSlice;

/* ============================================================================
 * Error Types
 * ============================================================================
 */

/**
 * @brief Error codes returned by verifier operations.
 */
typedef enum VerifierErrorCode {
    /** Operation completed successfully. */
    VERIFIER_OK = 0,

    /** A null pointer was passed where a valid pointer was required. */
    VERIFIER_ERROR_NULL_POINTER = 1,

    /** The provided handle is invalid or has been freed. */
    VERIFIER_ERROR_INVALID_HANDLE = 2,

    /** A string argument contained invalid UTF-8. */
    VERIFIER_ERROR_INVALID_UTF8 = 3,

    /** The requested operation is not valid in the current state. */
    VERIFIER_ERROR_INVALID_STATE = 4,

    /** Memory allocation failed. */
    VERIFIER_ERROR_OUT_OF_MEMORY = 5,

    /** An I/O error occurred. */
    VERIFIER_ERROR_IO = 6,

    /** A timeout occurred during the operation. */
    VERIFIER_ERROR_TIMEOUT = 7,

    /** The operation was cancelled. */
    VERIFIER_ERROR_CANCELLED = 8,

    /** A verification invariant was violated. */
    VERIFIER_ERROR_INVARIANT_VIOLATION = 9,

    /** An internal error occurred. Check logs for details. */
    VERIFIER_ERROR_INTERNAL = 10,

    /** The configuration is invalid. */
    VERIFIER_ERROR_INVALID_CONFIG = 11,

    /** The operation failed due to an injected fault. */
    VERIFIER_ERROR_FAULT_INJECTED = 12,

    /** An unknown or unspecified error occurred. */
    VERIFIER_ERROR_UNKNOWN = 255,
} VerifierErrorCode;

/**
 * @brief Extended error information.
 *
 * Contains an error code and optional message. The message pointer is
 * owned by the VerifierResult and will be freed with it.
 */
typedef struct VerifierError {
    /** The error code. */
    VerifierErrorCode code;
    /** Human-readable error message. Owned, may be NULL. */
    char* message;
} VerifierError;

/**
 * @brief Result type tag for VerifierResult discriminated union.
 */
typedef enum VerifierResultTag {
    /** The result contains a success value. */
    VERIFIER_RESULT_OK = 0,
    /** The result contains an error. */
    VERIFIER_RESULT_ERR = 1,
} VerifierResultTag;

/**
 * @brief Discriminated union for operation results.
 *
 * Check the tag field to determine which union member is valid.
 * Must be freed with verifier_result_free() when no longer needed.
 */
typedef struct VerifierResult {
    /** Discriminant indicating which union field is valid. */
    VerifierResultTag tag;
    union {
        /** Valid when tag == VERIFIER_RESULT_OK. Contains a handle or NULL. */
        void* ok;
        /** Valid when tag == VERIFIER_RESULT_ERR. Contains error details. */
        VerifierError err;
    } value;
} VerifierResult;

/* ============================================================================
 * Configuration Types
 * ============================================================================
 */

/**
 * @brief Execution mode for tests.
 */
typedef enum ExecutionMode {
    /** Execute operations sequentially. */
    EXECUTION_MODE_SEQUENTIAL = 0,
    /** Execute operations with controlled concurrency. */
    EXECUTION_MODE_CONCURRENT = 1,
    /** Execute with deterministic scheduling for reproducibility. */
    EXECUTION_MODE_DETERMINISTIC = 2,
} ExecutionMode;

/**
 * @brief Configuration for creating a new test.
 */
typedef struct TestConfig {
    /** Test name. Borrowed, must remain valid during verifier_create_test(). */
    StringSlice name;

    /** Optional test description. Borrowed. */
    StringSlice description;

    /** Maximum number of operations to execute. 0 for unlimited. */
    uint64_t max_operations;

    /** Timeout in milliseconds. 0 for no timeout. */
    uint64_t timeout_ms;

    /** Random seed for deterministic execution. 0 for random. */
    uint64_t seed;

    /** Execution mode. */
    ExecutionMode mode;

    /** Enable verbose logging. */
    bool verbose;

    /** Enable invariant checking after each operation. */
    bool check_invariants;

    /** Enable fault injection. */
    bool enable_fault_injection;
} TestConfig;

/* ============================================================================
 * Operation Types
 * ============================================================================
 */

/**
 * @brief Type of operation being performed.
 */
typedef enum OperationType {
    /** Read operation. */
    OPERATION_TYPE_READ = 0,
    /** Write operation. */
    OPERATION_TYPE_WRITE = 1,
    /** Delete operation. */
    OPERATION_TYPE_DELETE = 2,
    /** Transaction begin. */
    OPERATION_TYPE_TXN_BEGIN = 3,
    /** Transaction commit. */
    OPERATION_TYPE_TXN_COMMIT = 4,
    /** Transaction abort. */
    OPERATION_TYPE_TXN_ABORT = 5,
    /** Barrier/fence operation. */
    OPERATION_TYPE_BARRIER = 6,
    /** Custom operation type. */
    OPERATION_TYPE_CUSTOM = 255,
} OperationType;

/**
 * @brief An operation to be executed by the system under test.
 */
typedef struct Operation {
    /** Unique operation ID within the test. */
    uint64_t id;

    /** Type of operation. */
    OperationType op_type;

    /** Key for the operation. Borrowed. */
    ByteSlice key;

    /** Value for write operations. Borrowed. May be empty for reads/deletes. */
    ByteSlice value;

    /** Custom operation name for OPERATION_TYPE_CUSTOM. Borrowed. */
    StringSlice custom_name;

    /** Opaque user data pointer. Not accessed by the verifier. */
    void* user_data;
} Operation;

/**
 * @brief Status of an operation result.
 */
typedef enum OperationStatus {
    /** Operation succeeded. */
    OPERATION_STATUS_SUCCESS = 0,
    /** Operation failed with an error. */
    OPERATION_STATUS_FAILURE = 1,
    /** Operation was skipped. */
    OPERATION_STATUS_SKIPPED = 2,
    /** Operation timed out. */
    OPERATION_STATUS_TIMEOUT = 3,
    /** Operation was aborted due to fault injection. */
    OPERATION_STATUS_FAULT_INJECTED = 4,
} OperationStatus;

/**
 * @brief Result of executing an operation.
 */
typedef struct OperationResult {
    /** The operation ID this result corresponds to. */
    uint64_t operation_id;

    /** Status of the operation. */
    OperationStatus status;

    /** Returned value for read operations. Owned, must be freed. */
    uint8_t* value_data;
    /** Length of the returned value. */
    size_t value_len;

    /** Error message if status is FAILURE. Owned, must be freed. */
    char* error_message;

    /** Duration in nanoseconds. */
    uint64_t duration_ns;
} OperationResult;

/* ============================================================================
 * Fault Injection Types
 * ============================================================================
 */

/**
 * @brief Type of fault to inject.
 */
typedef enum FaultType {
    /** No fault. */
    FAULT_TYPE_NONE = 0,
    /** Simulate a crash/process termination. */
    FAULT_TYPE_CRASH = 1,
    /** Simulate a network partition. */
    FAULT_TYPE_NETWORK_PARTITION = 2,
    /** Simulate disk I/O errors. */
    FAULT_TYPE_DISK_ERROR = 3,
    /** Simulate memory allocation failure. */
    FAULT_TYPE_OOM = 4,
    /** Simulate CPU contention/slowdown. */
    FAULT_TYPE_SLOW_CPU = 5,
    /** Simulate clock skew. */
    FAULT_TYPE_CLOCK_SKEW = 6,
    /** Corrupt data in transit or at rest. */
    FAULT_TYPE_DATA_CORRUPTION = 7,
    /** Drop messages/packets. */
    FAULT_TYPE_MESSAGE_DROP = 8,
    /** Reorder messages/operations. */
    FAULT_TYPE_MESSAGE_REORDER = 9,
    /** Duplicate messages/operations. */
    FAULT_TYPE_MESSAGE_DUPLICATE = 10,
    /** Delay messages/operations. */
    FAULT_TYPE_DELAY = 11,
    /** Custom fault type. */
    FAULT_TYPE_CUSTOM = 255,
} FaultType;

/**
 * @brief Target for fault injection.
 */
typedef enum FaultTarget {
    /** Target all components. */
    FAULT_TARGET_ALL = 0,
    /** Target storage layer. */
    FAULT_TARGET_STORAGE = 1,
    /** Target network layer. */
    FAULT_TARGET_NETWORK = 2,
    /** Target compute layer. */
    FAULT_TARGET_COMPUTE = 3,
    /** Target memory subsystem. */
    FAULT_TARGET_MEMORY = 4,
    /** Target specific node by ID. */
    FAULT_TARGET_NODE = 5,
    /** Target specific operation by ID. */
    FAULT_TARGET_OPERATION = 6,
} FaultTarget;

/**
 * @brief Configuration for a fault injection.
 */
typedef struct FaultConfig {
    /** Type of fault to inject. */
    FaultType fault_type;

    /** Target of the fault. */
    FaultTarget target;

    /** Target ID (node ID, operation ID, etc.) when target is specific. */
    uint64_t target_id;

    /** Probability of fault occurring (0.0 - 1.0). */
    double probability;

    /** Duration of the fault in milliseconds. 0 for instantaneous. */
    uint64_t duration_ms;

    /** Additional fault-specific parameters. Borrowed. */
    ByteSlice params;
} FaultConfig;

/**
 * @brief Information about an injected fault.
 */
typedef struct FaultInfo {
    /** Unique fault ID. */
    uint64_t id;

    /** Type of fault that was injected. */
    FaultType fault_type;

    /** Target of the fault. */
    FaultTarget target;

    /** When the fault was injected (Unix timestamp ms). */
    uint64_t injected_at_ms;

    /** When the fault will clear (Unix timestamp ms). 0 if permanent. */
    uint64_t clear_at_ms;

    /** Whether the fault is currently active. */
    bool active;
} FaultInfo;

/* ============================================================================
 * Callback Types
 * ============================================================================
 */

/**
 * @brief Callback invoked when an operation is about to be executed.
 *
 * @param user_data Opaque pointer passed to the registration function.
 * @param operation The operation about to be executed.
 * @return true to allow the operation, false to skip it.
 */
typedef bool (*OperationCallback)(void* user_data, const Operation* operation);

/**
 * @brief Callback to execute an operation on the system under test.
 *
 * This callback is invoked by the verifier to perform the actual operation.
 * The implementation should execute the operation and fill in the result.
 *
 * @param user_data Opaque pointer passed to the registration function.
 * @param operation The operation to execute.
 * @param result Output parameter for the operation result. The callback
 *               is responsible for allocating value_data and error_message
 *               using verifier_alloc().
 * @return VERIFIER_OK on success, error code on failure.
 */
typedef VerifierErrorCode (*ExecuteOperationCallback)(
    void* user_data,
    const Operation* operation,
    OperationResult* result
);

/**
 * @brief Callback invoked when a fault is injected.
 *
 * @param user_data Opaque pointer passed to the registration function.
 * @param fault Information about the injected fault.
 */
typedef void (*FaultInjectedCallback)(void* user_data, const FaultInfo* fault);

/**
 * @brief Callback invoked when a fault is cleared.
 *
 * @param user_data Opaque pointer passed to the registration function.
 * @param fault Information about the cleared fault.
 */
typedef void (*FaultClearedCallback)(void* user_data, const FaultInfo* fault);

/**
 * @brief Callback for progress reporting.
 *
 * @param user_data Opaque pointer passed to the registration function.
 * @param completed Number of operations completed.
 * @param total Total number of operations (0 if unknown).
 */
typedef void (*ProgressCallback)(void* user_data, uint64_t completed, uint64_t total);

/* ============================================================================
 * Verifier Lifecycle Functions
 * ============================================================================
 */

/**
 * @brief Initialize the verification engine.
 *
 * This must be called before any other verifier functions. The returned
 * handle must be freed with verifier_shutdown().
 *
 * @return A VerifierResult containing either a VerifierHandle* (on success)
 *         or a VerifierError (on failure).
 */
VerifierResult verifier_init(void);

/**
 * @brief Initialize the verification engine with a configuration file.
 *
 * @param config_path Path to configuration file. Borrowed.
 * @return A VerifierResult containing either a VerifierHandle* or VerifierError.
 */
VerifierResult verifier_init_with_config(StringSlice config_path);

/**
 * @brief Shut down the verification engine and free all resources.
 *
 * This invalidates the handle and all associated test handles.
 *
 * @param handle The verifier handle to shut down. May be NULL (no-op).
 * @return VERIFIER_OK on success, error code on failure.
 */
VerifierErrorCode verifier_shutdown(VerifierHandle* handle);

/**
 * @brief Get the version string of the verifier library.
 *
 * @return A null-terminated version string. Statically allocated, do not free.
 */
const char* verifier_version(void);

/* ============================================================================
 * Test Management Functions
 * ============================================================================
 */

/**
 * @brief Create a new test context.
 *
 * @param verifier The verifier handle.
 * @param config Test configuration. Borrowed.
 * @return A VerifierResult containing either a TestHandle* or VerifierError.
 */
VerifierResult verifier_create_test(
    VerifierHandle* verifier,
    const TestConfig* config
);

/**
 * @brief Free a test handle and its resources.
 *
 * @param test The test handle to free. May be NULL (no-op).
 * @return VERIFIER_OK on success, error code on failure.
 */
VerifierErrorCode verifier_test_free(TestHandle* test);

/**
 * @brief Register the operation execution callback.
 *
 * This callback will be invoked to execute operations on the system under test.
 *
 * @param test The test handle.
 * @param callback The callback function.
 * @param user_data Opaque pointer passed to the callback.
 * @return VERIFIER_OK on success, error code on failure.
 */
VerifierErrorCode verifier_register_executor(
    TestHandle* test,
    ExecuteOperationCallback callback,
    void* user_data
);

/**
 * @brief Register a pre-operation callback.
 *
 * @param test The test handle.
 * @param callback The callback function.
 * @param user_data Opaque pointer passed to the callback.
 * @return VERIFIER_OK on success, error code on failure.
 */
VerifierErrorCode verifier_register_pre_operation(
    TestHandle* test,
    OperationCallback callback,
    void* user_data
);

/**
 * @brief Register a progress callback.
 *
 * @param test The test handle.
 * @param callback The callback function.
 * @param user_data Opaque pointer passed to the callback.
 * @return VERIFIER_OK on success, error code on failure.
 */
VerifierErrorCode verifier_register_progress(
    TestHandle* test,
    ProgressCallback callback,
    void* user_data
);

/* ============================================================================
 * Operation Functions
 * ============================================================================
 */

/**
 * @brief Add an operation to the test queue.
 *
 * @param test The test handle.
 * @param operation The operation to add. Borrowed.
 * @return The operation ID on success (>= 0), or -1 on failure.
 */
int64_t verifier_add_operation(TestHandle* test, const Operation* operation);

/**
 * @brief Generate random operations based on a schema.
 *
 * @param test The test handle.
 * @param count Number of operations to generate.
 * @param schema JSON schema describing operation generation rules. Borrowed.
 * @return VERIFIER_OK on success, error code on failure.
 */
VerifierErrorCode verifier_generate_operations(
    TestHandle* test,
    uint64_t count,
    StringSlice schema
);

/**
 * @brief Run the test, executing all queued operations.
 *
 * @param test The test handle.
 * @return VERIFIER_OK on success, error code on failure.
 */
VerifierErrorCode verifier_run_test(TestHandle* test);

/**
 * @brief Cancel a running test.
 *
 * @param test The test handle.
 * @return VERIFIER_OK on success, error code on failure.
 */
VerifierErrorCode verifier_cancel_test(TestHandle* test);

/* ============================================================================
 * Fault Injection Functions
 * ============================================================================
 */

/**
 * @brief Inject a fault into the test.
 *
 * @param test The test handle.
 * @param config Fault configuration. Borrowed.
 * @return The fault ID on success (>= 0), or -1 on failure.
 */
int64_t verifier_inject_fault(TestHandle* test, const FaultConfig* config);

/**
 * @brief Clear a previously injected fault.
 *
 * @param test The test handle.
 * @param fault_id The ID of the fault to clear.
 * @return VERIFIER_OK on success, error code on failure.
 */
VerifierErrorCode verifier_clear_fault(TestHandle* test, uint64_t fault_id);

/**
 * @brief Clear all active faults.
 *
 * @param test The test handle.
 * @return VERIFIER_OK on success, error code on failure.
 */
VerifierErrorCode verifier_clear_all_faults(TestHandle* test);

/**
 * @brief Register a fault injection callback.
 *
 * @param test The test handle.
 * @param on_inject Callback when a fault is injected. May be NULL.
 * @param on_clear Callback when a fault is cleared. May be NULL.
 * @param user_data Opaque pointer passed to the callbacks.
 * @return VERIFIER_OK on success, error code on failure.
 */
VerifierErrorCode verifier_register_fault_callbacks(
    TestHandle* test,
    FaultInjectedCallback on_inject,
    FaultClearedCallback on_clear,
    void* user_data
);

/* ============================================================================
 * Verification Functions
 * ============================================================================
 */

/**
 * @brief Check a linearizability property.
 *
 * @param test The test handle.
 * @return A VerifierResult containing either a CheckResultHandle* or VerifierError.
 */
VerifierResult verifier_check_linearizability(TestHandle* test);

/**
 * @brief Check a serializability property.
 *
 * @param test The test handle.
 * @return A VerifierResult containing either a CheckResultHandle* or VerifierError.
 */
VerifierResult verifier_check_serializability(TestHandle* test);

/**
 * @brief Check a custom invariant.
 *
 * @param test The test handle.
 * @param invariant_name Name of the invariant. Borrowed.
 * @param invariant_expr Expression defining the invariant. Borrowed.
 * @return A VerifierResult containing either a CheckResultHandle* or VerifierError.
 */
VerifierResult verifier_check_invariant(
    TestHandle* test,
    StringSlice invariant_name,
    StringSlice invariant_expr
);

/**
 * @brief Free a check result handle.
 *
 * @param result The check result to free. May be NULL (no-op).
 * @return VERIFIER_OK on success, error code on failure.
 */
VerifierErrorCode verifier_check_result_free(CheckResultHandle* result);

/**
 * @brief Get whether a check passed.
 *
 * @param result The check result handle.
 * @return true if the check passed, false otherwise.
 */
bool verifier_check_result_passed(const CheckResultHandle* result);

/**
 * @brief Get the check result message.
 *
 * @param result The check result handle.
 * @return A null-terminated message string. Borrowed from the result,
 *         valid until verifier_check_result_free() is called.
 */
const char* verifier_check_result_message(const CheckResultHandle* result);

/* ============================================================================
 * Report Functions
 * ============================================================================
 */

/**
 * @brief Finalize the test and generate a report.
 *
 * @param test The test handle.
 * @return A VerifierResult containing either a ReportHandle* or VerifierError.
 */
VerifierResult verifier_finalize_test(TestHandle* test);

/**
 * @brief Free a report handle.
 *
 * @param report The report to free. May be NULL (no-op).
 * @return VERIFIER_OK on success, error code on failure.
 */
VerifierErrorCode verifier_report_free(ReportHandle* report);

/**
 * @brief Get the report as a JSON string.
 *
 * @param report The report handle.
 * @return A null-terminated JSON string. Owned by caller, free with
 *         verifier_free_string().
 */
char* verifier_report_to_json(const ReportHandle* report);

/**
 * @brief Get the report as a human-readable string.
 *
 * @param report The report handle.
 * @return A null-terminated string. Owned by caller, free with
 *         verifier_free_string().
 */
char* verifier_report_to_string(const ReportHandle* report);

/**
 * @brief Get the total number of operations executed.
 *
 * @param report The report handle.
 * @return The number of operations.
 */
uint64_t verifier_report_operation_count(const ReportHandle* report);

/**
 * @brief Get the number of successful operations.
 *
 * @param report The report handle.
 * @return The number of successful operations.
 */
uint64_t verifier_report_success_count(const ReportHandle* report);

/**
 * @brief Get the number of failed operations.
 *
 * @param report The report handle.
 * @return The number of failed operations.
 */
uint64_t verifier_report_failure_count(const ReportHandle* report);

/**
 * @brief Get the total test duration in nanoseconds.
 *
 * @param report The report handle.
 * @return The duration in nanoseconds.
 */
uint64_t verifier_report_duration_ns(const ReportHandle* report);

/* ============================================================================
 * Memory Management Functions
 * ============================================================================
 */

/**
 * @brief Allocate memory through the verifier's allocator.
 *
 * Use this when allocating data to be returned through callbacks.
 *
 * @param size Number of bytes to allocate.
 * @return Pointer to allocated memory, or NULL on failure.
 */
void* verifier_alloc(size_t size);

/**
 * @brief Free memory allocated by verifier_alloc() or returned by verifier.
 *
 * @param ptr Pointer to free. May be NULL (no-op).
 */
void verifier_free(void* ptr);

/**
 * @brief Free a string returned by the verifier.
 *
 * Equivalent to verifier_free() but semantically clearer for strings.
 *
 * @param str String to free. May be NULL (no-op).
 */
void verifier_free_string(char* str);

/**
 * @brief Free a VerifierResult and any associated resources.
 *
 * @param result Pointer to the result to free. The result struct itself
 *               is not freed (assumed to be stack-allocated).
 */
void verifier_result_free(VerifierResult* result);

/**
 * @brief Free an OperationResult and any associated resources.
 *
 * Frees value_data and error_message if non-NULL.
 *
 * @param result Pointer to the result to free. The result struct itself
 *               is not freed (assumed to be stack-allocated).
 */
void verifier_operation_result_free(OperationResult* result);

/* ============================================================================
 * Utility Functions
 * ============================================================================
 */

/**
 * @brief Create a StringSlice from a null-terminated C string.
 *
 * @param str Null-terminated C string. May be NULL.
 * @return A StringSlice pointing to the string data.
 */
static inline StringSlice verifier_string_from_cstr(const char* str) {
    StringSlice slice;
    if (str) {
        slice.data = str;
        size_t len = 0;
        while (str[len]) len++;
        slice.len = len;
    } else {
        slice.data = NULL;
        slice.len = 0;
    }
    return slice;
}

/**
 * @brief Create a ByteSlice from a pointer and length.
 *
 * @param data Pointer to data.
 * @param len Length in bytes.
 * @return A ByteSlice pointing to the data.
 */
static inline ByteSlice verifier_bytes_from_ptr(const uint8_t* data, size_t len) {
    ByteSlice slice;
    slice.data = data;
    slice.len = len;
    return slice;
}

/**
 * @brief Check if a VerifierResult indicates success.
 *
 * @param result Pointer to the result to check.
 * @return true if the result is OK, false if it's an error.
 */
static inline bool verifier_result_is_ok(const VerifierResult* result) {
    return result && result->tag == VERIFIER_RESULT_OK;
}

/**
 * @brief Get the error code from a VerifierResult.
 *
 * @param result Pointer to the result.
 * @return The error code, or VERIFIER_OK if the result is successful.
 */
static inline VerifierErrorCode verifier_result_error_code(const VerifierResult* result) {
    if (!result) return VERIFIER_ERROR_NULL_POINTER;
    if (result->tag == VERIFIER_RESULT_OK) return VERIFIER_OK;
    return result->value.err.code;
}

#ifdef __cplusplus
}
#endif

#endif /* MAMUT_VERIFIER_H */
