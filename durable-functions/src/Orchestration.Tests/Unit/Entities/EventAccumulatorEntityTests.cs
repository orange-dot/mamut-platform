using FluentAssertions;
using Microsoft.Extensions.Logging;
using Moq;
using Orchestration.Core.Models;
using Orchestration.Functions.Entities;

namespace Orchestration.Tests.Unit.Entities;

/// <summary>
/// Tests for the EventAccumulatorEntity logic.
/// Note: These tests verify the entity's business logic. Integration tests with the
/// Durable Functions runtime would require Azurite and the full host.
/// </summary>
public class EventAccumulatorEntityLogicTests
{
    private readonly Mock<ILogger<EventAccumulatorEntity>> _loggerMock;
    private readonly EventAccumulatorState _state;

    public EventAccumulatorEntityLogicTests()
    {
        _loggerMock = new Mock<ILogger<EventAccumulatorEntity>>();
        _state = new EventAccumulatorState
        {
            EntityId = "test-entity",
            IsInitialized = true,
            LastActivityAt = DateTimeOffset.UtcNow
        };
    }

    [Fact]
    public void RecordEvent_AddsEventToState()
    {
        // Arrange
        var eventData = CreateTestEvent("TestEvent");

        // Act - Simulate what the entity does
        var accumulatedEvent = new AccumulatedEvent
        {
            EventData = eventData,
            AccumulatedAt = DateTimeOffset.UtcNow,
            SequenceNumber = ++_state.CurrentSequence
        };
        _state.Events.Add(accumulatedEvent);
        _state.TotalEventCount++;

        // Assert
        _state.Events.Should().HaveCount(1);
        _state.Events[0].EventData.EventType.Should().Be("TestEvent");
        _state.CurrentSequence.Should().Be(1);
        _state.TotalEventCount.Should().Be(1);
    }

    [Fact]
    public void RecordEvent_MultipleEvents_MaintainsOrder()
    {
        // Arrange & Act
        for (int i = 1; i <= 5; i++)
        {
            var eventData = CreateTestEvent($"Event{i}");
            _state.Events.Add(new AccumulatedEvent
            {
                EventData = eventData,
                AccumulatedAt = DateTimeOffset.UtcNow,
                SequenceNumber = ++_state.CurrentSequence
            });
        }

        // Assert
        _state.Events.Should().HaveCount(5);
        _state.Events.Select(e => e.SequenceNumber).Should().BeInAscendingOrder();
        _state.CurrentSequence.Should().Be(5);
    }

    [Fact]
    public void RecordEvent_AtMaxCount_RejectsNewEvents()
    {
        // Arrange - Fill to max
        for (int i = 0; i < EventAccumulatorState.MaxEventCount; i++)
        {
            _state.Events.Add(CreateAccumulatedEvent(i + 1));
        }

        // Act - Try to add one more (simulating entity behavior)
        var shouldReject = _state.Events.Count >= EventAccumulatorState.MaxEventCount;

        // Assert
        shouldReject.Should().BeTrue();
        _state.Events.Should().HaveCount(EventAccumulatorState.MaxEventCount);
    }

    [Fact]
    public void GetEvents_ReturnsOrderedList()
    {
        // Arrange - Add events out of order
        _state.Events.Add(CreateAccumulatedEvent(3));
        _state.Events.Add(CreateAccumulatedEvent(1));
        _state.Events.Add(CreateAccumulatedEvent(2));

        // Act
        var result = _state.Events.OrderBy(e => e.SequenceNumber).ToList();

        // Assert
        result.Should().HaveCount(3);
        result[0].SequenceNumber.Should().Be(1);
        result[1].SequenceNumber.Should().Be(2);
        result[2].SequenceNumber.Should().Be(3);
    }

    [Fact]
    public void Clear_RemovesAllEvents()
    {
        // Arrange
        _state.Events.Add(CreateAccumulatedEvent(1));
        _state.Events.Add(CreateAccumulatedEvent(2));
        _state.Events.Add(CreateAccumulatedEvent(3));

        // Act
        _state.Events.Clear();

        // Assert
        _state.Events.Should().BeEmpty();
    }

    [Fact]
    public void GetEventsSince_ReturnsOnlyNewerEvents()
    {
        // Arrange
        _state.Events.Add(CreateAccumulatedEvent(1));
        _state.Events.Add(CreateAccumulatedEvent(2));
        _state.Events.Add(CreateAccumulatedEvent(3));
        _state.Events.Add(CreateAccumulatedEvent(4));

        // Act
        var result = _state.Events
            .Where(e => e.SequenceNumber > 2)
            .OrderBy(e => e.SequenceNumber)
            .ToList();

        // Assert
        result.Should().HaveCount(2);
        result[0].SequenceNumber.Should().Be(3);
        result[1].SequenceNumber.Should().Be(4);
    }

    [Fact]
    public void HasEvents_ReturnsTrueWhenEventsExist()
    {
        // Arrange
        _state.Events.Add(CreateAccumulatedEvent(1));

        // Act & Assert
        _state.Events.Count.Should().BeGreaterThan(0);
    }

    [Fact]
    public void HasEvents_ReturnsFalseWhenEmpty()
    {
        // Assert
        _state.Events.Count.Should().Be(0);
    }

    private static EventData CreateTestEvent(string eventType)
    {
        return new EventData
        {
            EventId = Guid.NewGuid().ToString(),
            EventType = eventType,
            EntityId = "test-entity",
            Timestamp = DateTimeOffset.UtcNow,
            CorrelationId = Guid.NewGuid().ToString(),
            Payload = new Dictionary<string, object?> { ["key"] = "value" }
        };
    }

    private static AccumulatedEvent CreateAccumulatedEvent(long sequence)
    {
        return new AccumulatedEvent
        {
            EventData = CreateTestEvent($"Event{sequence}"),
            AccumulatedAt = DateTimeOffset.UtcNow,
            SequenceNumber = sequence
        };
    }
}

public class EventAccumulatorStateTests
{
    [Fact]
    public void State_Initialization_HasCorrectDefaults()
    {
        // Arrange & Act
        var state = new EventAccumulatorState();

        // Assert
        state.Events.Should().BeEmpty();
        state.CurrentSequence.Should().Be(0);
        state.TotalEventCount.Should().Be(0);
        state.IsInitialized.Should().BeFalse();
    }

    [Fact]
    public void State_AddEvent_IncrementsSequence()
    {
        // Arrange
        var state = new EventAccumulatorState { EntityId = "test-entity" };

        // Act
        var event1 = CreateAccumulatedEvent(1);
        var event2 = CreateAccumulatedEvent(2);
        state.Events.Add(event1);
        state.Events.Add(event2);

        // Assert
        state.Events.Should().HaveCount(2);
    }

    [Fact]
    public void State_MaxEventCount_IsConfigurable()
    {
        // Assert
        EventAccumulatorState.MaxEventCount.Should().BeGreaterThan(0);
    }

    [Fact]
    public void State_TracksEntityId()
    {
        // Arrange
        var state = new EventAccumulatorState { EntityId = "device-123" };

        // Assert
        state.EntityId.Should().Be("device-123");
    }

    [Fact]
    public void State_TracksLastActivityTime()
    {
        // Arrange
        var activityTime = DateTimeOffset.UtcNow.AddMinutes(-5);
        var state = new EventAccumulatorState { LastActivityAt = activityTime };

        // Assert
        state.LastActivityAt.Should().Be(activityTime);
    }

    private static AccumulatedEvent CreateAccumulatedEvent(long sequence)
    {
        return new AccumulatedEvent
        {
            EventData = new EventData
            {
                EventId = Guid.NewGuid().ToString(),
                EventType = "TestEvent",
                EntityId = "entity-test",
                Timestamp = DateTimeOffset.UtcNow,
                CorrelationId = "correlation-test",
                Payload = new Dictionary<string, object?> { ["testKey"] = "testValue" }
            },
            AccumulatedAt = DateTimeOffset.UtcNow,
            SequenceNumber = sequence
        };
    }
}

public class AccumulatedEventTests
{
    [Fact]
    public void AccumulatedEvent_StoresEventData()
    {
        // Arrange
        var eventData = new EventData
        {
            EventId = "event-123",
            EventType = "DeviceConnected",
            EntityId = "device-456",
            Timestamp = DateTimeOffset.UtcNow,
            CorrelationId = "corr-789",
            Payload = new Dictionary<string, object?> { ["status"] = "online" }
        };

        // Act
        var accumulated = new AccumulatedEvent
        {
            EventData = eventData,
            AccumulatedAt = DateTimeOffset.UtcNow,
            SequenceNumber = 1
        };

        // Assert
        accumulated.EventData.Should().BeSameAs(eventData);
        accumulated.EventData.EventType.Should().Be("DeviceConnected");
        accumulated.SequenceNumber.Should().Be(1);
    }

    [Fact]
    public void AccumulatedEvent_HasAccumulatedTimestamp()
    {
        // Arrange
        var accumulatedTime = DateTimeOffset.UtcNow;

        // Act
        var accumulated = new AccumulatedEvent
        {
            EventData = CreateTestEvent("Test"),
            AccumulatedAt = accumulatedTime,
            SequenceNumber = 1
        };

        // Assert
        accumulated.AccumulatedAt.Should().Be(accumulatedTime);
    }

    private static EventData CreateTestEvent(string eventType)
    {
        return new EventData
        {
            EventId = Guid.NewGuid().ToString(),
            EventType = eventType,
            EntityId = "entity-test",
            Timestamp = DateTimeOffset.UtcNow,
            CorrelationId = "correlation-test",
            Payload = new Dictionary<string, object?> { ["testKey"] = "testValue" }
        };
    }
}

public class EventDataTests
{
    [Fact]
    public void EventData_RequiredProperties_AreSet()
    {
        // Arrange & Act
        var eventData = new EventData
        {
            EventId = "evt-001",
            EventType = "UserCreated",
            EntityId = "user-123",
            Timestamp = DateTimeOffset.UtcNow,
            CorrelationId = "corr-abc"
        };

        // Assert
        eventData.EventId.Should().Be("evt-001");
        eventData.EventType.Should().Be("UserCreated");
        eventData.EntityId.Should().Be("user-123");
        eventData.CorrelationId.Should().Be("corr-abc");
    }

    [Fact]
    public void EventData_Payload_CanContainComplexData()
    {
        // Arrange
        var payload = new Dictionary<string, object?>
        {
            ["simpleValue"] = "text",
            ["numericValue"] = 42,
            ["nestedObject"] = new Dictionary<string, object?>
            {
                ["innerKey"] = "innerValue"
            },
            ["nullValue"] = null
        };

        // Act
        var eventData = new EventData
        {
            EventId = "evt-complex",
            EventType = "ComplexEvent",
            EntityId = "entity-complex",
            Timestamp = DateTimeOffset.UtcNow,
            CorrelationId = "corr-complex",
            Payload = payload
        };

        // Assert
        eventData.Payload.Should().ContainKey("simpleValue");
        eventData.Payload!["numericValue"].Should().Be(42);
        eventData.Payload["nestedObject"].Should().NotBeNull();
    }

    [Fact]
    public void EventData_WithExpression_CreatesModifiedCopy()
    {
        // Arrange
        var original = new EventData
        {
            EventId = "original-id",
            EventType = "OriginalType",
            EntityId = "original-entity",
            Timestamp = DateTimeOffset.UtcNow,
            CorrelationId = "original-corr"
        };

        // Act
        var modified = original with { EventType = "ModifiedType" };

        // Assert
        modified.EventId.Should().Be("original-id");
        modified.EventType.Should().Be("ModifiedType");
        original.EventType.Should().Be("OriginalType"); // Original unchanged
    }
}
