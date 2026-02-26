using FluentAssertions;
using Moq;
using Orchestration.Core.Models;
using Orchestration.Core.Workflow;
using Orchestration.Core.Workflow.Interpreter;
using Orchestration.Core.Workflow.StateTypes;

namespace Orchestration.Tests.Unit.Core;

public class WorkflowInterpreterTests
{
    private readonly Mock<IJsonPathResolver> _jsonPathResolverMock;
    private readonly WorkflowInterpreter _interpreter;

    public WorkflowInterpreterTests()
    {
        _jsonPathResolverMock = new Mock<IJsonPathResolver>();
        _interpreter = new WorkflowInterpreter(_jsonPathResolverMock.Object);
    }

    [Fact]
    public async Task ExecuteStepAsync_WithSucceedState_ReturnsNull()
    {
        // Arrange
        var context = new Mock<IWorkflowExecutionContext>();
        var workflow = CreateTestWorkflow();
        workflow.States["SuccessEnd"] = new SucceedStateDefinition
        {
            Comment = "Success"
        };
        var state = CreateTestState();

        // Act
        var result = await _interpreter.ExecuteStepAsync(
            context.Object, workflow, "SuccessEnd", state);

        // Assert
        result.Should().BeNull();
    }

    [Fact]
    public async Task ExecuteStepAsync_WithFailState_ThrowsWorkflowFailedException()
    {
        // Arrange
        var context = new Mock<IWorkflowExecutionContext>();
        var workflow = CreateTestWorkflow();
        workflow.States["FailEnd"] = new FailStateDefinition
        {
            Error = "TestError",
            Cause = "Test failure cause"
        };
        var state = CreateTestState();

        // Act & Assert
        var act = () => _interpreter.ExecuteStepAsync(
            context.Object, workflow, "FailEnd", state);

        await act.Should().ThrowAsync<WorkflowFailedException>()
            .WithMessage("Test failure cause");
    }

    [Fact]
    public async Task ExecuteStepAsync_WithChoiceState_SelectsCorrectBranch()
    {
        // Arrange
        var context = new Mock<IWorkflowExecutionContext>();
        var workflow = CreateTestWorkflow();
        workflow.States["CheckStatus"] = new ChoiceStateDefinition
        {
            Choices =
            [
                new ChoiceRule
                {
                    Condition = new ComparisonCondition
                    {
                        Variable = "$.variables.status",
                        ComparisonType = ComparisonType.Equals,
                        Value = "approved"
                    },
                    Next = "ApprovedBranch"
                },
                new ChoiceRule
                {
                    Condition = new ComparisonCondition
                    {
                        Variable = "$.variables.status",
                        ComparisonType = ComparisonType.Equals,
                        Value = "rejected"
                    },
                    Next = "RejectedBranch"
                }
            ],
            Default = "DefaultBranch"
        };
        var state = CreateTestState();
        state.Variables["status"] = "approved";

        _jsonPathResolverMock
            .Setup(x => x.Resolve("$.variables.status", state))
            .Returns("approved");

        // Act
        var result = await _interpreter.ExecuteStepAsync(
            context.Object, workflow, "CheckStatus", state);

        // Assert
        result.Should().Be("ApprovedBranch");
    }

    [Fact]
    public async Task ExecuteStepAsync_WithChoiceState_UsesDefaultWhenNoMatch()
    {
        // Arrange
        var context = new Mock<IWorkflowExecutionContext>();
        var workflow = CreateTestWorkflow();
        workflow.States["CheckStatus"] = new ChoiceStateDefinition
        {
            Choices =
            [
                new ChoiceRule
                {
                    Condition = new ComparisonCondition
                    {
                        Variable = "$.variables.status",
                        ComparisonType = ComparisonType.Equals,
                        Value = "approved"
                    },
                    Next = "ApprovedBranch"
                }
            ],
            Default = "DefaultBranch"
        };
        var state = CreateTestState();
        state.Variables["status"] = "unknown";

        _jsonPathResolverMock
            .Setup(x => x.Resolve("$.variables.status", state))
            .Returns("unknown");

        // Act
        var result = await _interpreter.ExecuteStepAsync(
            context.Object, workflow, "CheckStatus", state);

        // Assert
        result.Should().Be("DefaultBranch");
    }

    [Fact]
    public async Task ExecuteStepAsync_WithTaskState_CallsActivity()
    {
        // Arrange
        var context = new Mock<IWorkflowExecutionContext>();
        var workflow = CreateTestWorkflow();
        workflow.States["CallApi"] = new TaskStateDefinition
        {
            Activity = "CallExternalApi",
            Input = new Dictionary<string, object?>
            {
                ["url"] = "https://api.example.com"
            },
            ResultPath = "$.stepResults.apiResult",
            Next = "NextStep"
        };
        var state = CreateTestState();

        _jsonPathResolverMock
            .Setup(x => x.ResolveInput(It.IsAny<object?>(), state))
            .Returns(new Dictionary<string, object?> { ["url"] = "https://api.example.com" });

        context
            .Setup(x => x.CallActivityAsync<object?>(
                "CallExternalApi",
                It.IsAny<object?>(),
                It.IsAny<RetryPolicy?>()))
            .ReturnsAsync(new Dictionary<string, object?> { ["response"] = "success" });

        // Act
        var result = await _interpreter.ExecuteStepAsync(
            context.Object, workflow, "CallApi", state);

        // Assert
        result.Should().Be("NextStep");
        context.Verify(x => x.CallActivityAsync<object?>(
            "CallExternalApi",
            It.IsAny<object?>(),
            It.IsAny<RetryPolicy?>()), Times.Once);
    }

    [Fact]
    public async Task ExecuteStepAsync_WithWaitState_Duration_WaitsCorrectTime()
    {
        // Arrange
        var context = new Mock<IWorkflowExecutionContext>();
        var workflow = CreateTestWorkflow();
        workflow.States["WaitStep"] = new WaitStateDefinition
        {
            Seconds = 30,
            Next = "AfterWait"
        };
        var state = CreateTestState();

        context.Setup(x => x.CreateTimerAsync(It.IsAny<TimeSpan>()))
            .Returns(Task.CompletedTask);

        // Act
        var result = await _interpreter.ExecuteStepAsync(
            context.Object, workflow, "WaitStep", state);

        // Assert
        result.Should().Be("AfterWait");
        context.Verify(x => x.CreateTimerAsync(TimeSpan.FromSeconds(30)), Times.Once);
    }

    [Fact]
    public async Task ExecuteStepAsync_WithTaskState_End_ReturnsNull()
    {
        // Arrange
        var context = new Mock<IWorkflowExecutionContext>();
        var workflow = CreateTestWorkflow();
        workflow.States["FinalTask"] = new TaskStateDefinition
        {
            Activity = "FinalActivity",
            End = true
        };
        var state = CreateTestState();

        _jsonPathResolverMock
            .Setup(x => x.ResolveInput(It.IsAny<object?>(), state))
            .Returns((object?)null);

        context
            .Setup(x => x.CallActivityAsync<object?>(
                "FinalActivity",
                It.IsAny<object?>(),
                It.IsAny<RetryPolicy?>()))
            .ReturnsAsync((object?)null);

        // Act
        var result = await _interpreter.ExecuteStepAsync(
            context.Object, workflow, "FinalTask", state);

        // Assert
        result.Should().BeNull();
    }

    [Fact]
    public async Task ExecuteStepAsync_WithUnknownStep_ThrowsException()
    {
        // Arrange
        var context = new Mock<IWorkflowExecutionContext>();
        var workflow = CreateTestWorkflow();
        var state = CreateTestState();

        // Act & Assert
        var act = () => _interpreter.ExecuteStepAsync(
            context.Object, workflow, "NonExistentStep", state);

        await act.Should().ThrowAsync<InvalidOperationException>()
            .WithMessage("*not found*");
    }

    private static WorkflowDefinition CreateTestWorkflow()
    {
        return new WorkflowDefinition
        {
            Id = "test-workflow",
            Name = "Test Workflow",
            Version = "1.0",
            StartAt = "Start",
            States = new Dictionary<string, WorkflowStateDefinition>(),
            Config = new WorkflowConfiguration()
        };
    }

    private static WorkflowRuntimeState CreateTestState()
    {
        return new WorkflowRuntimeState
        {
            Variables = new Dictionary<string, object?>(),
            StepResults = new Dictionary<string, object?>(),
            System = new SystemValues
            {
                InstanceId = "test-instance",
                StartTime = DateTimeOffset.UtcNow,
                CurrentTime = DateTimeOffset.UtcNow
            }
        };
    }
}
