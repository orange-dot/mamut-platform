using System.Net;
using System.Text.Json;
using Microsoft.Azure.Functions.Worker;
using Microsoft.Azure.Functions.Worker.Http;
using Microsoft.DurableTask;
using Microsoft.DurableTask.Client;
using Microsoft.Extensions.Logging;

namespace Orchestration.Functions.Http;

/// <summary>
/// HTTP endpoint to raise events to running workflows.
/// </summary>
public class RaiseEventFunction
{
    private readonly ILogger<RaiseEventFunction> _logger;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };

    public RaiseEventFunction(ILogger<RaiseEventFunction> logger)
    {
        _logger = logger;
    }

    [Function("RaiseEvent")]
    public async Task<HttpResponseData> Run(
        [HttpTrigger(AuthorizationLevel.Anonymous, "post", Route = "workflows/{instanceId}/events/{eventName}")]
        HttpRequestData req,
        [DurableClient] DurableTaskClient client,
        string instanceId,
        string eventName)
    {
        _logger.LogInformation(
            "RaiseEvent request for {InstanceId}, event: {EventName}",
            instanceId, eventName);

        // Check if instance exists
        var metadata = await client.GetInstanceAsync(instanceId);
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

        // Check if instance is in a state that can receive events
        if (metadata.RuntimeStatus != OrchestrationRuntimeStatus.Running &&
            metadata.RuntimeStatus != OrchestrationRuntimeStatus.Pending)
        {
            var badRequestResponse = req.CreateResponse(HttpStatusCode.BadRequest);
            await badRequestResponse.WriteAsJsonAsync(new
            {
                error = "Workflow is not in a state that can receive events",
                instanceId,
                currentStatus = metadata.RuntimeStatus.ToString()
            });
            return badRequestResponse;
        }

        // Parse event data from body
        object? eventData = null;
        var body = await req.ReadAsStringAsync();
        if (!string.IsNullOrWhiteSpace(body))
        {
            try
            {
                eventData = JsonSerializer.Deserialize<object>(body, JsonOptions);
            }
            catch (JsonException)
            {
                // Use raw string if not valid JSON
                eventData = body;
            }
        }

        try
        {
            await client.RaiseEventAsync(instanceId, eventName, eventData);

            _logger.LogInformation(
                "Raised event {EventName} to workflow {InstanceId}",
                eventName, instanceId);

            var response = req.CreateResponse(HttpStatusCode.Accepted);
            await response.WriteAsJsonAsync(new
            {
                message = "Event raised successfully",
                instanceId,
                eventName,
                raisedAt = DateTimeOffset.UtcNow
            });
            return response;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to raise event {EventName} to {InstanceId}", eventName, instanceId);

            var errorResponse = req.CreateResponse(HttpStatusCode.InternalServerError);
            await errorResponse.WriteAsJsonAsync(new
            {
                error = "Failed to raise event",
                details = ex.Message
            });
            return errorResponse;
        }
    }

    [Function("TerminateWorkflow")]
    public async Task<HttpResponseData> TerminateWorkflow(
        [HttpTrigger(AuthorizationLevel.Function, "post", Route = "workflows/{instanceId}/terminate")]
        HttpRequestData req,
        [DurableClient] DurableTaskClient client,
        string instanceId)
    {
        _logger.LogInformation("TerminateWorkflow request for {InstanceId}", instanceId);

        var metadata = await client.GetInstanceAsync(instanceId);
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

        var body = await req.ReadAsStringAsync();
        var reason = string.IsNullOrWhiteSpace(body) ? "Terminated via API" : body;

        try
        {
            await client.TerminateInstanceAsync(instanceId, reason);

            _logger.LogInformation("Terminated workflow {InstanceId}", instanceId);

            var response = req.CreateResponse(HttpStatusCode.OK);
            await response.WriteAsJsonAsync(new
            {
                message = "Workflow terminated",
                instanceId,
                reason,
                terminatedAt = DateTimeOffset.UtcNow
            });
            return response;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to terminate workflow {InstanceId}", instanceId);

            var errorResponse = req.CreateResponse(HttpStatusCode.InternalServerError);
            await errorResponse.WriteAsJsonAsync(new
            {
                error = "Failed to terminate workflow",
                details = ex.Message
            });
            return errorResponse;
        }
    }
}
