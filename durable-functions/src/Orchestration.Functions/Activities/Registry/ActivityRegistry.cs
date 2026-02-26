using System.Collections.Concurrent;

namespace Orchestration.Functions.Activities.Registry;

/// <summary>
/// In-memory registry for activity functions.
/// </summary>
public sealed class ActivityRegistry : IActivityRegistry
{
    private readonly ConcurrentDictionary<string, ActivityMetadata> _activities = new(StringComparer.OrdinalIgnoreCase);

    public ActivityRegistry()
    {
        RegisterBuiltInActivities();
    }

    /// <inheritdoc />
    public void Register(ActivityMetadata metadata)
    {
        _activities[metadata.Name] = metadata;
    }

    /// <inheritdoc />
    public ActivityMetadata? GetMetadata(string name)
    {
        return _activities.TryGetValue(name, out var metadata) ? metadata : null;
    }

    /// <inheritdoc />
    public bool Exists(string name)
    {
        return _activities.ContainsKey(name);
    }

    /// <inheritdoc />
    public IReadOnlyList<ActivityMetadata> GetAll()
    {
        return _activities.Values.ToList();
    }

    private void RegisterBuiltInActivities()
    {
        // Database activities
        Register(new ActivityMetadata
        {
            Name = "CreateRecordActivity",
            Description = "Creates a new record in the database with idempotency",
            SupportsCompensation = true,
            CompensatingActivity = "CompensateCreateRecordActivity",
            Tags = ["database", "create"]
        });

        Register(new ActivityMetadata
        {
            Name = "UpdateRecordActivity",
            Description = "Updates an existing record in the database",
            SupportsCompensation = true,
            CompensatingActivity = "CompensateUpdateRecordActivity",
            Tags = ["database", "update"]
        });

        Register(new ActivityMetadata
        {
            Name = "GetRecordActivity",
            Description = "Retrieves a record from the database",
            SupportsCompensation = false,
            Tags = ["database", "read"]
        });

        Register(new ActivityMetadata
        {
            Name = "CompensateCreateRecordActivity",
            Description = "Rolls back a record creation",
            SupportsCompensation = false,
            Tags = ["database", "compensation"]
        });

        // Entity activities
        Register(new ActivityMetadata
        {
            Name = "GetEntityEventsActivity",
            Description = "Gets accumulated events from an entity",
            SupportsCompensation = false,
            Tags = ["entity", "read"]
        });

        Register(new ActivityMetadata
        {
            Name = "ClearEntityActivity",
            Description = "Clears accumulated events from an entity",
            SupportsCompensation = false,
            Tags = ["entity", "write"]
        });

        // External activities
        Register(new ActivityMetadata
        {
            Name = "CallExternalApiActivity",
            Description = "Calls an external HTTP API",
            SupportsCompensation = false,
            Tags = ["external", "http"]
        });

        Register(new ActivityMetadata
        {
            Name = "NotifyActivity",
            Description = "Sends notifications via various channels",
            SupportsCompensation = false,
            Tags = ["external", "notification"]
        });

        // Workflow activities
        Register(new ActivityMetadata
        {
            Name = "LoadWorkflowDefinitionActivity",
            Description = "Loads a workflow definition from storage",
            SupportsCompensation = false,
            Tags = ["workflow", "read"]
        });

        Register(new ActivityMetadata
        {
            Name = "ValidateWorkflowActivity",
            Description = "Validates a workflow definition",
            SupportsCompensation = false,
            Tags = ["workflow", "validation"]
        });
    }
}
