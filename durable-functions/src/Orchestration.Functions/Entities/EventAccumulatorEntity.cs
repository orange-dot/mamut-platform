using Microsoft.Azure.Functions.Worker;
using Microsoft.DurableTask.Entities;
using Microsoft.Extensions.Logging;
using Orchestration.Core.Models;

namespace Orchestration.Functions.Entities;

/// <summary>
/// Durable Entity that accumulates events for a specific device/entity.
/// </summary>
public class EventAccumulatorEntity : TaskEntity<EventAccumulatorState>
{
    private readonly ILogger<EventAccumulatorEntity> _logger;

    public EventAccumulatorEntity(ILogger<EventAccumulatorEntity> logger)
    {
        _logger = logger;
    }

    protected override EventAccumulatorState InitializeState(TaskEntityOperation operation)
    {
        return new EventAccumulatorState
        {
            EntityId = operation.Context.Id.Key,
            IsInitialized = true,
            LastActivityAt = DateTimeOffset.UtcNow
        };
    }

    /// <summary>
    /// Records a new event in the accumulator.
    /// </summary>
    public void RecordEvent(EventData eventData)
    {
        if (State.Events.Count >= EventAccumulatorState.MaxEventCount)
        {
            _logger.LogWarning(
                "Entity {EntityId} has reached max event count ({MaxCount}). Dropping event {EventId}.",
                State.EntityId, EventAccumulatorState.MaxEventCount, eventData.EventId);
            return;
        }

        var accumulatedEvent = new AccumulatedEvent
        {
            EventData = eventData,
            AccumulatedAt = DateTimeOffset.UtcNow,
            SequenceNumber = ++State.CurrentSequence
        };

        State.Events.Add(accumulatedEvent);
        State.LastActivityAt = DateTimeOffset.UtcNow;
        State.TotalEventCount++;

        _logger.LogDebug(
            "Entity {EntityId} accumulated event {EventId} (sequence: {Sequence}, total: {Count}).",
            State.EntityId, eventData.EventId, accumulatedEvent.SequenceNumber, State.Events.Count);
    }

    /// <summary>
    /// Gets all accumulated events.
    /// </summary>
    public List<AccumulatedEvent> GetEvents()
    {
        State.LastActivityAt = DateTimeOffset.UtcNow;
        return State.Events.OrderBy(e => e.SequenceNumber).ToList();
    }

    /// <summary>
    /// Clears all accumulated events.
    /// </summary>
    public void Clear()
    {
        var count = State.Events.Count;
        State.Events.Clear();
        State.LastActivityAt = DateTimeOffset.UtcNow;

        _logger.LogInformation(
            "Entity {EntityId} cleared {Count} events.",
            State.EntityId, count);
    }

    /// <summary>
    /// Gets the full entity state.
    /// </summary>
    public EventAccumulatorState GetState()
    {
        State.LastActivityAt = DateTimeOffset.UtcNow;
        return State;
    }

    /// <summary>
    /// Gets the count of accumulated events.
    /// </summary>
    public int GetEventCount()
    {
        return State.Events.Count;
    }

    /// <summary>
    /// Checks if the entity has any events.
    /// </summary>
    public bool HasEvents()
    {
        return State.Events.Count > 0;
    }

    /// <summary>
    /// Gets events since a specific sequence number.
    /// </summary>
    public List<AccumulatedEvent> GetEventsSince(long sinceSequence)
    {
        State.LastActivityAt = DateTimeOffset.UtcNow;
        return State.Events
            .Where(e => e.SequenceNumber > sinceSequence)
            .OrderBy(e => e.SequenceNumber)
            .ToList();
    }

    /// <summary>
    /// Entity function entry point.
    /// </summary>
    [Function(nameof(EventAccumulatorEntity))]
    public static Task RunEntityAsync([EntityTrigger] TaskEntityDispatcher dispatcher)
    {
        return dispatcher.DispatchAsync<EventAccumulatorEntity>();
    }
}
