using Microsoft.Azure.Functions.Worker;
using Microsoft.Extensions.Logging;
using Orchestration.Core.Workflow;
using Orchestration.Infrastructure.Storage;

namespace Orchestration.Functions.Activities.Workflow;

/// <summary>
/// Input for loading a workflow definition.
/// </summary>
public sealed class LoadWorkflowDefinitionInput
{
    public required string WorkflowType { get; init; }
    public string? Version { get; init; }
}

/// <summary>
/// Activity that loads a workflow definition from storage.
/// </summary>
public class LoadWorkflowDefinitionActivity
{
    private readonly IWorkflowDefinitionStorage _storage;
    private readonly ILogger<LoadWorkflowDefinitionActivity> _logger;

    public LoadWorkflowDefinitionActivity(
        IWorkflowDefinitionStorage storage,
        ILogger<LoadWorkflowDefinitionActivity> logger)
    {
        _storage = storage;
        _logger = logger;
    }

    [Function(nameof(LoadWorkflowDefinitionActivity))]
    public async Task<WorkflowDefinition> Run([ActivityTrigger] LoadWorkflowDefinitionInput input)
    {
        _logger.LogInformation(
            "Loading workflow definition: {WorkflowType} (version: {Version})",
            input.WorkflowType, input.Version ?? "latest");

        var definition = await _storage.GetAsync(input.WorkflowType, input.Version);

        _logger.LogInformation(
            "Loaded workflow definition: {WorkflowId} v{Version}",
            definition.Id, definition.Version);

        return definition;
    }
}
