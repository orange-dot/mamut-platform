using Microsoft.DurableTask;
using Orchestration.Core.Workflow.Interpreter;
using CoreRetryPolicy = Orchestration.Core.Workflow.RetryPolicy;

namespace Orchestration.Functions.Orchestrators;

/// <summary>
/// Implementation of IWorkflowExecutionContext that wraps the Durable Functions TaskOrchestrationContext.
/// </summary>
public sealed class DurableWorkflowExecutionContext : IWorkflowExecutionContext
{
    private readonly TaskOrchestrationContext _context;

    public DurableWorkflowExecutionContext(TaskOrchestrationContext context)
    {
        _context = context;
    }

    /// <inheritdoc />
    public DateTimeOffset CurrentUtcDateTime => _context.CurrentUtcDateTime;

    /// <inheritdoc />
    public string InstanceId => _context.InstanceId;

    /// <inheritdoc />
    public Guid NewGuid() => _context.NewGuid();

    /// <inheritdoc />
    public async Task<TResult> CallActivityAsync<TResult>(string activityName, object? input, CoreRetryPolicy? retry = null)
    {
        if (retry != null)
        {
            var options = new TaskOptions(CreateTaskRetryOptions(retry));
            return await _context.CallActivityAsync<TResult>(activityName, input, options);
        }
        return await _context.CallActivityAsync<TResult>(activityName, input);
    }

    /// <inheritdoc />
    public async Task CreateTimerAsync(TimeSpan duration)
    {
        await _context.CreateTimer(duration, CancellationToken.None);
    }

    /// <inheritdoc />
    public async Task CreateTimerAsync(DateTimeOffset fireAt)
    {
        await _context.CreateTimer(fireAt.UtcDateTime, CancellationToken.None);
    }

    /// <inheritdoc />
    public async Task<TResult> WaitForExternalEventAsync<TResult>(string eventName, TimeSpan? timeout = null)
    {
        if (timeout.HasValue)
        {
            using var cts = new CancellationTokenSource();
            var eventTask = _context.WaitForExternalEvent<TResult>(eventName);
            var timerTask = _context.CreateTimer(timeout.Value, cts.Token);

            var winner = await Task.WhenAny(eventTask, timerTask);

            if (winner == timerTask)
            {
                throw new TimeoutException($"Timeout waiting for external event '{eventName}'");
            }

            await cts.CancelAsync();
            return await eventTask;
        }

        return await _context.WaitForExternalEvent<TResult>(eventName);
    }

    /// <inheritdoc />
    public async Task<TResult[]> WhenAllAsync<TResult>(IEnumerable<Task<TResult>> tasks)
    {
        return await Task.WhenAll(tasks);
    }

    private static TaskRetryOptions CreateTaskRetryOptions(CoreRetryPolicy retry)
    {
        return new TaskRetryOptions(
            new RetryPolicy(
                retry.MaxAttempts,
                TimeSpan.FromSeconds(retry.InitialIntervalSeconds),
                retry.BackoffCoefficient,
                TimeSpan.FromSeconds(retry.MaxIntervalSeconds)));
    }
}
