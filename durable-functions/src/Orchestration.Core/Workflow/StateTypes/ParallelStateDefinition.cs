using System.Text.Json.Serialization;

namespace Orchestration.Core.Workflow.StateTypes;

/// <summary>
/// A state that executes multiple branches in parallel.
/// </summary>
public sealed class ParallelStateDefinition : WorkflowStateDefinition
{
    [JsonPropertyName("type")]
    public override string Type => "Parallel";

    /// <summary>
    /// Branches to execute in parallel.
    /// </summary>
    [JsonPropertyName("branches")]
    public required List<ParallelBranch> Branches { get; init; }

    /// <summary>
    /// Where to store the combined branch results in state.
    /// </summary>
    [JsonPropertyName("resultPath")]
    public string? ResultPath { get; init; }

    /// <summary>
    /// Error handlers for catching errors from any branch.
    /// </summary>
    [JsonPropertyName("catch")]
    public List<CatchDefinition>? Catch { get; init; }
}

/// <summary>
/// A single branch within a parallel state.
/// </summary>
public sealed class ParallelBranch
{
    /// <summary>
    /// Name of this branch for identification.
    /// </summary>
    [JsonPropertyName("name")]
    public required string Name { get; init; }

    /// <summary>
    /// The first state to execute in this branch.
    /// </summary>
    [JsonPropertyName("startAt")]
    public required string StartAt { get; init; }

    /// <summary>
    /// States within this branch.
    /// </summary>
    [JsonPropertyName("states")]
    public required Dictionary<string, WorkflowStateDefinition> States { get; init; }
}
