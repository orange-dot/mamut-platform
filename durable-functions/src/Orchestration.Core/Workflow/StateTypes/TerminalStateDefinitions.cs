using System.Text.Json.Serialization;

namespace Orchestration.Core.Workflow.StateTypes;

/// <summary>
/// A terminal state indicating successful completion.
/// </summary>
public sealed class SucceedStateDefinition : WorkflowStateDefinition
{
    [JsonPropertyName("type")]
    public override string Type => "Succeed";

    /// <summary>
    /// JSONPath or object defining the output of the workflow.
    /// </summary>
    [JsonPropertyName("output")]
    public object? Output { get; init; }
}

/// <summary>
/// A terminal state indicating failure.
/// </summary>
public sealed class FailStateDefinition : WorkflowStateDefinition
{
    [JsonPropertyName("type")]
    public override string Type => "Fail";

    /// <summary>
    /// Error code to return.
    /// </summary>
    [JsonPropertyName("error")]
    public required string Error { get; init; }

    /// <summary>
    /// Human-readable error message.
    /// </summary>
    [JsonPropertyName("cause")]
    public string? Cause { get; init; }
}
