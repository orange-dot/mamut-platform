using Orchestration.Core.Models;

namespace Orchestration.Core.Workflow.Interpreter;

/// <summary>
/// Interprets and executes workflow state definitions.
/// </summary>
public interface IWorkflowInterpreter
{
    /// <summary>
    /// Executes a single step in the workflow.
    /// </summary>
    /// <param name="context">Execution context providing access to activities and timers.</param>
    /// <param name="definition">The workflow definition.</param>
    /// <param name="currentStep">Name of the current step to execute.</param>
    /// <param name="state">Current workflow runtime state.</param>
    /// <returns>Name of the next step to execute, or null if workflow is complete.</returns>
    Task<string?> ExecuteStepAsync(
        IWorkflowExecutionContext context,
        WorkflowDefinition definition,
        string currentStep,
        WorkflowRuntimeState state);
}

/// <summary>
/// Execution context for workflow steps, abstracting Durable Functions primitives.
/// </summary>
public interface IWorkflowExecutionContext
{
    /// <summary>
    /// Calls an activity function.
    /// </summary>
    Task<TResult> CallActivityAsync<TResult>(string activityName, object? input, RetryPolicy? retry = null);

    /// <summary>
    /// Creates a timer that completes after the specified duration.
    /// </summary>
    Task CreateTimerAsync(TimeSpan duration);

    /// <summary>
    /// Creates a timer that completes at the specified time.
    /// </summary>
    Task CreateTimerAsync(DateTimeOffset fireAt);

    /// <summary>
    /// Waits for an external event.
    /// </summary>
    Task<TResult> WaitForExternalEventAsync<TResult>(string eventName, TimeSpan? timeout = null);

    /// <summary>
    /// Executes multiple tasks in parallel.
    /// </summary>
    Task<TResult[]> WhenAllAsync<TResult>(IEnumerable<Task<TResult>> tasks);

    /// <summary>
    /// Gets the current UTC time (deterministic).
    /// </summary>
    DateTimeOffset CurrentUtcDateTime { get; }

    /// <summary>
    /// Gets the orchestration instance ID.
    /// </summary>
    string InstanceId { get; }

    /// <summary>
    /// Generates a new GUID (deterministic).
    /// </summary>
    Guid NewGuid();
}
