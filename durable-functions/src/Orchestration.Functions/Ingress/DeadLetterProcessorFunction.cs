using System.Text.Json;
using Azure.Messaging.ServiceBus;
using Microsoft.Azure.Functions.Worker;
using Microsoft.Extensions.Logging;

namespace Orchestration.Functions.Ingress;

/// <summary>
/// Processes dead-lettered messages for alerting and potential recovery.
/// </summary>
public class DeadLetterProcessorFunction
{
    private readonly ILogger<DeadLetterProcessorFunction> _logger;

    public DeadLetterProcessorFunction(ILogger<DeadLetterProcessorFunction> logger)
    {
        _logger = logger;
    }

    /// <summary>
    /// Processes messages from the dead-letter queue.
    /// </summary>
    [Function(nameof(DeadLetterProcessorFunction))]
    public async Task Run(
        [ServiceBusTrigger("device-events/$deadletterqueue", Connection = "ServiceBusConnection")]
        ServiceBusReceivedMessage message,
        ServiceBusMessageActions messageActions)
    {
        var deadLetterReason = message.DeadLetterReason ?? "Unknown";
        var deadLetterError = message.DeadLetterErrorDescription ?? "No description";

        _logger.LogError(
            "Dead-lettered message received. MessageId: {MessageId}, SessionId: {SessionId}, " +
            "Reason: {Reason}, Error: {Error}, DeliveryCount: {DeliveryCount}",
            message.MessageId,
            message.SessionId,
            deadLetterReason,
            deadLetterError,
            message.DeliveryCount);

        // Log additional details for investigation
        var details = new
        {
            message.MessageId,
            message.SessionId,
            message.CorrelationId,
            message.Subject,
            DeadLetterReason = deadLetterReason,
            DeadLetterError = deadLetterError,
            message.DeliveryCount,
            message.EnqueuedTime,
            Body = GetBodyPreview(message),
            ApplicationProperties = message.ApplicationProperties
        };

        _logger.LogWarning(
            "Dead letter details: {Details}",
            JsonSerializer.Serialize(details));

        // Check if this is a recoverable error
        if (IsRecoverable(deadLetterReason))
        {
            _logger.LogInformation(
                "Message {MessageId} may be recoverable. Manual intervention required.",
                message.MessageId);
        }

        // Complete the message to remove it from DLQ
        // In production, you might want to move it to a poison message store
        await messageActions.CompleteMessageAsync(message);
    }

    private static string GetBodyPreview(ServiceBusReceivedMessage message)
    {
        try
        {
            var body = message.Body.ToString();
            return body.Length > 500 ? body[..500] + "..." : body;
        }
        catch
        {
            return "[Unable to read body]";
        }
    }

    private static bool IsRecoverable(string deadLetterReason)
    {
        var recoverableReasons = new[]
        {
            "MaxRetriesExceeded",
            "ServiceUnavailable",
            "Timeout"
        };

        return recoverableReasons.Any(r =>
            deadLetterReason.Contains(r, StringComparison.OrdinalIgnoreCase));
    }
}
