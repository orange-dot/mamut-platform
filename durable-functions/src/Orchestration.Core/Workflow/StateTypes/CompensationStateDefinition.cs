using System.Text.Json.Serialization;

namespace Orchestration.Core.Workflow.StateTypes;

/// <summary>
/// A state that executes compensation/rollback activities.
/// </summary>
public sealed class CompensationStateDefinition : WorkflowStateDefinition
{
    [JsonPropertyName("type")]
    public override string Type => "Compensation";

    /// <summary>
    /// Ordered list of compensation steps to execute.
    /// </summary>
    [JsonPropertyName("steps")]
    public required List<CompensationStep> Steps { get; init; }

    /// <summary>
    /// Whether to continue despite individual step failures.
    /// </summary>
    [JsonPropertyName("continueOnError")]
    public bool ContinueOnError { get; init; } = true;

    /// <summary>
    /// State to transition to after compensation completes.
    /// </summary>
    [JsonPropertyName("finalState")]
    public string? FinalState { get; init; }
}

/// <summary>
/// A single compensation step.
/// </summary>
public sealed class CompensationStep
{
    /// <summary>
    /// Name of this compensation step.
    /// </summary>
    [JsonPropertyName("name")]
    public required string Name { get; init; }

    /// <summary>
    /// The activity to execute for compensation.
    /// </summary>
    [JsonPropertyName("activity")]
    public required string Activity { get; init; }

    /// <summary>
    /// Input for the compensation activity.
    /// </summary>
    [JsonPropertyName("input")]
    public object? Input { get; init; }

    /// <summary>
    /// Condition that must be true for this step to execute.
    /// </summary>
    [JsonPropertyName("condition")]
    public string? Condition { get; init; }
}
