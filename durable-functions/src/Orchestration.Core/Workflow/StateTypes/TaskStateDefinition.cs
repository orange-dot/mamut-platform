using System.Text.Json.Serialization;

namespace Orchestration.Core.Workflow.StateTypes;

/// <summary>
/// A state that executes an activity function.
/// </summary>
public sealed class TaskStateDefinition : WorkflowStateDefinition
{
    [JsonPropertyName("type")]
    public override string Type => "Task";

    /// <summary>
    /// The name of the activity to execute.
    /// </summary>
    [JsonPropertyName("activity")]
    public required string Activity { get; init; }

    /// <summary>
    /// JSONPath expression or object to pass as input to the activity.
    /// </summary>
    [JsonPropertyName("input")]
    public object? Input { get; init; }

    /// <summary>
    /// JSONPath expression for where to store the activity output in state.
    /// </summary>
    [JsonPropertyName("resultPath")]
    public string? ResultPath { get; init; }

    /// <summary>
    /// Retry policy for this specific task.
    /// </summary>
    [JsonPropertyName("retry")]
    public RetryPolicy? Retry { get; init; }

    /// <summary>
    /// Error handlers for catching specific errors.
    /// </summary>
    [JsonPropertyName("catch")]
    public List<CatchDefinition>? Catch { get; init; }

    /// <summary>
    /// Timeout for this task in seconds.
    /// </summary>
    [JsonPropertyName("timeoutSeconds")]
    public int? TimeoutSeconds { get; init; }

    /// <summary>
    /// Name of the compensating activity to call on rollback.
    /// </summary>
    [JsonPropertyName("compensateWith")]
    public string? CompensateWith { get; init; }
}

/// <summary>
/// Error handler definition for catching errors.
/// </summary>
public sealed class CatchDefinition
{
    /// <summary>
    /// Error types to catch (e.g., "States.TaskFailed", "CustomError").
    /// </summary>
    [JsonPropertyName("errors")]
    public required List<string> Errors { get; init; }

    /// <summary>
    /// Where to store the error information in state.
    /// </summary>
    [JsonPropertyName("resultPath")]
    public string? ResultPath { get; init; }

    /// <summary>
    /// State to transition to when this error is caught.
    /// </summary>
    [JsonPropertyName("next")]
    public required string Next { get; init; }
}
