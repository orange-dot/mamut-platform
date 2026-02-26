using FluentAssertions;
using Microsoft.Extensions.Logging;
using Moq;
using Orchestration.Core.Contracts;
using Orchestration.Functions.Activities.Database;

namespace Orchestration.Tests.Unit.Activities;

public class CreateRecordActivityTests
{
    private readonly Mock<IWorkflowRepository> _repositoryMock;
    private readonly Mock<ILogger<CreateRecordActivity>> _loggerMock;
    private readonly CreateRecordActivity _activity;

    public CreateRecordActivityTests()
    {
        _repositoryMock = new Mock<IWorkflowRepository>();
        _loggerMock = new Mock<ILogger<CreateRecordActivity>>();
        _activity = new CreateRecordActivity(_repositoryMock.Object, _loggerMock.Object);
    }

    [Fact]
    public async Task Run_WithNewRecord_CreatesSuccessfully()
    {
        // Arrange
        var input = new CreateRecordInput
        {
            RecordType = "OnboardingRecord",
            IdempotencyKey = "idem-key-1",
            Data = new Dictionary<string, object?>
            {
                ["status"] = "pending",
                ["deviceId"] = "device-456"
            }
        };

        _repositoryMock
            .Setup(x => x.GetIdempotencyRecordAsync<CreateRecordOutput>(input.IdempotencyKey, default))
            .ReturnsAsync((CreateRecordOutput?)null);

        _repositoryMock
            .Setup(x => x.CreateRecordAsync(It.IsAny<Dictionary<string, object?>>(), input.IdempotencyKey, default))
            .ReturnsAsync(new Dictionary<string, object?> { ["id"] = "new-record-id" });

        // Act
        var result = await _activity.Run(input);

        // Assert
        result.Should().NotBeNull();
        result.RecordType.Should().Be("OnboardingRecord");
        result.WasExisting.Should().BeFalse();
        result.RecordId.Should().NotBeNullOrEmpty();
    }

    [Fact]
    public async Task Run_WithExistingIdempotencyKey_ReturnsExistingRecord()
    {
        // Arrange
        var input = new CreateRecordInput
        {
            RecordType = "OnboardingRecord",
            IdempotencyKey = "existing-key",
            Data = new Dictionary<string, object?> { ["status"] = "pending" }
        };

        var existingResult = new CreateRecordOutput
        {
            RecordId = "existing-record-id",
            RecordType = "OnboardingRecord",
            WasExisting = false,
            CreatedAt = DateTimeOffset.UtcNow.AddMinutes(-10)
        };

        _repositoryMock
            .Setup(x => x.GetIdempotencyRecordAsync<CreateRecordOutput>(input.IdempotencyKey, default))
            .ReturnsAsync(existingResult);

        // Act
        var result = await _activity.Run(input);

        // Assert
        result.Should().NotBeNull();
        result.RecordId.Should().Be("existing-record-id");
        result.WasExisting.Should().BeTrue();

        // Verify that CreateRecordAsync was never called since we found an existing record
        _repositoryMock.Verify(x => x.CreateRecordAsync(
            It.IsAny<Dictionary<string, object?>>(),
            It.IsAny<string>(),
            default), Times.Never);
    }
}

public class CreateRecordInputTests
{
    [Fact]
    public void CreateRecordInput_RequiredProperties_AreSet()
    {
        // Arrange & Act
        var input = new CreateRecordInput
        {
            RecordType = "TestRecord",
            IdempotencyKey = "test-key",
            Data = new Dictionary<string, object?> { ["field"] = "value" }
        };

        // Assert
        input.RecordType.Should().Be("TestRecord");
        input.IdempotencyKey.Should().Be("test-key");
        input.Data.Should().ContainKey("field");
    }
}

public class CreateRecordOutputTests
{
    [Fact]
    public void CreateRecordOutput_RequiredProperties_AreSet()
    {
        // Arrange & Act
        var output = new CreateRecordOutput
        {
            RecordId = "record-123",
            RecordType = "TestRecord",
            WasExisting = false,
            CreatedAt = DateTimeOffset.UtcNow
        };

        // Assert
        output.RecordId.Should().Be("record-123");
        output.RecordType.Should().Be("TestRecord");
        output.WasExisting.Should().BeFalse();
    }

    [Fact]
    public void CreateRecordOutput_WithExpression_CreatesModifiedCopy()
    {
        // Arrange
        var original = new CreateRecordOutput
        {
            RecordId = "record-123",
            RecordType = "TestRecord",
            WasExisting = false,
            CreatedAt = DateTimeOffset.UtcNow
        };

        // Act
        var modified = original with { WasExisting = true };

        // Assert
        modified.WasExisting.Should().BeTrue();
        original.WasExisting.Should().BeFalse();
    }
}
