using System.Text.Json.Serialization;

namespace Orchestration.Core.Models;

/// <summary>
/// Represents an incoming event from the Service Bus.
/// </summary>
public sealed record EventData
{
    /// <summary>
    /// Unique identifier for the event.
    /// </summary>
    [JsonPropertyName("eventId")]
    public string EventId { get; init; } = Guid.NewGuid().ToString();

    /// <summary>
    /// The type of event (e.g., "DeviceConnected", "DataReceived").
    /// </summary>
    [JsonPropertyName("eventType")]
    public required string EventType { get; init; }

    /// <summary>
    /// The entity/device ID this event belongs to.
    /// </summary>
    [JsonPropertyName("entityId")]
    public required string EntityId { get; init; }

    /// <summary>
    /// The event payload as a JSON object.
    /// </summary>
    [JsonPropertyName("payload")]
    public Dictionary<string, object?>? Payload { get; init; }

    /// <summary>
    /// Timestamp when the event was created.
    /// </summary>
    [JsonPropertyName("timestamp")]
    public DateTimeOffset Timestamp { get; init; } = DateTimeOffset.UtcNow;

    /// <summary>
    /// Correlation ID for distributed tracing.
    /// </summary>
    [JsonPropertyName("correlationId")]
    public string? CorrelationId { get; init; }

    /// <summary>
    /// Optional metadata associated with the event.
    /// </summary>
    [JsonPropertyName("metadata")]
    public Dictionary<string, string>? Metadata { get; init; }
}
