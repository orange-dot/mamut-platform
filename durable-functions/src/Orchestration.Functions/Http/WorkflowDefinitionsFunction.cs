using System.Net;
using Microsoft.Azure.Functions.Worker;
using Microsoft.Azure.Functions.Worker.Http;
using Microsoft.Extensions.Logging;
using Orchestration.Infrastructure.Storage;

namespace Orchestration.Functions.Http;

/// <summary>
/// Response for a workflow definition summary.
/// </summary>
public sealed class WorkflowDefinitionSummary
{
    public required string Id { get; init; }
    public required string Name { get; init; }
    public required string Version { get; init; }
    public string? Description { get; init; }
    public int StateCount { get; init; }
}

/// <summary>
/// HTTP endpoints for workflow definitions.
/// </summary>
public class WorkflowDefinitionsFunction
{
    private readonly IWorkflowDefinitionStorage _definitionStorage;
    private readonly ILogger<WorkflowDefinitionsFunction> _logger;

    public WorkflowDefinitionsFunction(
        IWorkflowDefinitionStorage definitionStorage,
        ILogger<WorkflowDefinitionsFunction> logger)
    {
        _definitionStorage = definitionStorage;
        _logger = logger;
    }

    [Function("ListWorkflowDefinitions")]
    public async Task<HttpResponseData> ListDefinitions(
        [HttpTrigger(AuthorizationLevel.Anonymous, "get", Route = "definitions")]
        HttpRequestData req)
    {
        _logger.LogInformation("ListWorkflowDefinitions request");

        var types = await _definitionStorage.ListWorkflowTypesAsync();
        var definitions = new List<WorkflowDefinitionSummary>();

        foreach (var type in types)
        {
            try
            {
                var definition = await _definitionStorage.GetAsync(type);
                definitions.Add(new WorkflowDefinitionSummary
                {
                    Id = definition.Id,
                    Name = definition.Name,
                    Version = definition.Version,
                    Description = definition.Description,
                    StateCount = definition.States?.Count ?? 0
                });
            }
            catch (Exception ex)
            {
                _logger.LogWarning(ex, "Failed to load definition {Type}", type);
            }
        }

        var response = req.CreateResponse(HttpStatusCode.OK);
        await response.WriteAsJsonAsync(new
        {
            count = definitions.Count,
            definitions
        });

        return response;
    }

    [Function("GetWorkflowDefinition")]
    public async Task<HttpResponseData> GetDefinition(
        [HttpTrigger(AuthorizationLevel.Anonymous, "get", Route = "definitions/{definitionId}")]
        HttpRequestData req,
        string definitionId)
    {
        _logger.LogInformation("GetWorkflowDefinition request for {DefinitionId}", definitionId);

        var version = req.Query["version"];

        try
        {
            var definition = await _definitionStorage.GetAsync(definitionId, version);

            var response = req.CreateResponse(HttpStatusCode.OK);
            await response.WriteAsJsonAsync(definition);
            return response;
        }
        catch (InvalidOperationException)
        {
            var notFoundResponse = req.CreateResponse(HttpStatusCode.NotFound);
            await notFoundResponse.WriteAsJsonAsync(new
            {
                error = "Definition not found",
                definitionId
            });
            return notFoundResponse;
        }
    }

    [Function("ListDefinitionVersions")]
    public async Task<HttpResponseData> ListVersions(
        [HttpTrigger(AuthorizationLevel.Anonymous, "get", Route = "definitions/{definitionId}/versions")]
        HttpRequestData req,
        string definitionId)
    {
        _logger.LogInformation("ListDefinitionVersions request for {DefinitionId}", definitionId);

        var versions = await _definitionStorage.ListVersionsAsync(definitionId);

        var response = req.CreateResponse(HttpStatusCode.OK);
        await response.WriteAsJsonAsync(new
        {
            definitionId,
            versions
        });

        return response;
    }
}
