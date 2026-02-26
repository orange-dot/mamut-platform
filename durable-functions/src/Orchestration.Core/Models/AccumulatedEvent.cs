using System.Text.Json.Serialization;

namespace Orchestration.Core.Models;

/// <summary>
/// Represents an event stored in the accumulator entity.
/// </summary>
public sealed class AccumulatedEvent
{
    /// <summary>
    /// The original event data.
    /// </summary>
    [JsonPropertyName("eventData")]
    public required EventData EventData { get; init; }

    /// <summary>
    /// Timestamp when the event was accumulated.
    /// </summary>
    [JsonPropertyName("accumulatedAt")]
    public DateTimeOffset AccumulatedAt { get; init; } = DateTimeOffset.UtcNow;

    /// <summary>
    /// Sequence number within the entity's event stream.
    /// </summary>
    [JsonPropertyName("sequenceNumber")]
    public long SequenceNumber { get; init; }
}
