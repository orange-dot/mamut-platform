using Microsoft.Azure.Functions.Worker;
using Microsoft.DurableTask.Client;
using Microsoft.DurableTask.Client.Entities;
using Microsoft.DurableTask.Entities;
using Microsoft.Extensions.Logging;
using Orchestration.Functions.Entities;

namespace Orchestration.Functions.Activities.Entity;

/// <summary>
/// Input for clearing entity events.
/// </summary>
public sealed class ClearEntityInput
{
    public required string EntityId { get; init; }
}

/// <summary>
/// Output from clearing entity events.
/// </summary>
public sealed class ClearEntityOutput
{
    public required string EntityId { get; init; }
    public bool Success { get; init; }
    public DateTimeOffset ClearedAt { get; init; }
}

/// <summary>
/// Activity that clears accumulated events from an entity.
/// </summary>
public class ClearEntityActivity
{
    private readonly ILogger<ClearEntityActivity> _logger;

    public ClearEntityActivity(ILogger<ClearEntityActivity> logger)
    {
        _logger = logger;
    }

    [Function(nameof(ClearEntityActivity))]
    public async Task<ClearEntityOutput> Run(
        [ActivityTrigger] ClearEntityInput input,
        [DurableClient] DurableTaskClient client)
    {
        _logger.LogInformation("Clearing events from entity {EntityId}.", input.EntityId);

        var entityId = new EntityInstanceId(nameof(EventAccumulatorEntity), input.EntityId);

        await client.Entities.SignalEntityAsync(
            entityId,
            nameof(EventAccumulatorEntity.Clear));

        _logger.LogInformation("Cleared events from entity {EntityId}.", input.EntityId);

        return new ClearEntityOutput
        {
            EntityId = input.EntityId,
            Success = true,
            ClearedAt = DateTimeOffset.UtcNow
        };
    }
}
