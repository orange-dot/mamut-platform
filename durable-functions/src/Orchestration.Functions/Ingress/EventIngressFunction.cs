using System.Text.Json;
using Azure.Messaging.ServiceBus;
using Microsoft.Azure.Functions.Worker;
using Microsoft.DurableTask;
using Microsoft.DurableTask.Client;
using Microsoft.DurableTask.Client.Entities;
using Microsoft.DurableTask.Entities;
using Microsoft.Extensions.Logging;
using Orchestration.Core.Models;
using Orchestration.Functions.Entities;
using Orchestration.Functions.Orchestrators;

namespace Orchestration.Functions.Ingress;

/// <summary>
/// Service Bus triggered function that implements the start-or-route pattern.
/// Routes events to orchestrations based on entity ID from message properties.
/// </summary>
public class EventIngressFunction
{
    private readonly ILogger<EventIngressFunction> _logger;
    private const string DefaultWorkflowType = "DeviceOnboarding";
    private const int MaxDeliveryAttempts = 5;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };

    public EventIngressFunction(ILogger<EventIngressFunction> logger)
    {
        _logger = logger;
    }

    /// <summary>
    /// Processes incoming device events from Service Bus.
    /// Entity ID is extracted from message properties or Subject.
    /// </summary>
    [Function(nameof(EventIngressFunction))]
    public async Task Run(
        [ServiceBusTrigger("device-events", Connection = "ServiceBusConnection", IsSessionsEnabled = false)]
        ServiceBusReceivedMessage message,
        ServiceBusMessageActions messageActions,
        [DurableClient] DurableTaskClient client)
    {
        // Get entity ID from message properties, Subject, or generate one
        var entityId = (message.ApplicationProperties.TryGetValue("EntityId", out var eid)
            ? eid?.ToString()
            : message.Subject) ?? message.MessageId;
        var correlationId = message.CorrelationId ?? Guid.NewGuid().ToString();

        _logger.LogInformation(
            "Processing message {MessageId} for entity {EntityId} (delivery: {DeliveryCount}).",
            message.MessageId, entityId, message.DeliveryCount);

        try
        {
            // Parse the event data
            var eventData = ParseEventData(message, entityId, correlationId);
            if (eventData == null)
            {
                _logger.LogWarning("Failed to parse event data for message {MessageId}. Dead-lettering.", message.MessageId);
                await messageActions.DeadLetterMessageAsync(message, deadLetterReason: "InvalidPayload");
                return;
            }

            // Create entity reference
            var accumulatorEntityId = new EntityInstanceId(nameof(EventAccumulatorEntity), entityId);

            // Signal entity to record the event
            await client.Entities.SignalEntityAsync(
                accumulatorEntityId,
                nameof(EventAccumulatorEntity.RecordEvent),
                eventData);

            // Check if there's an existing orchestration for this entity
            var instanceId = $"workflow-{entityId}";
            var existingInstance = await client.GetInstanceAsync(instanceId);

            if (existingInstance == null ||
                existingInstance.RuntimeStatus == OrchestrationRuntimeStatus.Completed ||
                existingInstance.RuntimeStatus == OrchestrationRuntimeStatus.Failed ||
                existingInstance.RuntimeStatus == OrchestrationRuntimeStatus.Terminated)
            {
                // Start a new orchestration
                var workflowType = message.ApplicationProperties.TryGetValue("WorkflowType", out var wt)
                    ? wt.ToString() ?? DefaultWorkflowType
                    : DefaultWorkflowType;

                var input = new WorkflowInput
                {
                    WorkflowType = workflowType,
                    EntityId = entityId,
                    CorrelationId = correlationId,
                    IdempotencyKey = $"{entityId}-{DateTime.UtcNow:yyyyMMddHH}",
                    Data = new Dictionary<string, object?>
                    {
                        ["triggerEvent"] = eventData
                    }
                };

                await client.ScheduleNewOrchestrationInstanceAsync(
                    nameof(WorkflowOrchestrator),
                    input,
                    new StartOrchestrationOptions { InstanceId = instanceId });

                _logger.LogInformation(
                    "Started new orchestration {InstanceId} for entity {EntityId}.",
                    instanceId, entityId);
            }
            else if (existingInstance.RuntimeStatus == OrchestrationRuntimeStatus.Running ||
                     existingInstance.RuntimeStatus == OrchestrationRuntimeStatus.Pending ||
                     existingInstance.RuntimeStatus == OrchestrationRuntimeStatus.Suspended)
            {
                // Raise event to running orchestration
                await client.RaiseEventAsync(instanceId, "NewEvent", eventData);

                _logger.LogInformation(
                    "Raised event to running orchestration {InstanceId} for entity {EntityId}.",
                    instanceId, entityId);
            }

            // Complete the message
            await messageActions.CompleteMessageAsync(message);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex,
                "Error processing message {MessageId} for entity {EntityId}.",
                message.MessageId, entityId);

            if (message.DeliveryCount >= MaxDeliveryAttempts)
            {
                await messageActions.DeadLetterMessageAsync(message,
                    deadLetterReason: "MaxRetriesExceeded",
                    deadLetterErrorDescription: ex.Message);
            }
            else
            {
                await messageActions.AbandonMessageAsync(message);
            }
        }
    }

    private EventData? ParseEventData(
        ServiceBusReceivedMessage message,
        string entityId,
        string correlationId)
    {
        try
        {
            var body = message.Body.ToString();
            var eventData = JsonSerializer.Deserialize<EventData>(body, JsonOptions);

            if (eventData != null)
            {
                return eventData with
                {
                    EntityId = entityId,
                    CorrelationId = correlationId
                };
            }

            // If deserialization returns null, create a basic event
            return new EventData
            {
                EventType = message.Subject ?? "Unknown",
                EntityId = entityId,
                CorrelationId = correlationId,
                Payload = new Dictionary<string, object?>
                {
                    ["rawBody"] = body
                },
                Metadata = message.ApplicationProperties
                    .Where(p => p.Value is string)
                    .ToDictionary(p => p.Key, p => p.Value?.ToString() ?? "")
            };
        }
        catch (JsonException ex)
        {
            _logger.LogWarning(ex, "Failed to deserialize message body as EventData.");
            return null;
        }
    }
}
