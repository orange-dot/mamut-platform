using Microsoft.Azure.Functions.Worker;
using Microsoft.DurableTask.Client;
using Microsoft.DurableTask.Client.Entities;
using Microsoft.DurableTask.Entities;
using Microsoft.Extensions.Logging;
using Orchestration.Core.Models;
using Orchestration.Functions.Entities;

namespace Orchestration.Functions.Activities.Entity;

/// <summary>
/// Input for getting entity events.
/// </summary>
public sealed class GetEntityEventsInput
{
    public required string EntityId { get; init; }
    public long? SinceSequence { get; init; }
}

/// <summary>
/// Output from getting entity events.
/// </summary>
public sealed class GetEntityEventsOutput
{
    public required string EntityId { get; init; }
    public required List<AccumulatedEvent> Events { get; init; }
    public int EventCount { get; init; }
    public long LatestSequence { get; init; }
}

/// <summary>
/// Activity that retrieves accumulated events from an entity.
/// </summary>
public class GetEntityEventsActivity
{
    private readonly ILogger<GetEntityEventsActivity> _logger;

    public GetEntityEventsActivity(ILogger<GetEntityEventsActivity> logger)
    {
        _logger = logger;
    }

    [Function(nameof(GetEntityEventsActivity))]
    public async Task<GetEntityEventsOutput> Run(
        [ActivityTrigger] GetEntityEventsInput input,
        [DurableClient] DurableTaskClient client)
    {
        _logger.LogInformation(
            "Getting events from entity {EntityId} (since sequence: {SinceSequence}).",
            input.EntityId, input.SinceSequence);

        var entityId = new EntityInstanceId(nameof(EventAccumulatorEntity), input.EntityId);

        // Get the entity state
        var entityState = await client.Entities.GetEntityAsync<EventAccumulatorState>(entityId);

        List<AccumulatedEvent> events;
        if (entityState?.State != null)
        {
            if (input.SinceSequence.HasValue)
            {
                events = entityState.State.Events
                    .Where(e => e.SequenceNumber > input.SinceSequence.Value)
                    .OrderBy(e => e.SequenceNumber)
                    .ToList();
            }
            else
            {
                events = entityState.State.Events.OrderBy(e => e.SequenceNumber).ToList();
            }
        }
        else
        {
            events = new List<AccumulatedEvent>();
        }

        var latestSequence = events.Count > 0
            ? events.Max(e => e.SequenceNumber)
            : input.SinceSequence ?? 0;

        _logger.LogInformation(
            "Retrieved {EventCount} events from entity {EntityId}.",
            events.Count, input.EntityId);

        return new GetEntityEventsOutput
        {
            EntityId = input.EntityId,
            Events = events,
            EventCount = events.Count,
            LatestSequence = latestSequence
        };
    }
}
