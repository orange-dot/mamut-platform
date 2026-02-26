using System.Text.Json.Serialization;

namespace Orchestration.Core.Workflow.StateTypes;

/// <summary>
/// A state that makes branching decisions based on conditions.
/// </summary>
public sealed class ChoiceStateDefinition : WorkflowStateDefinition
{
    [JsonPropertyName("type")]
    public override string Type => "Choice";

    /// <summary>
    /// List of choice rules to evaluate.
    /// </summary>
    [JsonPropertyName("choices")]
    public required List<ChoiceRule> Choices { get; init; }

    /// <summary>
    /// Default state to transition to if no choices match.
    /// </summary>
    [JsonPropertyName("default")]
    public string? Default { get; init; }
}

/// <summary>
/// A single choice rule with a condition and next state.
/// </summary>
public sealed class ChoiceRule
{
    /// <summary>
    /// The condition to evaluate.
    /// </summary>
    [JsonPropertyName("condition")]
    public required ChoiceCondition Condition { get; init; }

    /// <summary>
    /// State to transition to if the condition is true.
    /// </summary>
    [JsonPropertyName("next")]
    public required string Next { get; init; }
}

/// <summary>
/// A condition for choice evaluation.
/// </summary>
[JsonPolymorphic(TypeDiscriminatorPropertyName = "operator")]
[JsonDerivedType(typeof(ComparisonCondition), "comparison")]
[JsonDerivedType(typeof(LogicalCondition), "logical")]
public abstract class ChoiceCondition
{
    [JsonPropertyName("operator")]
    public abstract string Operator { get; }
}

/// <summary>
/// A comparison condition (equals, notEquals, greaterThan, etc.).
/// </summary>
public sealed class ComparisonCondition : ChoiceCondition
{
    [JsonPropertyName("operator")]
    public override string Operator => "comparison";

    /// <summary>
    /// JSONPath to the variable to compare.
    /// </summary>
    [JsonPropertyName("variable")]
    public required string Variable { get; init; }

    /// <summary>
    /// Comparison operator.
    /// </summary>
    [JsonPropertyName("comparisonType")]
    public required ComparisonType ComparisonType { get; init; }

    /// <summary>
    /// Value to compare against.
    /// </summary>
    [JsonPropertyName("value")]
    public object? Value { get; init; }

    /// <summary>
    /// JSONPath to the value to compare against.
    /// </summary>
    [JsonPropertyName("valuePath")]
    public string? ValuePath { get; init; }
}

/// <summary>
/// Types of comparison operations.
/// </summary>
[JsonConverter(typeof(JsonStringEnumConverter))]
public enum ComparisonType
{
    Equals,
    NotEquals,
    GreaterThan,
    GreaterThanOrEquals,
    LessThan,
    LessThanOrEquals,
    Contains,
    StartsWith,
    EndsWith,
    IsNull,
    IsNotNull,
    IsTrue,
    IsFalse
}

/// <summary>
/// A logical condition combining multiple conditions.
/// </summary>
public sealed class LogicalCondition : ChoiceCondition
{
    [JsonPropertyName("operator")]
    public override string Operator => "logical";

    /// <summary>
    /// Logical operator type.
    /// </summary>
    [JsonPropertyName("logicalType")]
    public required LogicalType LogicalType { get; init; }

    /// <summary>
    /// Conditions to combine (for And/Or).
    /// </summary>
    [JsonPropertyName("conditions")]
    public List<ChoiceCondition>? Conditions { get; init; }

    /// <summary>
    /// Single condition to negate (for Not).
    /// </summary>
    [JsonPropertyName("condition")]
    public ChoiceCondition? Condition { get; init; }
}

/// <summary>
/// Types of logical operations.
/// </summary>
[JsonConverter(typeof(JsonStringEnumConverter))]
public enum LogicalType
{
    And,
    Or,
    Not
}
