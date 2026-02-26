using Microsoft.Azure.Functions.Worker;
using Microsoft.DurableTask;
using Microsoft.DurableTask.Client;
using Microsoft.Extensions.Logging;

namespace Orchestration.Functions.Orchestrators;

/// <summary>
/// Timer-triggered function that monitors for stuck workflows.
/// Runs every 15 minutes to check for workflows exceeding the stuck threshold.
/// </summary>
public class StuckWorkflowMonitorFunction
{
    private readonly ILogger<StuckWorkflowMonitorFunction> _logger;
    private static readonly TimeSpan StuckThreshold = TimeSpan.FromHours(50);

    public StuckWorkflowMonitorFunction(ILogger<StuckWorkflowMonitorFunction> logger)
    {
        _logger = logger;
    }

    [Function("StuckWorkflowMonitor")]
    public async Task Run(
        [TimerTrigger("0 */15 * * * *")] object timer,
        [DurableClient] DurableTaskClient client)
    {
        _logger.LogInformation(
            "StuckWorkflowMonitor executing at {Time}",
            DateTimeOffset.UtcNow);

        var stuckWorkflows = new List<StuckWorkflowInfo>();
        var cutoffTime = DateTimeOffset.UtcNow - StuckThreshold;

        try
        {
            // Query running orchestrations
            var query = new OrchestrationQuery
            {
                Statuses = [
                    OrchestrationRuntimeStatus.Running,
                    OrchestrationRuntimeStatus.Pending
                ],
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

                _logger.LogWarning(
                    "Stuck workflow detected: {InstanceId} ({WorkflowType}) running for {Duration:g}",
                    instance.InstanceId, instance.Name, duration);
            }

            if (stuckWorkflows.Count > 0)
            {
                _logger.LogError(
                    "ALERT: {Count} stuck workflows detected (threshold: {Threshold:g})",
                    stuckWorkflows.Count, StuckThreshold);

                // Log details for Application Insights tracking
                foreach (var workflow in stuckWorkflows)
                {
                    _logger.LogError(
                        "Stuck workflow: InstanceId={InstanceId}, Type={WorkflowType}, " +
                        "StartedAt={StartedAt}, Duration={Duration:g}",
                        workflow.InstanceId,
                        workflow.WorkflowType,
                        workflow.StartedAt,
                        workflow.Duration);
                }
            }
            else
            {
                _logger.LogInformation("No stuck workflows detected");
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error checking for stuck workflows");
        }
    }
}
