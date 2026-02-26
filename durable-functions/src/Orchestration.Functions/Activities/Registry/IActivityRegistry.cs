namespace Orchestration.Functions.Activities.Registry;

/// <summary>
/// Registry for activity functions to enable dynamic discovery and validation.
/// </summary>
public interface IActivityRegistry
{
    /// <summary>
    /// Registers an activity with its metadata.
    /// </summary>
    void Register(ActivityMetadata metadata);

    /// <summary>
    /// Gets metadata for an activity by name.
    /// </summary>
    ActivityMetadata? GetMetadata(string name);

    /// <summary>
    /// Checks if an activity exists.
    /// </summary>
    bool Exists(string name);

    /// <summary>
    /// Gets all registered activities.
    /// </summary>
    IReadOnlyList<ActivityMetadata> GetAll();
}

/// <summary>
/// Metadata about a registered activity.
/// </summary>
public sealed class ActivityMetadata
{
    /// <summary>
    /// The unique name of the activity.
    /// </summary>
    public required string Name { get; init; }

    /// <summary>
    /// Description of what the activity does.
    /// </summary>
    public string? Description { get; init; }

    /// <summary>
    /// The input type for the activity.
    /// </summary>
    public Type? InputType { get; init; }

    /// <summary>
    /// The output type for the activity.
    /// </summary>
    public Type? OutputType { get; init; }

    /// <summary>
    /// Whether the activity supports compensation.
    /// </summary>
    public bool SupportsCompensation { get; init; }

    /// <summary>
    /// The name of the compensating activity.
    /// </summary>
    public string? CompensatingActivity { get; init; }

    /// <summary>
    /// Tags for categorization.
    /// </summary>
    public IReadOnlyList<string>? Tags { get; init; }
}
