using System.Text.Json.Serialization;

namespace Orchestration.Core.Workflow.StateTypes;

/// <summary>
/// A state that waits for a duration, timestamp, or external event.
/// </summary>
public sealed class WaitStateDefinition : WorkflowStateDefinition
{
    [JsonPropertyName("type")]
    public override string Type => "Wait";

    /// <summary>
    /// Fixed number of seconds to wait.
    /// </summary>
    [JsonPropertyName("seconds")]
    public int? Seconds { get; init; }

    /// <summary>
    /// JSONPath to resolve the number of seconds to wait.
    /// </summary>
    [JsonPropertyName("secondsPath")]
    public string? SecondsPath { get; init; }

    /// <summary>
    /// Fixed timestamp to wait until.
    /// </summary>
    [JsonPropertyName("timestamp")]
    public DateTimeOffset? Timestamp { get; init; }

    /// <summary>
    /// JSONPath to resolve the timestamp to wait until.
    /// </summary>
    [JsonPropertyName("timestampPath")]
    public string? TimestampPath { get; init; }

    /// <summary>
    /// External event to wait for.
    /// </summary>
    [JsonPropertyName("externalEvent")]
    public ExternalEventWait? ExternalEvent { get; init; }
}

/// <summary>
/// Configuration for waiting on an external event.
/// </summary>
public sealed class ExternalEventWait
{
    /// <summary>
    /// Name of the event to wait for.
    /// </summary>
    [JsonPropertyName("eventName")]
    public required string EventName { get; init; }

    /// <summary>
    /// Timeout in seconds for the wait.
    /// </summary>
    [JsonPropertyName("timeoutSeconds")]
    public int? TimeoutSeconds { get; init; }

    /// <summary>
    /// State to transition to if the wait times out.
    /// </summary>
    [JsonPropertyName("timeoutNext")]
    public string? TimeoutNext { get; init; }

    /// <summary>
    /// Where to store the event data in state.
    /// </summary>
    [JsonPropertyName("resultPath")]
    public string? ResultPath { get; init; }
}
