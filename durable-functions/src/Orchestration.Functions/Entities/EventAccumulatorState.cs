using Orchestration.Core.Models;

namespace Orchestration.Functions.Entities;

/// <summary>
/// State model for the Event Accumulator Entity.
/// </summary>
public sealed class EventAccumulatorState
{
    /// <summary>
    /// Maximum number of events to accumulate before rejecting new events.
    /// </summary>
    public const int MaxEventCount = 1000;

    /// <summary>
    /// The accumulated events.
    /// </summary>
    public List<AccumulatedEvent> Events { get; set; } = new();

    /// <summary>
    /// Current sequence number for ordering events.
    /// </summary>
    public long CurrentSequence { get; set; }

    /// <summary>
    /// Timestamp of the last activity.
    /// </summary>
    public DateTimeOffset LastActivityAt { get; set; } = DateTimeOffset.UtcNow;

    /// <summary>
    /// The entity ID (device/correlation ID).
    /// </summary>
    public string? EntityId { get; set; }

    /// <summary>
    /// Whether the entity has been initialized.
    /// </summary>
    public bool IsInitialized { get; set; }

    /// <summary>
    /// Total count of events ever accumulated (including cleared).
    /// </summary>
    public long TotalEventCount { get; set; }
}
