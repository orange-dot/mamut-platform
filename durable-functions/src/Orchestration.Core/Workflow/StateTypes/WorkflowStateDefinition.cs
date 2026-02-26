using System.Text.Json;
using System.Text.Json.Serialization;

namespace Orchestration.Core.Workflow.StateTypes;

/// <summary>
/// Base class for all workflow state definitions.
/// </summary>
[JsonPolymorphic(TypeDiscriminatorPropertyName = "type")]
[JsonDerivedType(typeof(TaskStateDefinition), "Task")]
[JsonDerivedType(typeof(WaitStateDefinition), "Wait")]
[JsonDerivedType(typeof(ChoiceStateDefinition), "Choice")]
[JsonDerivedType(typeof(ParallelStateDefinition), "Parallel")]
[JsonDerivedType(typeof(CompensationStateDefinition), "Compensation")]
[JsonDerivedType(typeof(SucceedStateDefinition), "Succeed")]
[JsonDerivedType(typeof(FailStateDefinition), "Fail")]
public abstract class WorkflowStateDefinition
{
    /// <summary>
    /// The type of this state.
    /// </summary>
    [JsonPropertyName("type")]
    public abstract string Type { get; }

    /// <summary>
    /// Optional comment/description for this state.
    /// </summary>
    [JsonPropertyName("comment")]
    public string? Comment { get; init; }

    /// <summary>
    /// Whether this is an end state.
    /// </summary>
    [JsonPropertyName("end")]
    public bool End { get; init; }

    /// <summary>
    /// Name of the next state to transition to.
    /// </summary>
    [JsonPropertyName("next")]
    public string? Next { get; init; }
}
