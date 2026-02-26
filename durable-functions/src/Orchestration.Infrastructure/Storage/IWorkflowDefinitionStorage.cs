using Orchestration.Core.Workflow;

namespace Orchestration.Infrastructure.Storage;

/// <summary>
/// Storage interface for workflow definitions.
/// </summary>
public interface IWorkflowDefinitionStorage
{
    /// <summary>
    /// Gets a workflow definition by type and optional version.
    /// </summary>
    Task<WorkflowDefinition> GetAsync(string workflowType, string? version = null);

    /// <summary>
    /// Saves a workflow definition.
    /// </summary>
    Task SaveAsync(WorkflowDefinition definition);

    /// <summary>
    /// Lists all versions of a workflow type.
    /// </summary>
    Task<IReadOnlyList<string>> ListVersionsAsync(string workflowType);

    /// <summary>
    /// Lists all workflow types.
    /// </summary>
    Task<IReadOnlyList<string>> ListWorkflowTypesAsync();

    /// <summary>
    /// Deletes a specific version of a workflow.
    /// </summary>
    Task DeleteAsync(string workflowType, string version);
}
