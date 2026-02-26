using FluentAssertions;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using Moq;
using Orchestration.Core.Workflow;
using Orchestration.Core.Workflow.StateTypes;
using Orchestration.Infrastructure.Storage;

namespace Orchestration.Tests.Unit.Infrastructure;

public class WorkflowDefinitionStorageTests
{
    private readonly Mock<IConfiguration> _configurationMock;
    private readonly Mock<ILogger<WorkflowDefinitionStorage>> _loggerMock;

    public WorkflowDefinitionStorageTests()
    {
        _configurationMock = new Mock<IConfiguration>();
        _loggerMock = new Mock<ILogger<WorkflowDefinitionStorage>>();

        // Setup configuration to return empty values so blob storage is not used
        _configurationMock.Setup(x => x["WorkflowStorageConnection"]).Returns((string?)null);
        _configurationMock.Setup(x => x["WorkflowStorageContainer"]).Returns("workflow-definitions");
    }

    [Fact]
    public async Task GetAsync_WithBuiltInWorkflow_ReturnsDefinition()
    {
        // Arrange
        var storage = new WorkflowDefinitionStorage(_configurationMock.Object, _loggerMock.Object);

        // Act
        var result = await storage.GetAsync("DeviceOnboarding");

        // Assert
        result.Should().NotBeNull();
        result.Name.Should().Be("Device Onboarding Workflow");
        result.StartAt.Should().Be("WaitForEvents");
    }

    [Fact]
    public async Task GetAsync_WithNonExistentWorkflow_ThrowsException()
    {
        // Arrange
        var storage = new WorkflowDefinitionStorage(_configurationMock.Object, _loggerMock.Object);

        // Act & Assert
        var act = () => storage.GetAsync("NonExistentWorkflow");

        await act.Should().ThrowAsync<InvalidOperationException>()
            .WithMessage("*not found*");
    }

    [Fact]
    public async Task GetAsync_BuiltInWorkflow_HasCorrectStates()
    {
        // Arrange
        var storage = new WorkflowDefinitionStorage(_configurationMock.Object, _loggerMock.Object);

        // Act
        var result = await storage.GetAsync("DeviceOnboarding");

        // Assert
        result.Should().NotBeNull();
        result.States.Should().ContainKey("WaitForEvents");
        result.States.Should().ContainKey("CreateRecord");
        result.States.Should().ContainKey("Complete");
        result.States.Should().ContainKey("Compensate");
        result.States.Should().ContainKey("Failed");
    }

    [Fact]
    public async Task GetAsync_BuiltInWorkflow_HasValidConfiguration()
    {
        // Arrange
        var storage = new WorkflowDefinitionStorage(_configurationMock.Object, _loggerMock.Object);

        // Act
        var result = await storage.GetAsync("DeviceOnboarding");

        // Assert
        result.Should().NotBeNull();
        result.Config.Should().NotBeNull();
        result.Config.TimeoutSeconds.Should().Be(3600);
        result.Config.CompensationState.Should().Be("Compensate");
    }

    [Fact]
    public async Task GetAsync_BuiltInWorkflow_HasRetryPolicy()
    {
        // Arrange
        var storage = new WorkflowDefinitionStorage(_configurationMock.Object, _loggerMock.Object);

        // Act
        var result = await storage.GetAsync("DeviceOnboarding");

        // Assert
        result.Should().NotBeNull();
        result.Config.DefaultRetryPolicy.Should().NotBeNull();
        result.Config.DefaultRetryPolicy!.MaxAttempts.Should().Be(3);
        result.Config.DefaultRetryPolicy.InitialIntervalSeconds.Should().Be(5);
    }

    [Fact]
    public async Task ListWorkflowTypesAsync_ReturnsBuiltInWorkflows()
    {
        // Arrange
        var storage = new WorkflowDefinitionStorage(_configurationMock.Object, _loggerMock.Object);

        // Act
        var result = await storage.ListWorkflowTypesAsync();

        // Assert
        result.Should().Contain("DeviceOnboarding");
    }

    [Fact]
    public async Task ListVersionsAsync_ReturnsBuiltInVersion()
    {
        // Arrange
        var storage = new WorkflowDefinitionStorage(_configurationMock.Object, _loggerMock.Object);

        // Act
        var result = await storage.ListVersionsAsync("DeviceOnboarding");

        // Assert
        result.Should().Contain("1.0.0");
    }

    [Fact]
    public async Task GetAsync_CachesDefinition()
    {
        // Arrange
        var storage = new WorkflowDefinitionStorage(_configurationMock.Object, _loggerMock.Object);

        // Act - Call twice
        var result1 = await storage.GetAsync("DeviceOnboarding");
        var result2 = await storage.GetAsync("DeviceOnboarding");

        // Assert - Both should be the same cached instance
        result1.Should().NotBeNull();
        result2.Should().NotBeNull();
        result1.Id.Should().Be(result2.Id);
    }
}

public class WorkflowDefinitionTests
{
    [Fact]
    public void WorkflowDefinition_RequiredProperties_AreSet()
    {
        // Arrange & Act
        var definition = new WorkflowDefinition
        {
            Id = "test-workflow",
            Name = "Test Workflow",
            Version = "1.0",
            StartAt = "InitialState",
            States = new Dictionary<string, WorkflowStateDefinition>
            {
                ["InitialState"] = new SucceedStateDefinition()
            }
        };

        // Assert
        definition.Id.Should().Be("test-workflow");
        definition.Name.Should().Be("Test Workflow");
        definition.Version.Should().Be("1.0");
        definition.StartAt.Should().Be("InitialState");
        definition.States.Should().ContainKey("InitialState");
    }

    [Fact]
    public void WorkflowConfiguration_HasDefaultValues()
    {
        // Arrange & Act
        var config = new WorkflowConfiguration();

        // Assert - Config should be creatable with defaults
        config.Should().NotBeNull();
    }
}

public class RetryPolicyTests
{
    [Fact]
    public void RetryPolicy_RequiredProperties_AreSet()
    {
        // Arrange & Act
        var policy = new RetryPolicy
        {
            MaxAttempts = 5,
            InitialIntervalSeconds = 10,
            BackoffCoefficient = 2.0
        };

        // Assert
        policy.MaxAttempts.Should().Be(5);
        policy.InitialIntervalSeconds.Should().Be(10);
        policy.BackoffCoefficient.Should().Be(2.0);
    }

    [Fact]
    public void RetryPolicy_OptionalMaxInterval_CanBeSet()
    {
        // Arrange & Act
        var policy = new RetryPolicy
        {
            MaxAttempts = 3,
            InitialIntervalSeconds = 5,
            MaxIntervalSeconds = 300
        };

        // Assert
        policy.MaxIntervalSeconds.Should().Be(300);
    }
}
