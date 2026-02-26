using FluentAssertions;
using Microsoft.Azure.Functions.Worker;
using Microsoft.DurableTask;
using Microsoft.Extensions.Logging;
using Moq;
using Orchestration.Core.Models;
using Orchestration.Core.Workflow;
using Orchestration.Core.Workflow.Interpreter;
using Orchestration.Core.Workflow.StateTypes;
using Orchestration.Functions.Activities.Workflow;
using Orchestration.Functions.Orchestrators;

namespace Orchestration.Tests.Unit.Orchestrators;

public class WorkflowOrchestratorTests
{
    private readonly Mock<IWorkflowInterpreter> _interpreterMock;
    private readonly Mock<ILogger<WorkflowOrchestrator>> _loggerMock;
    private readonly WorkflowOrchestrator _orchestrator;

    public WorkflowOrchestratorTests()
    {
        _interpreterMock = new Mock<IWorkflowInterpreter>();
        _loggerMock = new Mock<ILogger<WorkflowOrchestrator>>();
        _orchestrator = new WorkflowOrchestrator(_interpreterMock.Object, _loggerMock.Object);
    }

    [Fact]
    public void Constructor_WithValidDependencies_CreatesInstance()
    {
        // Assert
        _orchestrator.Should().NotBeNull();
    }

    [Fact]
    public void WorkflowOrchestrator_HasFunctionAttribute()
    {
        // Arrange
        var methodInfo = typeof(WorkflowOrchestrator).GetMethod("RunOrchestrator");

        // Assert
        var attribute = methodInfo!.GetCustomAttributes(typeof(FunctionAttribute), false).FirstOrDefault();
        attribute.Should().NotBeNull();

        var funcAttr = attribute as FunctionAttribute;
        funcAttr!.Name.Should().Be(nameof(WorkflowOrchestrator));
    }

    [Fact]
    public void WorkflowOrchestrator_HasOrchestrationTriggerParameter()
    {
        // Arrange
        var methodInfo = typeof(WorkflowOrchestrator).GetMethod("RunOrchestrator");
        var parameters = methodInfo!.GetParameters();
        var contextParam = parameters.FirstOrDefault(p => p.ParameterType == typeof(TaskOrchestrationContext));

        // Assert
        contextParam.Should().NotBeNull();
        var attribute = contextParam!.GetCustomAttributes(typeof(OrchestrationTriggerAttribute), false).FirstOrDefault();
        attribute.Should().NotBeNull();
    }

    [Fact]
    public void WorkflowOrchestrator_ReturnsWorkflowResult()
    {
        // Arrange
        var methodInfo = typeof(WorkflowOrchestrator).GetMethod("RunOrchestrator");

        // Assert
        methodInfo!.ReturnType.Should().Be(typeof(Task<WorkflowResult>));
    }
}

/// <summary>
/// Tests for WorkflowResult model.
/// </summary>
public class WorkflowResultTests
{
    [Fact]
    public void Succeeded_CreatesSuccessfulResult()
    {
        // Arrange
        var output = new Dictionary<string, object?> { ["value"] = "test" };
        var state = new WorkflowRuntimeState();

        // Act
        var result = WorkflowResult.Succeeded(output, state);

        // Assert
        result.Success.Should().BeTrue();
        result.Output.Should().ContainKey("value");
        result.Error.Should().BeNull();
    }

    [Fact]
    public void Failed_CreatesFailedResult()
    {
        // Arrange
        var errorMessage = "Something went wrong";
        var errorCode = "TEST_ERROR";

        // Act
        var result = WorkflowResult.Failed(errorMessage, errorCode);

        // Assert
        result.Success.Should().BeFalse();
        result.Error.Should().Be(errorMessage);
        result.ErrorCode.Should().Be(errorCode);
    }

    [Fact]
    public void Failed_WithCompensation_MarksAsCompensated()
    {
        // Act
        var result = WorkflowResult.Failed("Error", "CODE", compensated: true);

        // Assert
        result.Success.Should().BeFalse();
        result.Compensated.Should().BeTrue();
    }

    [Fact]
    public void Succeeded_WithNullOutput_IsStillSuccessful()
    {
        // Act
        var result = WorkflowResult.Succeeded(null, new WorkflowRuntimeState());

        // Assert
        result.Success.Should().BeTrue();
        result.Output.Should().BeNull();
    }
}

/// <summary>
/// Tests for WorkflowInput model.
/// </summary>
public class WorkflowInputTests
{
    [Fact]
    public void WorkflowInput_RequiredProperties_AreSet()
    {
        // Arrange & Act
        var input = new WorkflowInput
        {
            WorkflowType = "DeviceOnboarding",
            EntityId = "device-123",
            CorrelationId = "corr-456",
            IdempotencyKey = "key-789"
        };

        // Assert
        input.WorkflowType.Should().Be("DeviceOnboarding");
        input.EntityId.Should().Be("device-123");
        input.CorrelationId.Should().Be("corr-456");
        input.IdempotencyKey.Should().Be("key-789");
    }

    [Fact]
    public void WorkflowInput_Data_CanContainComplexTypes()
    {
        // Arrange & Act
        var input = new WorkflowInput
        {
            WorkflowType = "Test",
            EntityId = "test",
            CorrelationId = "test",
            IdempotencyKey = "test",
            Data = new Dictionary<string, object?>
            {
                ["string"] = "value",
                ["number"] = 42,
                ["nested"] = new Dictionary<string, object?>
                {
                    ["inner"] = "innerValue"
                }
            }
        };

        // Assert
        input.Data.Should().ContainKey("string");
        input.Data!["number"].Should().Be(42);
        input.Data["nested"].Should().NotBeNull();
    }

    [Fact]
    public void WorkflowInput_Version_DefaultsToNull()
    {
        // Arrange & Act
        var input = new WorkflowInput
        {
            WorkflowType = "Test",
            EntityId = "test",
            CorrelationId = "test",
            IdempotencyKey = "test"
        };

        // Assert
        input.Version.Should().BeNull();
    }
}

/// <summary>
/// Tests for WorkflowRuntimeState model.
/// </summary>
public class WorkflowRuntimeStateTests
{
    [Fact]
    public void WorkflowRuntimeState_Initialization_HasEmptyCollections()
    {
        // Arrange & Act
        var state = new WorkflowRuntimeState();

        // Assert
        state.Variables.Should().BeEmpty();
        state.StepResults.Should().BeEmpty();
        state.ExecutedSteps.Should().BeEmpty();
    }

    [Fact]
    public void WorkflowRuntimeState_Variables_CanBeModified()
    {
        // Arrange
        var state = new WorkflowRuntimeState();

        // Act
        state.Variables["key1"] = "value1";
        state.Variables["key2"] = 42;

        // Assert
        state.Variables.Should().HaveCount(2);
        state.Variables["key1"].Should().Be("value1");
        state.Variables["key2"].Should().Be(42);
    }

    [Fact]
    public void WorkflowRuntimeState_StepResults_TracksActivityOutput()
    {
        // Arrange
        var state = new WorkflowRuntimeState();

        // Act
        state.StepResults["Step1"] = new { result = "success" };
        state.StepResults["Step2"] = new { count = 10 };

        // Assert
        state.StepResults.Should().HaveCount(2);
        state.StepResults.Should().ContainKey("Step1");
        state.StepResults.Should().ContainKey("Step2");
    }

    [Fact]
    public void WorkflowRuntimeState_ExecutedSteps_TracksCompensation()
    {
        // Arrange
        var state = new WorkflowRuntimeState();

        // Act
        state.ExecutedSteps.Add(new ExecutedStep
        {
            StepName = "CreateRecord",
            StepType = "Task",
            ActivityName = "CreateRecordActivity",
            CompensationActivity = "CompensateCreateRecordActivity"
        });

        // Assert
        state.ExecutedSteps.Should().HaveCount(1);
        state.ExecutedSteps[0].CompensationActivity.Should().Be("CompensateCreateRecordActivity");
    }

    [Fact]
    public void WorkflowRuntimeState_CurrentStep_TracksCurrentExecution()
    {
        // Arrange
        var state = new WorkflowRuntimeState();

        // Act
        state.CurrentStep = "ProcessData";

        // Assert
        state.CurrentStep.Should().Be("ProcessData");
    }

    [Fact]
    public void WorkflowRuntimeState_Error_CapturesFailureDetails()
    {
        // Arrange
        var state = new WorkflowRuntimeState();

        // Act
        state.Error = new WorkflowError
        {
            Message = "Operation failed",
            Code = "ACTIVITY_FAILED",
            StepName = "CallApi",
            ActivityName = "CallExternalApiActivity"
        };

        // Assert
        state.Error.Should().NotBeNull();
        state.Error.Message.Should().Be("Operation failed");
        state.Error.Code.Should().Be("ACTIVITY_FAILED");
        state.Error.StepName.Should().Be("CallApi");
    }

    [Fact]
    public void WorkflowRuntimeState_System_ContainsSystemValues()
    {
        // Arrange
        var state = new WorkflowRuntimeState
        {
            System = new SystemValues
            {
                InstanceId = "workflow-123",
                StartTime = DateTimeOffset.UtcNow.AddMinutes(-5),
                CurrentTime = DateTimeOffset.UtcNow
            }
        };

        // Assert
        state.System.Should().NotBeNull();
        state.System.InstanceId.Should().Be("workflow-123");
        state.System.CurrentTime.Should().BeAfter(state.System.StartTime);
    }
}

/// <summary>
/// Tests for DurableWorkflowExecutionContext adapter.
/// </summary>
public class DurableWorkflowExecutionContextTests
{
    [Fact]
    public void DurableWorkflowExecutionContext_ImplementsInterface()
    {
        // Assert
        typeof(DurableWorkflowExecutionContext).Should().Implement<IWorkflowExecutionContext>();
    }
}

/// <summary>
/// Tests for LoadWorkflowDefinitionInput model.
/// </summary>
public class LoadWorkflowDefinitionInputTests
{
    [Fact]
    public void LoadWorkflowDefinitionInput_RequiredProperties_AreSet()
    {
        // Arrange & Act
        var input = new LoadWorkflowDefinitionInput
        {
            WorkflowType = "DeviceOnboarding",
            Version = "1.0"
        };

        // Assert
        input.WorkflowType.Should().Be("DeviceOnboarding");
        input.Version.Should().Be("1.0");
    }

    [Fact]
    public void LoadWorkflowDefinitionInput_Version_CanBeNull()
    {
        // Arrange & Act
        var input = new LoadWorkflowDefinitionInput
        {
            WorkflowType = "TestWorkflow"
        };

        // Assert
        input.Version.Should().BeNull();
    }
}

/// <summary>
/// Tests for ExecutedStep model used in compensation.
/// </summary>
public class ExecutedStepTests
{
    [Fact]
    public void ExecutedStep_CapturesAllFields()
    {
        // Arrange & Act
        var step = new ExecutedStep
        {
            StepName = "CreateUser",
            StepType = "Task",
            ActivityName = "CreateUserActivity",
            CompensationActivity = "DeleteUserActivity",
            Input = new { userId = "123" },
            Output = new { success = true }
        };

        // Assert
        step.StepName.Should().Be("CreateUser");
        step.StepType.Should().Be("Task");
        step.ActivityName.Should().Be("CreateUserActivity");
        step.CompensationActivity.Should().Be("DeleteUserActivity");
        step.Input.Should().NotBeNull();
        step.Output.Should().NotBeNull();
    }

    [Fact]
    public void ExecutedStep_WithoutCompensation_HasNullCompensationActivity()
    {
        // Arrange & Act
        var step = new ExecutedStep
        {
            StepName = "ReadData",
            StepType = "Task",
            ActivityName = "GetDataActivity"
        };

        // Assert
        step.CompensationActivity.Should().BeNull();
    }
}
