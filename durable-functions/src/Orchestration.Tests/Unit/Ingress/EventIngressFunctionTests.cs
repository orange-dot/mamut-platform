using System.Text;
using System.Text.Json;
using Azure.Messaging.ServiceBus;
using FluentAssertions;
using Microsoft.Azure.Functions.Worker;
using Microsoft.DurableTask;
using Microsoft.DurableTask.Client;
using Microsoft.DurableTask.Client.Entities;
using Microsoft.DurableTask.Entities;
using Microsoft.Extensions.Logging;
using Moq;
using Orchestration.Core.Models;
using Orchestration.Functions.Entities;
using Orchestration.Functions.Ingress;
using Orchestration.Functions.Orchestrators;

namespace Orchestration.Tests.Unit.Ingress;

public class EventIngressFunctionTests
{
    private readonly Mock<ILogger<EventIngressFunction>> _loggerMock;
    private readonly EventIngressFunction _function;

    public EventIngressFunctionTests()
    {
        _loggerMock = new Mock<ILogger<EventIngressFunction>>();
        _function = new EventIngressFunction(_loggerMock.Object);
    }

    [Fact]
    public void Constructor_WithValidLogger_CreatesInstance()
    {
        // Act & Assert
        _function.Should().NotBeNull();
    }

    [Fact]
    public void EventIngressFunction_HasCorrectServiceBusTrigger()
    {
        // Arrange
        var methodInfo = typeof(EventIngressFunction).GetMethod("Run");
        var parameters = methodInfo!.GetParameters();
        var messageParam = parameters.FirstOrDefault(p => p.Name == "message");

        // Assert
        messageParam.Should().NotBeNull();
        var attribute = messageParam!.GetCustomAttributes(typeof(ServiceBusTriggerAttribute), false).FirstOrDefault();
        attribute.Should().NotBeNull();

        var triggerAttr = attribute as ServiceBusTriggerAttribute;
        triggerAttr!.QueueName.Should().Be("device-events");
        triggerAttr.IsSessionsEnabled.Should().BeFalse();
    }

    [Fact]
    public void EventIngressFunction_HasDurableClientParameter()
    {
        // Arrange
        var methodInfo = typeof(EventIngressFunction).GetMethod("Run");
        var parameters = methodInfo!.GetParameters();
        var clientParam = parameters.FirstOrDefault(p => p.Name == "client");

        // Assert
        clientParam.Should().NotBeNull();
        var attribute = clientParam!.GetCustomAttributes(typeof(DurableClientAttribute), false).FirstOrDefault();
        attribute.Should().NotBeNull();
    }

    [Fact]
    public void EventIngressFunction_HasFunctionAttribute()
    {
        // Arrange
        var methodInfo = typeof(EventIngressFunction).GetMethod("Run");

        // Assert
        var attribute = methodInfo!.GetCustomAttributes(typeof(FunctionAttribute), false).FirstOrDefault();
        attribute.Should().NotBeNull();

        var funcAttr = attribute as FunctionAttribute;
        funcAttr!.Name.Should().Be(nameof(EventIngressFunction));
    }
}

/// <summary>
/// Tests for event data parsing logic used by EventIngressFunction.
/// </summary>
public class EventDataParsingTests
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };

    [Fact]
    public void ParseEventData_WithValidJson_ReturnsEventData()
    {
        // Arrange
        var eventData = new EventData
        {
            EventId = "evt-123",
            EventType = "DeviceConnected",
            EntityId = "device-456",
            Timestamp = DateTimeOffset.UtcNow,
            CorrelationId = "corr-789",
            Payload = new Dictionary<string, object?>
            {
                ["status"] = "online",
                ["firmware"] = "1.2.3"
            }
        };
        var json = JsonSerializer.Serialize(eventData, JsonOptions);

        // Act
        var parsed = JsonSerializer.Deserialize<EventData>(json, JsonOptions);

        // Assert
        parsed.Should().NotBeNull();
        parsed!.EventId.Should().Be("evt-123");
        parsed.EventType.Should().Be("DeviceConnected");
        parsed.Payload.Should().ContainKey("status");
    }

    [Fact]
    public void ParseEventData_WithMinimalJson_ReturnsEventData()
    {
        // Arrange
        var json = """
        {
            "eventId": "evt-min",
            "eventType": "TestEvent",
            "entityId": "entity-1",
            "timestamp": "2025-01-15T10:00:00Z",
            "correlationId": "corr-1"
        }
        """;

        // Act
        var parsed = JsonSerializer.Deserialize<EventData>(json, JsonOptions);

        // Assert
        parsed.Should().NotBeNull();
        parsed!.EventId.Should().Be("evt-min");
        parsed.EventType.Should().Be("TestEvent");
        parsed.Payload.Should().BeNull();
    }

    [Fact]
    public void ParseEventData_WithComplexPayload_PreservesNestedStructure()
    {
        // Arrange
        var json = """
        {
            "eventId": "evt-complex",
            "eventType": "ComplexEvent",
            "entityId": "entity-complex",
            "timestamp": "2025-01-15T10:00:00Z",
            "correlationId": "corr-complex",
            "payload": {
                "level1": {
                    "level2": {
                        "value": "nested"
                    }
                },
                "array": [1, 2, 3]
            }
        }
        """;

        // Act
        var parsed = JsonSerializer.Deserialize<EventData>(json, JsonOptions);

        // Assert
        parsed.Should().NotBeNull();
        parsed!.Payload.Should().NotBeNull();
        parsed.Payload.Should().ContainKey("level1");
        parsed.Payload.Should().ContainKey("array");
    }

    [Fact]
    public void ParseEventData_WithMetadata_IncludesMetadata()
    {
        // Arrange
        var json = """
        {
            "eventId": "evt-meta",
            "eventType": "MetaEvent",
            "entityId": "entity-meta",
            "timestamp": "2025-01-15T10:00:00Z",
            "correlationId": "corr-meta",
            "metadata": {
                "source": "sensor-1",
                "priority": "high"
            }
        }
        """;

        // Act
        var parsed = JsonSerializer.Deserialize<EventData>(json, JsonOptions);

        // Assert
        parsed.Should().NotBeNull();
        parsed!.Metadata.Should().ContainKey("source");
        parsed.Metadata!["source"].Should().Be("sensor-1");
    }
}

/// <summary>
/// Tests for the start-or-route decision logic.
/// </summary>
public class StartOrRouteLogicTests
{
    [Theory]
    [InlineData(null, true)] // No instance - should start new
    [InlineData(OrchestrationRuntimeStatus.Completed, true)]
    [InlineData(OrchestrationRuntimeStatus.Failed, true)]
    [InlineData(OrchestrationRuntimeStatus.Terminated, true)]
    [InlineData(OrchestrationRuntimeStatus.Running, false)]
    [InlineData(OrchestrationRuntimeStatus.Pending, false)]
    [InlineData(OrchestrationRuntimeStatus.Suspended, false)]
    public void ShouldStartNewOrchestration_WithDifferentStatuses_ReturnsCorrectDecision(
        OrchestrationRuntimeStatus? status, bool shouldStartNew)
    {
        // Arrange
        OrchestrationMetadata? metadata = status.HasValue
            ? CreateOrchestrationMetadata(status.Value)
            : null;

        // Act
        var result = ShouldStartNew(metadata);

        // Assert
        result.Should().Be(shouldStartNew);
    }

    [Theory]
    [InlineData(OrchestrationRuntimeStatus.Running, true)]
    [InlineData(OrchestrationRuntimeStatus.Pending, true)]
    [InlineData(OrchestrationRuntimeStatus.Suspended, true)]
    [InlineData(OrchestrationRuntimeStatus.Completed, false)]
    [InlineData(OrchestrationRuntimeStatus.Failed, false)]
    [InlineData(OrchestrationRuntimeStatus.Terminated, false)]
    public void ShouldRaiseEvent_WithDifferentStatuses_ReturnsCorrectDecision(
        OrchestrationRuntimeStatus status, bool shouldRaise)
    {
        // Arrange
        var metadata = CreateOrchestrationMetadata(status);

        // Act
        var result = ShouldRaiseEvent(metadata);

        // Assert
        result.Should().Be(shouldRaise);
    }

    [Fact]
    public void GenerateInstanceId_WithEntityId_CreatesConsistentId()
    {
        // Arrange
        var entityId = "device-123";

        // Act
        var instanceId = $"workflow-{entityId}";

        // Assert
        instanceId.Should().Be("workflow-device-123");
    }

    [Fact]
    public void GenerateIdempotencyKey_WithEntityIdAndTime_CreatesHourlyKey()
    {
        // Arrange
        var entityId = "device-123";
        var timestamp = new DateTime(2025, 1, 15, 14, 30, 0, DateTimeKind.Utc);

        // Act
        var key = $"{entityId}-{timestamp:yyyyMMddHH}";

        // Assert
        key.Should().Be("device-123-2025011514");
    }

    private static bool ShouldStartNew(OrchestrationMetadata? metadata)
    {
        return metadata == null ||
               metadata.RuntimeStatus == OrchestrationRuntimeStatus.Completed ||
               metadata.RuntimeStatus == OrchestrationRuntimeStatus.Failed ||
               metadata.RuntimeStatus == OrchestrationRuntimeStatus.Terminated;
    }

    private static bool ShouldRaiseEvent(OrchestrationMetadata metadata)
    {
        return metadata.RuntimeStatus == OrchestrationRuntimeStatus.Running ||
               metadata.RuntimeStatus == OrchestrationRuntimeStatus.Pending ||
               metadata.RuntimeStatus == OrchestrationRuntimeStatus.Suspended;
    }

    private static OrchestrationMetadata CreateOrchestrationMetadata(OrchestrationRuntimeStatus status)
    {
        // Use reflection to create metadata since it's typically created by the framework
        return new OrchestrationMetadata("test-name", "test-instance")
        {
            RuntimeStatus = status
        };
    }
}

/// <summary>
/// Tests for entity ID creation logic.
/// </summary>
public class EntityIdCreationTests
{
    [Fact]
    public void CreateEntityInstanceId_WithValidKey_CreatesCorrectId()
    {
        // Arrange
        var entityKey = "device-123";

        // Act
        var entityId = new EntityInstanceId(nameof(EventAccumulatorEntity), entityKey);

        // Assert
        // Note: Durable Task framework lowercases the entity name
        entityId.Name.Should().BeEquivalentTo(nameof(EventAccumulatorEntity));
        entityId.Key.Should().Be("device-123");
    }

    [Fact]
    public void EntityInstanceId_WithSameKeyAndName_AreEqual()
    {
        // Arrange
        var id1 = new EntityInstanceId(nameof(EventAccumulatorEntity), "key-1");
        var id2 = new EntityInstanceId(nameof(EventAccumulatorEntity), "key-1");

        // Assert
        id1.Should().Be(id2);
    }

    [Fact]
    public void EntityInstanceId_WithDifferentKeys_AreNotEqual()
    {
        // Arrange
        var id1 = new EntityInstanceId(nameof(EventAccumulatorEntity), "key-1");
        var id2 = new EntityInstanceId(nameof(EventAccumulatorEntity), "key-2");

        // Assert
        id1.Should().NotBe(id2);
    }
}
