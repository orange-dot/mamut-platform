using Microsoft.Azure.Functions.Worker;
using Microsoft.Extensions.Logging;
using Orchestration.Core.Workflow;
using Orchestration.Core.Workflow.StateTypes;
using Orchestration.Functions.Activities.Registry;

namespace Orchestration.Functions.Activities.Workflow;

/// <summary>
/// Output from validating a workflow.
/// </summary>
public sealed class ValidateWorkflowOutput
{
    public bool IsValid { get; init; }
    public List<string> Errors { get; init; } = new();
    public List<string> Warnings { get; init; } = new();
}

/// <summary>
/// Activity that validates a workflow definition.
/// </summary>
public class ValidateWorkflowActivity
{
    private readonly IActivityRegistry _activityRegistry;
    private readonly ILogger<ValidateWorkflowActivity> _logger;

    public ValidateWorkflowActivity(
        IActivityRegistry activityRegistry,
        ILogger<ValidateWorkflowActivity> logger)
    {
        _activityRegistry = activityRegistry;
        _logger = logger;
    }

    [Function(nameof(ValidateWorkflowActivity))]
    public ValidateWorkflowOutput Run([ActivityTrigger] WorkflowDefinition definition)
    {
        _logger.LogInformation(
            "Validating workflow definition: {WorkflowId} v{Version}",
            definition.Id, definition.Version);

        var errors = new List<string>();
        var warnings = new List<string>();

        // Validate required fields
        if (string.IsNullOrEmpty(definition.Id))
        {
            errors.Add("Workflow ID is required");
        }

        if (string.IsNullOrEmpty(definition.Name))
        {
            errors.Add("Workflow name is required");
        }

        if (string.IsNullOrEmpty(definition.StartAt))
        {
            errors.Add("StartAt state is required");
        }

        if (definition.States == null || definition.States.Count == 0)
        {
            errors.Add("At least one state is required");
            return new ValidateWorkflowOutput { IsValid = false, Errors = errors };
        }

        // Validate StartAt points to an existing state
        if (!string.IsNullOrEmpty(definition.StartAt) &&
            !definition.States.ContainsKey(definition.StartAt))
        {
            errors.Add($"StartAt state '{definition.StartAt}' does not exist");
        }

        // Validate each state
        foreach (var (stateName, state) in definition.States)
        {
            ValidateState(stateName, state, definition.States, errors, warnings);
        }

        // Check for unreachable states
        var reachable = GetReachableStates(definition);
        foreach (var stateName in definition.States.Keys)
        {
            if (!reachable.Contains(stateName))
            {
                warnings.Add($"State '{stateName}' is unreachable");
            }
        }

        var isValid = errors.Count == 0;

        _logger.LogInformation(
            "Workflow validation complete. Valid: {IsValid}, Errors: {ErrorCount}, Warnings: {WarningCount}",
            isValid, errors.Count, warnings.Count);

        return new ValidateWorkflowOutput
        {
            IsValid = isValid,
            Errors = errors,
            Warnings = warnings
        };
    }

    private void ValidateState(
        string stateName,
        WorkflowStateDefinition state,
        Dictionary<string, WorkflowStateDefinition> allStates,
        List<string> errors,
        List<string> warnings)
    {
        // Validate Next reference
        if (!string.IsNullOrEmpty(state.Next) && !allStates.ContainsKey(state.Next))
        {
            errors.Add($"State '{stateName}' references non-existent next state '{state.Next}'");
        }

        // Must have either End or Next (except for Choice and Fail states)
        if (!state.End && string.IsNullOrEmpty(state.Next) &&
            state is not ChoiceStateDefinition &&
            state is not FailStateDefinition)
        {
            errors.Add($"State '{stateName}' must have either 'end' or 'next' defined");
        }

        // State-specific validation
        switch (state)
        {
            case TaskStateDefinition taskState:
                ValidateTaskState(stateName, taskState, allStates, errors, warnings);
                break;

            case ChoiceStateDefinition choiceState:
                ValidateChoiceState(stateName, choiceState, allStates, errors);
                break;

            case ParallelStateDefinition parallelState:
                ValidateParallelState(stateName, parallelState, errors, warnings);
                break;

            case WaitStateDefinition waitState:
                ValidateWaitState(stateName, waitState, errors);
                break;

            case CompensationStateDefinition compensationState:
                ValidateCompensationState(stateName, compensationState, errors, warnings);
                break;
        }
    }

    private void ValidateTaskState(
        string stateName,
        TaskStateDefinition taskState,
        Dictionary<string, WorkflowStateDefinition> allStates,
        List<string> errors,
        List<string> warnings)
    {
        if (string.IsNullOrEmpty(taskState.Activity))
        {
            errors.Add($"Task state '{stateName}' must specify an activity");
            return;
        }

        // Validate activity exists
        if (!_activityRegistry.Exists(taskState.Activity))
        {
            warnings.Add($"Task state '{stateName}' references unknown activity '{taskState.Activity}'");
        }

        // Validate catch handlers
        if (taskState.Catch != null)
        {
            foreach (var catchDef in taskState.Catch)
            {
                if (!allStates.ContainsKey(catchDef.Next))
                {
                    errors.Add(
                        $"Catch handler in state '{stateName}' references non-existent state '{catchDef.Next}'");
                }
            }
        }
    }

    private void ValidateChoiceState(
        string stateName,
        ChoiceStateDefinition choiceState,
        Dictionary<string, WorkflowStateDefinition> allStates,
        List<string> errors)
    {
        if (choiceState.Choices == null || choiceState.Choices.Count == 0)
        {
            errors.Add($"Choice state '{stateName}' must have at least one choice");
            return;
        }

        foreach (var choice in choiceState.Choices)
        {
            if (!allStates.ContainsKey(choice.Next))
            {
                errors.Add($"Choice in state '{stateName}' references non-existent state '{choice.Next}'");
            }
        }

        if (!string.IsNullOrEmpty(choiceState.Default) && !allStates.ContainsKey(choiceState.Default))
        {
            errors.Add($"Default in choice state '{stateName}' references non-existent state '{choiceState.Default}'");
        }
    }

    private void ValidateParallelState(
        string stateName,
        ParallelStateDefinition parallelState,
        List<string> errors,
        List<string> warnings)
    {
        if (parallelState.Branches == null || parallelState.Branches.Count == 0)
        {
            errors.Add($"Parallel state '{stateName}' must have at least one branch");
            return;
        }

        foreach (var branch in parallelState.Branches)
        {
            if (string.IsNullOrEmpty(branch.Name))
            {
                errors.Add($"Branch in parallel state '{stateName}' must have a name");
            }

            if (string.IsNullOrEmpty(branch.StartAt))
            {
                errors.Add($"Branch '{branch.Name}' in parallel state '{stateName}' must have a StartAt");
            }
            else if (!branch.States.ContainsKey(branch.StartAt))
            {
                errors.Add(
                    $"Branch '{branch.Name}' StartAt '{branch.StartAt}' does not exist in branch states");
            }
        }
    }

    private void ValidateWaitState(
        string stateName,
        WaitStateDefinition waitState,
        List<string> errors)
    {
        var waitTypeCount = 0;

        if (waitState.Seconds.HasValue) waitTypeCount++;
        if (!string.IsNullOrEmpty(waitState.SecondsPath)) waitTypeCount++;
        if (waitState.Timestamp.HasValue) waitTypeCount++;
        if (!string.IsNullOrEmpty(waitState.TimestampPath)) waitTypeCount++;
        if (waitState.ExternalEvent != null) waitTypeCount++;

        if (waitTypeCount == 0)
        {
            errors.Add($"Wait state '{stateName}' must specify a wait type");
        }
        else if (waitTypeCount > 1)
        {
            errors.Add($"Wait state '{stateName}' can only have one wait type");
        }
    }

    private void ValidateCompensationState(
        string stateName,
        CompensationStateDefinition compensationState,
        List<string> errors,
        List<string> warnings)
    {
        if (compensationState.Steps == null || compensationState.Steps.Count == 0)
        {
            warnings.Add($"Compensation state '{stateName}' has no steps");
            return;
        }

        foreach (var step in compensationState.Steps)
        {
            if (string.IsNullOrEmpty(step.Activity))
            {
                errors.Add($"Compensation step '{step.Name}' must specify an activity");
            }
            else if (!_activityRegistry.Exists(step.Activity))
            {
                warnings.Add(
                    $"Compensation step '{step.Name}' references unknown activity '{step.Activity}'");
            }
        }
    }

    private HashSet<string> GetReachableStates(WorkflowDefinition definition)
    {
        var reachable = new HashSet<string>();
        var queue = new Queue<string>();

        if (!string.IsNullOrEmpty(definition.StartAt))
        {
            queue.Enqueue(definition.StartAt);
        }

        while (queue.Count > 0)
        {
            var current = queue.Dequeue();

            if (!reachable.Add(current))
                continue;

            if (!definition.States.TryGetValue(current, out var state))
                continue;

            // Add next state
            if (!string.IsNullOrEmpty(state.Next))
            {
                queue.Enqueue(state.Next);
            }

            // Add choice transitions
            if (state is ChoiceStateDefinition choiceState)
            {
                foreach (var choice in choiceState.Choices ?? [])
                {
                    queue.Enqueue(choice.Next);
                }
                if (!string.IsNullOrEmpty(choiceState.Default))
                {
                    queue.Enqueue(choiceState.Default);
                }
            }

            // Add catch transitions
            if (state is TaskStateDefinition taskState && taskState.Catch != null)
            {
                foreach (var catchDef in taskState.Catch)
                {
                    queue.Enqueue(catchDef.Next);
                }
            }

            // Add parallel branch states
            if (state is ParallelStateDefinition parallelState)
            {
                foreach (var branch in parallelState.Branches ?? [])
                {
                    foreach (var branchState in branch.States.Keys)
                    {
                        reachable.Add($"{current}:{branchState}");
                    }
                }
            }

            // Add compensation final state
            if (state is CompensationStateDefinition compensationState &&
                !string.IsNullOrEmpty(compensationState.FinalState))
            {
                queue.Enqueue(compensationState.FinalState);
            }
        }

        return reachable;
    }
}
