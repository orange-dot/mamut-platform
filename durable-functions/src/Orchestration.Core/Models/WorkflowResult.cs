using System.Text.Json.Serialization;

namespace Orchestration.Core.Models;

/// <summary>
/// Result of a completed workflow orchestration.
/// </summary>
public sealed class WorkflowResult
{
    /// <summary>
    /// Whether the workflow completed successfully.
    /// </summary>
    [JsonPropertyName("success")]
    public bool Success { get; init; }

    /// <summary>
    /// The final output of the workflow.
    /// </summary>
    [JsonPropertyName("output")]
    public Dictionary<string, object?>? Output { get; init; }

    /// <summary>
    /// Error message if the workflow failed.
    /// </summary>
    [JsonPropertyName("error")]
    public string? Error { get; init; }

    /// <summary>
    /// Error code for programmatic error handling.
    /// </summary>
    [JsonPropertyName("errorCode")]
    public string? ErrorCode { get; init; }

    /// <summary>
    /// The final state of the workflow.
    /// </summary>
    [JsonPropertyName("state")]
    public WorkflowRuntimeState? State { get; init; }

    /// <summary>
    /// Whether compensation was executed.
    /// </summary>
    [JsonPropertyName("compensated")]
    public bool Compensated { get; init; }

    /// <summary>
    /// Creates a successful result.
    /// </summary>
    public static WorkflowResult Succeeded(Dictionary<string, object?>? output = null, WorkflowRuntimeState? state = null)
        => new() { Success = true, Output = output, State = state };

    /// <summary>
    /// Creates a failed result.
    /// </summary>
    public static WorkflowResult Failed(string error, string? errorCode = null, bool compensated = false)
        => new() { Success = false, Error = error, ErrorCode = errorCode, Compensated = compensated };
}
