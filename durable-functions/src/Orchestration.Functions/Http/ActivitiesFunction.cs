using System.Net;
using System.Text.Json;
using Microsoft.Azure.Functions.Worker;
using Microsoft.Azure.Functions.Worker.Http;
using Microsoft.Extensions.Logging;
using Orchestration.Functions.Activities.Registry;

namespace Orchestration.Functions.Http;

/// <summary>
/// HTTP endpoints for browsing available activities.
/// </summary>
public sealed class ActivitiesFunction
{
    private readonly IActivityRegistry _activityRegistry;
    private readonly ILogger<ActivitiesFunction> _logger;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = true
    };

    public ActivitiesFunction(IActivityRegistry activityRegistry, ILogger<ActivitiesFunction> logger)
    {
        _activityRegistry = activityRegistry;
        _logger = logger;
    }

    /// <summary>
    /// Lists all registered activities.
    /// GET /api/activities
    /// </summary>
    [Function("ListActivities")]
    public async Task<HttpResponseData> ListActivities(
        [HttpTrigger(AuthorizationLevel.Anonymous, "get", Route = "activities")]
        HttpRequestData req)
    {
        _logger.LogInformation("Listing all registered activities");

        var activities = _activityRegistry.GetAll();

        var response = new ActivityListResponse
        {
            Count = activities.Count,
            Activities = activities.Select(a => new ActivitySummary
            {
                Name = a.Name,
                Description = a.Description,
                SupportsCompensation = a.SupportsCompensation,
                Tags = a.Tags?.ToList() ?? []
            }).ToList()
        };

        var httpResponse = req.CreateResponse(HttpStatusCode.OK);
        httpResponse.Headers.Add("Content-Type", "application/json");
        await httpResponse.WriteStringAsync(JsonSerializer.Serialize(response, JsonOptions));
        return httpResponse;
    }

    /// <summary>
    /// Gets details about a specific activity.
    /// GET /api/activities/{activityName}
    /// </summary>
    [Function("GetActivity")]
    public async Task<HttpResponseData> GetActivity(
        [HttpTrigger(AuthorizationLevel.Anonymous, "get", Route = "activities/{activityName}")]
        HttpRequestData req,
        string activityName)
    {
        _logger.LogInformation("Getting activity details for {ActivityName}", activityName);

        var metadata = _activityRegistry.GetMetadata(activityName);

        if (metadata == null)
        {
            var notFoundResponse = req.CreateResponse(HttpStatusCode.NotFound);
            notFoundResponse.Headers.Add("Content-Type", "application/json");
            await notFoundResponse.WriteStringAsync(JsonSerializer.Serialize(new { error = $"Activity '{activityName}' not found" }, JsonOptions));
            return notFoundResponse;
        }

        var detail = new ActivityDetail
        {
            Name = metadata.Name,
            Description = metadata.Description,
            SupportsCompensation = metadata.SupportsCompensation,
            CompensatingActivity = metadata.CompensatingActivity,
            Tags = metadata.Tags?.ToList() ?? [],
            InputType = metadata.InputType?.Name,
            OutputType = metadata.OutputType?.Name
        };

        var response = req.CreateResponse(HttpStatusCode.OK);
        response.Headers.Add("Content-Type", "application/json");
        await response.WriteStringAsync(JsonSerializer.Serialize(detail, JsonOptions));
        return response;
    }
}

public sealed class ActivityListResponse
{
    public int Count { get; set; }
    public List<ActivitySummary> Activities { get; set; } = [];
}

public sealed class ActivitySummary
{
    public required string Name { get; set; }
    public string? Description { get; set; }
    public bool SupportsCompensation { get; set; }
    public List<string> Tags { get; set; } = [];
}

public sealed class ActivityDetail
{
    public required string Name { get; set; }
    public string? Description { get; set; }
    public bool SupportsCompensation { get; set; }
    public string? CompensatingActivity { get; set; }
    public List<string> Tags { get; set; } = [];
    public string? InputType { get; set; }
    public string? OutputType { get; set; }
}
