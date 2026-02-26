using System.Text.Json.Serialization;
using Orchestration.Core.Workflow.StateTypes;

namespace Orchestration.Core.Workflow;

/// <summary>
/// Root model for a JSON-configurable workflow definition.
/// </summary>
public sealed class WorkflowDefinition
{
    /// <summary>
    /// Unique identifier for this workflow definition.
    /// </summary>
    [JsonPropertyName("id")]
    public required string Id { get; init; }

    /// <summary>
    /// Version of this workflow definition.
    /// </summary>
    [JsonPropertyName("version")]
    public string Version { get; init; } = "1.0.0";

    /// <summary>
    /// Human-readable name for the workflow.
    /// </summary>
    [JsonPropertyName("name")]
    public required string Name { get; init; }

    /// <summary>
    /// Description of what the workflow does.
    /// </summary>
    [JsonPropertyName("description")]
    public string? Description { get; init; }

    /// <summary>
    /// Metadata about the workflow.
    /// </summary>
    [JsonPropertyName("metadata")]
    public WorkflowMetadata? Metadata { get; init; }

    /// <summary>
    /// Default configuration for the workflow.
    /// </summary>
    [JsonPropertyName("config")]
    public WorkflowConfiguration Config { get; init; } = new();

    /// <summary>
    /// The name of the first state to execute.
    /// </summary>
    [JsonPropertyName("startAt")]
    public required string StartAt { get; init; }

    /// <summary>
    /// Dictionary of state definitions.
    /// </summary>
    [JsonPropertyName("states")]
    public required Dictionary<string, WorkflowStateDefinition> States { get; init; }
}

/// <summary>
/// Metadata about a workflow definition.
/// </summary>
public sealed class WorkflowMetadata
{
    [JsonPropertyName("author")]
    public string? Author { get; init; }

    [JsonPropertyName("createdAt")]
    public DateTimeOffset? CreatedAt { get; init; }

    [JsonPropertyName("updatedAt")]
    public DateTimeOffset? UpdatedAt { get; init; }

    [JsonPropertyName("tags")]
    public List<string>? Tags { get; init; }
}

/// <summary>
/// Default configuration for a workflow.
/// </summary>
public sealed class WorkflowConfiguration
{
    /// <summary>
    /// Default timeout for the entire workflow.
    /// </summary>
    [JsonPropertyName("timeoutSeconds")]
    public int? TimeoutSeconds { get; init; }

    /// <summary>
    /// Default retry policy for activities.
    /// </summary>
    [JsonPropertyName("defaultRetryPolicy")]
    public RetryPolicy? DefaultRetryPolicy { get; init; }

    /// <summary>
    /// Name of the compensation state to execute on failure.
    /// </summary>
    [JsonPropertyName("compensationState")]
    public string? CompensationState { get; init; }
}

/// <summary>
/// Retry policy configuration.
/// </summary>
public sealed class RetryPolicy
{
    /// <summary>
    /// Maximum number of retry attempts.
    /// </summary>
    [JsonPropertyName("maxAttempts")]
    public int MaxAttempts { get; init; } = 3;

    /// <summary>
    /// Initial delay between retries in seconds.
    /// </summary>
    [JsonPropertyName("initialIntervalSeconds")]
    public int InitialIntervalSeconds { get; init; } = 1;

    /// <summary>
    /// Maximum delay between retries in seconds.
    /// </summary>
    [JsonPropertyName("maxIntervalSeconds")]
    public int MaxIntervalSeconds { get; init; } = 60;

    /// <summary>
    /// Backoff coefficient for exponential backoff.
    /// </summary>
    [JsonPropertyName("backoffCoefficient")]
    public double BackoffCoefficient { get; init; } = 2.0;

    /// <summary>
    /// Exception types that should trigger a retry.
    /// </summary>
    [JsonPropertyName("retryableExceptions")]
    public List<string>? RetryableExceptions { get; init; }
}
