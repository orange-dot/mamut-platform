using FluentAssertions;
using Orchestration.Core.Models;
using Orchestration.Core.Workflow.Interpreter;

namespace Orchestration.Tests.Unit.Core;

public class JsonPathResolverTests
{
    private readonly JsonPathResolver _resolver;

    public JsonPathResolverTests()
    {
        _resolver = new JsonPathResolver();
    }

    [Fact]
    public void Resolve_WithValidInputPath_ReturnsValue()
    {
        // Arrange
        var state = CreateTestState();
        state.Input = new WorkflowInput
        {
            WorkflowType = "Test",
            EntityId = "entity-123",
            Data = new Dictionary<string, object?>
            {
                ["deviceId"] = "device-456"
            }
        };

        // Act
        var result = _resolver.Resolve("$.input.deviceId", state);

        // Assert
        result.Should().Be("device-456");
    }

    [Fact]
    public void Resolve_WithVariablePath_ReturnsValue()
    {
        // Arrange
        var state = CreateTestState();
        state.Variables["counter"] = 42;

        // Act
        var result = _resolver.Resolve("$.variables.counter", state);

        // Assert
        result.Should().Be(42L);
    }

    [Fact]
    public void Resolve_WithNestedPath_ReturnsValue()
    {
        // Arrange
        var state = CreateTestState();
        state.Variables["nested"] = new Dictionary<string, object?>
        {
            ["level1"] = new Dictionary<string, object?>
            {
                ["level2"] = "deepValue"
            }
        };

        // Act
        var result = _resolver.Resolve("$.variables.nested.level1.level2", state);

        // Assert
        result.Should().Be("deepValue");
    }

    [Fact]
    public void Resolve_WithInvalidPath_ReturnsNull()
    {
        // Arrange
        var state = CreateTestState();

        // Act
        var result = _resolver.Resolve("$.nonexistent.path", state);

        // Assert
        result.Should().BeNull();
    }

    [Fact]
    public void SetValue_AddsNewVariable()
    {
        // Arrange
        var state = CreateTestState();

        // Act
        _resolver.SetValue("$.variables.newValue", "hello", state);

        // Assert
        state.Variables["newValue"].Should().Be("hello");
    }

    [Fact]
    public void SetValue_UpdatesExistingVariable()
    {
        // Arrange
        var state = CreateTestState();
        state.Variables["existing"] = "oldValue";

        // Act
        _resolver.SetValue("$.variables.existing", "newValue", state);

        // Assert
        state.Variables["existing"].Should().Be("newValue");
    }

    [Fact]
    public void SetValue_CreatesNestedStructure()
    {
        // Arrange
        var state = CreateTestState();

        // Act
        _resolver.SetValue("$.variables.level1.level2", "nestedValue", state);

        // Assert
        state.Variables.Should().ContainKey("level1");
        var level1 = state.Variables["level1"] as Dictionary<string, object?>;
        level1.Should().NotBeNull();
        level1!["level2"].Should().Be("nestedValue");
    }

    [Fact]
    public void ResolveInput_WithPlainString_ReturnsString()
    {
        // Arrange
        var state = CreateTestState();

        // Act
        var result = _resolver.ResolveInput("plainString", state);

        // Assert
        result.Should().Be("plainString");
    }

    [Fact]
    public void ResolveInput_WithJsonPathString_ResolvesPath()
    {
        // Arrange
        var state = CreateTestState();
        state.Variables["myValue"] = "resolvedValue";

        // Act
        var result = _resolver.ResolveInput("$.variables.myValue", state);

        // Assert
        result.Should().Be("resolvedValue");
    }

    [Fact]
    public void ResolveInput_WithDictionary_ResolvesAllPaths()
    {
        // Arrange
        var state = CreateTestState();
        state.Variables["var1"] = "value1";
        state.Variables["var2"] = 123;

        var input = new Dictionary<string, object?>
        {
            ["plain"] = "plainText",
            ["resolved1"] = "$.variables.var1",
            ["resolved2"] = "$.variables.var2"
        };

        // Act
        var result = _resolver.ResolveInput(input, state) as Dictionary<string, object?>;

        // Assert
        result.Should().NotBeNull();
        result!["plain"].Should().Be("plainText");
        result["resolved1"].Should().Be("value1");
        result["resolved2"].Should().Be(123L);
    }

    [Fact]
    public void Resolve_WithStepResultsPath_ReturnsValue()
    {
        // Arrange
        var state = CreateTestState();
        state.StepResults["previousStep"] = new Dictionary<string, object?>
        {
            ["outputField"] = "stepOutput"
        };

        // Act
        var result = _resolver.Resolve("$.stepResults.previousStep.outputField", state);

        // Assert
        result.Should().Be("stepOutput");
    }

    [Fact]
    public void Resolve_WithSystemPath_ReturnsValue()
    {
        // Arrange
        var state = CreateTestState();
        state.System.InstanceId = "instance-123";

        // Act
        var result = _resolver.Resolve("$.system.instanceId", state);

        // Assert
        result.Should().Be("instance-123");
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
