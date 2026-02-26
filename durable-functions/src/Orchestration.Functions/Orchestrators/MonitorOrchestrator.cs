using Microsoft.Azure.Functions.Worker;
using Microsoft.DurableTask;
using Microsoft.DurableTask.Client;
using Microsoft.Extensions.Logging;
using Orchestration.Functions.Activities.External;

namespace Orchestration.Functions.Orchestrators;

/// <summary>
/// Input for the monitor orchestrator.
/// </summary>
public sealed class MonitorOrchestratorInput
{
    public TimeSpan CheckInterval { get; init; } = TimeSpan.FromMinutes(15);
    public TimeSpan StuckThreshold { get; init; } = TimeSpan.FromHours(50);
    public int MaxIterations { get; init; } = 100;
}

/// <summary>
/// Orchestrator that monitors for stuck workflows and alerts operations.
/// </summary>
public class MonitorOrchestrator
{
    private readonly ILogger<MonitorOrchestrator> _logger;

    public MonitorOrchestrator(ILogger<MonitorOrchestrator> logger)
    {
        _logger = logger;
    }

    [Function(nameof(MonitorOrchestrator))]
    public async Task RunOrchestrator(
        [OrchestrationTrigger] TaskOrchestrationContext context)
    {
        var input = context.GetInput<MonitorOrchestratorInput>() ?? new MonitorOrchestratorInput();
        var logger = context.CreateReplaySafeLogger<MonitorOrchestrator>();

        for (var iteration = 0; iteration < input.MaxIterations; iteration++)
        {
            logger.LogInformation(
                "Monitor iteration {Iteration}: checking for stuck workflows",
                iteration);

            // Call activity to check for stuck workflows
            var stuckWorkflows = await context.CallActivityAsync<List<StuckWorkflowInfo>>(
                nameof(CheckStuckWorkflowsActivity),
                input.StuckThreshold);

            if (stuckWorkflows.Count > 0)
            {
                logger.LogWarning(
                    "Found {Count} stuck workflows",
                    stuckWorkflows.Count);

                // Notify operations
                await context.CallActivityAsync(
                    nameof(NotifyActivity),
                    new NotifyInput
                    {
                        Channel = NotificationChannel.Log,
                        Subject = "Stuck Workflows Detected",
                        Message = $"Found {stuckWorkflows.Count} workflows running longer than {input.StuckThreshold.TotalHours} hours",
                        Data = new Dictionary<string, object?>
                        {
                            ["workflows"] = stuckWorkflows
                        }
                    });
            }

            // Wait for next check interval
            await context.CreateTimer(input.CheckInterval, CancellationToken.None);
        }

        logger.LogInformation("Monitor completed after {Iterations} iterations", input.MaxIterations);
    }
}

/// <summary>
/// Information about a stuck workflow.
/// </summary>
public sealed class StuckWorkflowInfo
{
    public required string InstanceId { get; init; }
    public required string WorkflowType { get; init; }
    public DateTimeOffset StartedAt { get; init; }
    public TimeSpan Duration { get; init; }
    public string? CurrentStep { get; init; }
}

/// <summary>
/// Activity that checks for stuck workflows.
/// </summary>
public class CheckStuckWorkflowsActivity
{
    private readonly ILogger<CheckStuckWorkflowsActivity> _logger;

    public CheckStuckWorkflowsActivity(ILogger<CheckStuckWorkflowsActivity> logger)
    {
        _logger = logger;
    }

    [Function(nameof(CheckStuckWorkflowsActivity))]
    public async Task<List<StuckWorkflowInfo>> Run(
        [ActivityTrigger] TimeSpan stuckThreshold,
        [DurableClient] DurableTaskClient client)
    {
        _logger.LogInformation("Checking for workflows stuck longer than {Threshold}", stuckThreshold);

        var stuckWorkflows = new List<StuckWorkflowInfo>();
        var cutoffTime = DateTimeOffset.UtcNow - stuckThreshold;

        // Query running orchestrations
        var query = new OrchestrationQuery
        {
            Statuses = [OrchestrationRuntimeStatus.Running, OrchestrationRuntimeStatus.Pending],
            CreatedFrom = DateTimeOffset.MinValue,
            CreatedTo = cutoffTime
        };

        await foreach (var instance in client.GetAllInstancesAsync(query))
        {
            var duration = DateTimeOffset.UtcNow - instance.CreatedAt;

            stuckWorkflows.Add(new StuckWorkflowInfo
            {
                InstanceId = instance.InstanceId,
                WorkflowType = instance.Name,
                StartedAt = instance.CreatedAt,
                Duration = duration,
                CurrentStep = instance.SerializedCustomStatus
            });
        }

        _logger.LogInformation("Found {Count} stuck workflows", stuckWorkflows.Count);

        return stuckWorkflows;
    }
}
