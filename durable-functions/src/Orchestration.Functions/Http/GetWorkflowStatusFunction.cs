using System.Net;
using Microsoft.Azure.Functions.Worker;
using Microsoft.Azure.Functions.Worker.Http;
using Microsoft.DurableTask;
using Microsoft.DurableTask.Client;
using Microsoft.Extensions.Logging;

namespace Orchestration.Functions.Http;

/// <summary>
/// Response for workflow status.
/// </summary>
public sealed class WorkflowStatusResponse
{
    public required string InstanceId { get; init; }
    public required string Status { get; init; }
    public string? WorkflowType { get; init; }
    public DateTimeOffset CreatedAt { get; init; }
    public DateTimeOffset LastUpdatedAt { get; init; }
    public object? Input { get; init; }
    public object? Output { get; init; }
    public object? CustomStatus { get; init; }
    public string? FailureDetails { get; init; }
}

/// <summary>
/// HTTP endpoint to get workflow status.
/// </summary>
public class GetWorkflowStatusFunction
{
    private readonly ILogger<GetWorkflowStatusFunction> _logger;

    public GetWorkflowStatusFunction(ILogger<GetWorkflowStatusFunction> logger)
    {
        _logger = logger;
    }

    [Function("GetWorkflowStatus")]
    public async Task<HttpResponseData> Run(
        [HttpTrigger(AuthorizationLevel.Anonymous, "get", Route = "workflows/{instanceId}")]
        HttpRequestData req,
        [DurableClient] DurableTaskClient client,
        string instanceId)
    {
        _logger.LogInformation("GetWorkflowStatus request for {InstanceId}", instanceId);

        var metadata = await client.GetInstanceAsync(instanceId, getInputsAndOutputs: true);

        if (metadata == null)
        {
            var notFoundResponse = req.CreateResponse(HttpStatusCode.NotFound);
            await notFoundResponse.WriteAsJsonAsync(new
            {
                error = "Workflow not found",
                instanceId
            });
            return notFoundResponse;
        }

        var response = req.CreateResponse(HttpStatusCode.OK);
        await response.WriteAsJsonAsync(new WorkflowStatusResponse
        {
            InstanceId = metadata.InstanceId,
            Status = metadata.RuntimeStatus.ToString(),
            WorkflowType = metadata.Name,
            CreatedAt = metadata.CreatedAt,
            LastUpdatedAt = metadata.LastUpdatedAt,
            Input = metadata.SerializedInput,
            Output = metadata.SerializedOutput,
            CustomStatus = metadata.SerializedCustomStatus,
            FailureDetails = metadata.FailureDetails?.ErrorMessage
        });

        return response;
    }

    [Function("ListWorkflows")]
    public async Task<HttpResponseData> ListWorkflows(
        [HttpTrigger(AuthorizationLevel.Anonymous, "get", Route = "workflows")]
        HttpRequestData req,
        [DurableClient] DurableTaskClient client)
    {
        _logger.LogInformation("ListWorkflows request");

        var statusFilter = req.Query["status"];
        var pageSize = int.TryParse(req.Query["pageSize"], out var ps) ? Math.Min(ps, 100) : 20;

        var query = new OrchestrationQuery { PageSize = pageSize };

        if (!string.IsNullOrEmpty(statusFilter) &&
            Enum.TryParse<OrchestrationRuntimeStatus>(statusFilter, true, out var status))
        {
            query = query with { Statuses = [status] };
        }

        var workflows = new List<object>();
        await foreach (var instance in client.GetAllInstancesAsync(query))
        {
            workflows.Add(new
            {
                instance.InstanceId,
                Status = instance.RuntimeStatus.ToString(),
                WorkflowType = instance.Name,
                instance.CreatedAt,
                instance.LastUpdatedAt
            });

            if (workflows.Count >= pageSize)
                break;
        }

        var response = req.CreateResponse(HttpStatusCode.OK);
        await response.WriteAsJsonAsync(new
        {
            count = workflows.Count,
            workflows
        });

        return response;
    }
}
