using Orchestration.Core.Models;

namespace Orchestration.Core.Contracts;

/// <summary>
/// Processes incoming events from the Service Bus.
/// </summary>
public interface IEventProcessor
{
    /// <summary>
    /// Processes an incoming event, routing it to the appropriate entity/orchestration.
    /// </summary>
    Task ProcessEventAsync(EventData eventData, CancellationToken cancellationToken = default);

    /// <summary>
    /// Handles a dead-lettered message.
    /// </summary>
    Task HandleDeadLetterAsync(EventData eventData, string deadLetterReason, CancellationToken cancellationToken = default);
}
