using System.Text.Json;
using Orchestration.Core.Models;
using Orchestration.Core.Workflow.StateTypes;

namespace Orchestration.Core.Workflow.Interpreter;

/// <summary>
/// Interprets and executes workflow state definitions.
/// </summary>
public sealed class WorkflowInterpreter : IWorkflowInterpreter
{
    private readonly IJsonPathResolver _jsonPathResolver;

    public WorkflowInterpreter(IJsonPathResolver jsonPathResolver)
    {
        _jsonPathResolver = jsonPathResolver;
    }

    /// <inheritdoc />
    public async Task<string?> ExecuteStepAsync(
        IWorkflowExecutionContext context,
        WorkflowDefinition definition,
        string currentStep,
        WorkflowRuntimeState state)
    {
        if (!definition.States.TryGetValue(currentStep, out var stateDefinition))
        {
            throw new InvalidOperationException($"State '{currentStep}' not found in workflow definition.");
        }

        state.CurrentStep = currentStep;

        try
        {
            var nextStep = stateDefinition switch
            {
                TaskStateDefinition taskState => await ExecuteTaskStateAsync(context, taskState, state, definition),
                WaitStateDefinition waitState => await ExecuteWaitStateAsync(context, waitState, state),
                ChoiceStateDefinition choiceState => ExecuteChoiceState(choiceState, state),
                ParallelStateDefinition parallelState => await ExecuteParallelStateAsync(context, parallelState, state, definition),
                CompensationStateDefinition compensationState => await ExecuteCompensationStateAsync(context, compensationState, state),
                SucceedStateDefinition succeedState => ExecuteSucceedState(succeedState, state),
                FailStateDefinition failState => ExecuteFailState(failState, state),
                _ => throw new InvalidOperationException($"Unknown state type: {stateDefinition.Type}")
            };

            return nextStep;
        }
        catch (Exception ex) when (stateDefinition is TaskStateDefinition taskState && taskState.Catch?.Any() == true)
        {
            return HandleCatch(taskState.Catch, ex, state);
        }
    }

    private async Task<string?> ExecuteTaskStateAsync(
        IWorkflowExecutionContext context,
        TaskStateDefinition taskState,
        WorkflowRuntimeState state,
        WorkflowDefinition definition)
    {
        var input = _jsonPathResolver.ResolveInput(taskState.Input, state);
        var retry = taskState.Retry ?? definition.Config.DefaultRetryPolicy;

        object? result;
        try
        {
            result = await context.CallActivityAsync<object?>(taskState.Activity, input, retry);
        }
        catch (Exception ex)
        {
            state.Error = new WorkflowError
            {
                Message = ex.Message,
                StepName = state.CurrentStep,
                ActivityName = taskState.Activity,
                StackTrace = ex.StackTrace
            };
            throw;
        }

        // Store result in state
        if (!string.IsNullOrEmpty(taskState.ResultPath))
        {
            _jsonPathResolver.SetValue(taskState.ResultPath, result, state);
        }

        // Track executed step for potential compensation
        if (!string.IsNullOrEmpty(taskState.CompensateWith))
        {
            state.ExecutedSteps.Add(new ExecutedStep
            {
                StepName = state.CurrentStep!,
                StepType = taskState.Type,
                ActivityName = taskState.Activity,
                CompensationActivity = taskState.CompensateWith,
                Input = input,
                Output = result
            });
        }

        return taskState.End ? null : taskState.Next;
    }

    private async Task<string?> ExecuteWaitStateAsync(
        IWorkflowExecutionContext context,
        WaitStateDefinition waitState,
        WorkflowRuntimeState state)
    {
        if (waitState.ExternalEvent != null)
        {
            return await ExecuteExternalEventWaitAsync(context, waitState.ExternalEvent, state, waitState);
        }

        if (waitState.Seconds.HasValue)
        {
            await context.CreateTimerAsync(TimeSpan.FromSeconds(waitState.Seconds.Value));
        }
        else if (!string.IsNullOrEmpty(waitState.SecondsPath))
        {
            var seconds = _jsonPathResolver.Resolve<int>(waitState.SecondsPath, state);
            await context.CreateTimerAsync(TimeSpan.FromSeconds(seconds));
        }
        else if (waitState.Timestamp.HasValue)
        {
            await context.CreateTimerAsync(waitState.Timestamp.Value);
        }
        else if (!string.IsNullOrEmpty(waitState.TimestampPath))
        {
            var timestamp = _jsonPathResolver.Resolve<DateTimeOffset>(waitState.TimestampPath, state);
            await context.CreateTimerAsync(timestamp);
        }

        return waitState.End ? null : waitState.Next;
    }

    private async Task<string?> ExecuteExternalEventWaitAsync(
        IWorkflowExecutionContext context,
        ExternalEventWait eventWait,
        WorkflowRuntimeState state,
        WaitStateDefinition waitState)
    {
        var timeout = eventWait.TimeoutSeconds.HasValue
            ? TimeSpan.FromSeconds(eventWait.TimeoutSeconds.Value)
            : (TimeSpan?)null;

        try
        {
            var eventData = await context.WaitForExternalEventAsync<object?>(eventWait.EventName, timeout);

            if (!string.IsNullOrEmpty(eventWait.ResultPath))
            {
                _jsonPathResolver.SetValue(eventWait.ResultPath, eventData, state);
            }

            return waitState.End ? null : waitState.Next;
        }
        catch (TimeoutException)
        {
            if (!string.IsNullOrEmpty(eventWait.TimeoutNext))
            {
                return eventWait.TimeoutNext;
            }
            throw;
        }
    }

    private string? ExecuteChoiceState(ChoiceStateDefinition choiceState, WorkflowRuntimeState state)
    {
        foreach (var choice in choiceState.Choices)
        {
            if (EvaluateCondition(choice.Condition, state))
            {
                return choice.Next;
            }
        }

        if (!string.IsNullOrEmpty(choiceState.Default))
        {
            return choiceState.Default;
        }

        throw new InvalidOperationException("No choice matched and no default specified.");
    }

    private bool EvaluateCondition(ChoiceCondition condition, WorkflowRuntimeState state)
    {
        return condition switch
        {
            ComparisonCondition comparison => EvaluateComparisonCondition(comparison, state),
            LogicalCondition logical => EvaluateLogicalCondition(logical, state),
            _ => false
        };
    }

    private bool EvaluateComparisonCondition(ComparisonCondition condition, WorkflowRuntimeState state)
    {
        var variable = _jsonPathResolver.Resolve(condition.Variable, state);
        var compareValue = !string.IsNullOrEmpty(condition.ValuePath)
            ? _jsonPathResolver.Resolve(condition.ValuePath, state)
            : condition.Value;

        return condition.ComparisonType switch
        {
            ComparisonType.Equals => Equals(variable, ConvertValue(compareValue, variable)),
            ComparisonType.NotEquals => !Equals(variable, ConvertValue(compareValue, variable)),
            ComparisonType.GreaterThan => Compare(variable, compareValue) > 0,
            ComparisonType.GreaterThanOrEquals => Compare(variable, compareValue) >= 0,
            ComparisonType.LessThan => Compare(variable, compareValue) < 0,
            ComparisonType.LessThanOrEquals => Compare(variable, compareValue) <= 0,
            ComparisonType.Contains => variable?.ToString()?.Contains(compareValue?.ToString() ?? "") == true,
            ComparisonType.StartsWith => variable?.ToString()?.StartsWith(compareValue?.ToString() ?? "") == true,
            ComparisonType.EndsWith => variable?.ToString()?.EndsWith(compareValue?.ToString() ?? "") == true,
            ComparisonType.IsNull => variable == null,
            ComparisonType.IsNotNull => variable != null,
            ComparisonType.IsTrue => variable is true || (variable is string s && s.Equals("true", StringComparison.OrdinalIgnoreCase)),
            ComparisonType.IsFalse => variable is false || (variable is string s && s.Equals("false", StringComparison.OrdinalIgnoreCase)),
            _ => false
        };
    }

    private static object? ConvertValue(object? value, object? targetType)
    {
        if (value == null || targetType == null)
            return value;

        if (value is JsonElement element)
        {
            return element.ValueKind switch
            {
                JsonValueKind.String => element.GetString(),
                JsonValueKind.Number when targetType is int => element.GetInt32(),
                JsonValueKind.Number when targetType is long => element.GetInt64(),
                JsonValueKind.Number => element.GetDouble(),
                JsonValueKind.True => true,
                JsonValueKind.False => false,
                _ => value
            };
        }

        return value;
    }

    private static int Compare(object? left, object? right)
    {
        if (left == null && right == null) return 0;
        if (left == null) return -1;
        if (right == null) return 1;

        if (left is IComparable comparable)
        {
            var rightConverted = Convert.ChangeType(right, left.GetType());
            return comparable.CompareTo(rightConverted);
        }

        return string.Compare(left.ToString(), right.ToString(), StringComparison.Ordinal);
    }

    private bool EvaluateLogicalCondition(LogicalCondition condition, WorkflowRuntimeState state)
    {
        return condition.LogicalType switch
        {
            LogicalType.And => condition.Conditions?.All(c => EvaluateCondition(c, state)) == true,
            LogicalType.Or => condition.Conditions?.Any(c => EvaluateCondition(c, state)) == true,
            LogicalType.Not => condition.Condition != null && !EvaluateCondition(condition.Condition, state),
            _ => false
        };
    }

    private async Task<string?> ExecuteParallelStateAsync(
        IWorkflowExecutionContext context,
        ParallelStateDefinition parallelState,
        WorkflowRuntimeState state,
        WorkflowDefinition definition)
    {
        var branchTasks = parallelState.Branches.Select(async branch =>
        {
            var branchState = new WorkflowRuntimeState
            {
                Input = state.Input,
                Variables = new Dictionary<string, object?>(state.Variables),
                System = state.System
            };

            var currentStep = branch.StartAt;
            while (currentStep != null)
            {
                if (!branch.States.TryGetValue(currentStep, out var branchStepDef))
                    break;

                var branchDef = new WorkflowDefinition
                {
                    Id = definition.Id,
                    Name = $"{definition.Name}_{branch.Name}",
                    Version = definition.Version,
                    StartAt = branch.StartAt,
                    States = branch.States,
                    Config = definition.Config
                };

                currentStep = await ExecuteStepAsync(context, branchDef, currentStep, branchState);
            }

            return new { Branch = branch.Name, State = branchState };
        });

        var results = await context.WhenAllAsync(branchTasks.ToArray());

        if (!string.IsNullOrEmpty(parallelState.ResultPath))
        {
            var branchResults = results.ToDictionary(
                r => r.Branch,
                r => (object?)r.State.StepResults);
            _jsonPathResolver.SetValue(parallelState.ResultPath, branchResults, state);
        }

        return parallelState.End ? null : parallelState.Next;
    }

    private async Task<string?> ExecuteCompensationStateAsync(
        IWorkflowExecutionContext context,
        CompensationStateDefinition compensationState,
        WorkflowRuntimeState state)
    {
        foreach (var step in compensationState.Steps)
        {
            if (!string.IsNullOrEmpty(step.Condition))
            {
                var conditionResult = _jsonPathResolver.Resolve<bool>(step.Condition, state);
                if (!conditionResult)
                    continue;
            }

            try
            {
                var input = _jsonPathResolver.ResolveInput(step.Input, state);
                await context.CallActivityAsync<object?>(step.Activity, input);
            }
            catch (Exception) when (compensationState.ContinueOnError)
            {
                // Log but continue
            }
        }

        if (!string.IsNullOrEmpty(compensationState.FinalState))
        {
            return compensationState.FinalState;
        }

        return compensationState.End ? null : compensationState.Next;
    }

    private string? ExecuteSucceedState(SucceedStateDefinition succeedState, WorkflowRuntimeState state)
    {
        if (succeedState.Output != null)
        {
            var output = _jsonPathResolver.ResolveInput(succeedState.Output, state);
            _jsonPathResolver.SetValue("$.variables.output", output, state);
        }

        return null; // Terminal state
    }

    private string? ExecuteFailState(FailStateDefinition failState, WorkflowRuntimeState state)
    {
        state.Error = new WorkflowError
        {
            Message = failState.Cause ?? failState.Error,
            Code = failState.Error,
            StepName = state.CurrentStep
        };

        throw new WorkflowFailedException(failState.Error, failState.Cause);
    }

    private string? HandleCatch(List<CatchDefinition> catches, Exception ex, WorkflowRuntimeState state)
    {
        var errorType = ex.GetType().Name;

        foreach (var catchDef in catches)
        {
            if (catchDef.Errors.Contains("States.ALL") ||
                catchDef.Errors.Contains(errorType) ||
                catchDef.Errors.Any(e => ex.Message.Contains(e)))
            {
                if (!string.IsNullOrEmpty(catchDef.ResultPath))
                {
                    _jsonPathResolver.SetValue(catchDef.ResultPath, new
                    {
                        error = errorType,
                        message = ex.Message
                    }, state);
                }

                return catchDef.Next;
            }
        }

        throw ex; // Re-throw if no catch matched
    }
}

/// <summary>
/// Exception thrown when a workflow explicitly fails via a Fail state.
/// </summary>
public class WorkflowFailedException : Exception
{
    public string ErrorCode { get; }

    public WorkflowFailedException(string errorCode, string? cause)
        : base(cause ?? errorCode)
    {
        ErrorCode = errorCode;
    }
}
