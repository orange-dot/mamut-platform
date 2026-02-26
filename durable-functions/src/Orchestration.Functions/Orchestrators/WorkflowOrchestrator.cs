using Microsoft.Azure.Functions.Worker;
using Microsoft.DurableTask;
using Microsoft.Extensions.Logging;
using Orchestration.Core.Models;
using Orchestration.Core.Workflow;
using Orchestration.Core.Workflow.Interpreter;
using Orchestration.Core.Workflow.StateTypes;
using Orchestration.Functions.Activities.Workflow;

namespace Orchestration.Functions.Orchestrators;

/// <summary>
/// The main config-driven workflow orchestrator.
/// Interprets JSON workflow definitions and executes them using Durable Functions.
/// </summary>
public class WorkflowOrchestrator
{
    private readonly IWorkflowInterpreter _interpreter;
    private readonly ILogger<WorkflowOrchestrator> _logger;

    public WorkflowOrchestrator(IWorkflowInterpreter interpreter, ILogger<WorkflowOrchestrator> logger)
    {
        _interpreter = interpreter;
        _logger = logger;
    }

    [Function(nameof(WorkflowOrchestrator))]
    public async Task<WorkflowResult> RunOrchestrator(
        [OrchestrationTrigger] TaskOrchestrationContext context)
    {
        var input = context.GetInput<WorkflowInput>()!;

        // Use replay-safe logging
        var logger = context.CreateReplaySafeLogger<WorkflowOrchestrator>();
        logger.LogInformation(
            "Starting workflow {WorkflowType} for entity {EntityId}",
            input.WorkflowType, input.EntityId);

        try
        {
            // Load the workflow definition ONCE (determinism rule)
            var workflowDef = await context.CallActivityAsync<WorkflowDefinition>(
                nameof(LoadWorkflowDefinitionActivity),
                new LoadWorkflowDefinitionInput
                {
                    WorkflowType = input.WorkflowType,
                    Version = input.Version
                });

            // Initialize runtime state
            var state = new WorkflowRuntimeState
            {
                Input = input,
                System = new SystemValues
                {
                    InstanceId = context.InstanceId,
                    StartTime = context.CurrentUtcDateTime,
                    CurrentTime = context.CurrentUtcDateTime
                }
            };

            // Copy input data to variables for easier access
            if (input.Data != null)
            {
                foreach (var kvp in input.Data)
                {
                    state.Variables[kvp.Key] = kvp.Value;
                }
            }

            // Create the execution context wrapper
            var executionContext = new DurableWorkflowExecutionContext(context);

            // Execute the workflow step by step
            var currentStep = workflowDef.StartAt;

            while (currentStep != null)
            {
                // Update system time for each step
                state.System.CurrentTime = context.CurrentUtcDateTime;

                logger.LogInformation(
                    "Executing step {StepName} for workflow {WorkflowType}",
                    currentStep, input.WorkflowType);

                try
                {
                    currentStep = await _interpreter.ExecuteStepAsync(
                        executionContext,
                        workflowDef,
                        currentStep,
                        state);
                }
                catch (WorkflowFailedException ex)
                {
                    // Explicit failure from a Fail state
                    logger.LogWarning(
                        "Workflow failed at step {StepName}: {Error}",
                        state.CurrentStep, ex.ErrorCode);

                    return WorkflowResult.Failed(ex.Message, ex.ErrorCode);
                }
                catch (Exception ex)
                {
                    // Unexpected error - try compensation if configured
                    logger.LogError(ex,
                        "Error in step {StepName}. Attempting compensation.",
                        state.CurrentStep);

                    if (!string.IsNullOrEmpty(workflowDef.Config.CompensationState) &&
                        workflowDef.States.ContainsKey(workflowDef.Config.CompensationState))
                    {
                        state.Error = new WorkflowError
                        {
                            Message = ex.Message,
                            StepName = state.CurrentStep,
                            StackTrace = ex.StackTrace
                        };

                        // Execute compensation
                        currentStep = workflowDef.Config.CompensationState;
                        while (currentStep != null)
                        {
                            try
                            {
                                currentStep = await _interpreter.ExecuteStepAsync(
                                    executionContext,
                                    workflowDef,
                                    currentStep,
                                    state);
                            }
                            catch (Exception compEx)
                            {
                                logger.LogError(compEx,
                                    "Compensation step {StepName} failed",
                                    currentStep);
                                break;
                            }
                        }

                        return WorkflowResult.Failed(
                            ex.Message,
                            "WorkflowFailed",
                            compensated: true);
                    }

                    throw;
                }
            }

            logger.LogInformation(
                "Workflow {WorkflowType} completed successfully for entity {EntityId}",
                input.WorkflowType, input.EntityId);

            // Build output from state
            var output = state.Variables.TryGetValue("output", out var outputValue)
                ? outputValue as Dictionary<string, object?>
                : state.StepResults;

            return WorkflowResult.Succeeded(output, state);
        }
        catch (Exception ex)
        {
            logger.LogError(ex,
                "Workflow {WorkflowType} failed for entity {EntityId}",
                input.WorkflowType, input.EntityId);

            return WorkflowResult.Failed(ex.Message, "UnhandledException");
        }
    }
}
