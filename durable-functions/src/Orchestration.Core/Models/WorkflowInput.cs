using System.Text.Json.Serialization;

namespace Orchestration.Core.Models;

/// <summary>
/// Input data for starting a workflow orchestration.
/// </summary>
public sealed class WorkflowInput
{
    /// <summary>
    /// The type of workflow to execute.
    /// </summary>
    [JsonPropertyName("workflowType")]
    public required string WorkflowType { get; init; }

    /// <summary>
    /// Optional version of the workflow definition to use.
    /// </summary>
    [JsonPropertyName("version")]
    public string? Version { get; init; }

    /// <summary>
    /// The entity ID this workflow is processing.
    /// </summary>
    [JsonPropertyName("entityId")]
    public required string EntityId { get; init; }

    /// <summary>
    /// Initial input data for the workflow.
    /// </summary>
    [JsonPropertyName("data")]
    public Dictionary<string, object?>? Data { get; init; }

    /// <summary>
    /// Correlation ID for distributed tracing.
    /// </summary>
    [JsonPropertyName("correlationId")]
    public string? CorrelationId { get; init; }

    /// <summary>
    /// Idempotency key to prevent duplicate workflow executions.
    /// </summary>
    [JsonPropertyName("idempotencyKey")]
    public string? IdempotencyKey { get; init; }
}
