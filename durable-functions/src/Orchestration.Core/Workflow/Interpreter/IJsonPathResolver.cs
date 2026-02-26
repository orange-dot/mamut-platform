using Orchestration.Core.Models;

namespace Orchestration.Core.Workflow.Interpreter;

/// <summary>
/// Resolves JSONPath expressions against workflow state.
/// </summary>
public interface IJsonPathResolver
{
    /// <summary>
    /// Resolves a JSONPath expression to a value.
    /// </summary>
    /// <param name="path">JSONPath expression (e.g., "$.input.deviceId").</param>
    /// <param name="state">Current workflow state.</param>
    /// <returns>The resolved value, or null if not found.</returns>
    object? Resolve(string path, WorkflowRuntimeState state);

    /// <summary>
    /// Resolves a JSONPath expression to a typed value.
    /// </summary>
    T? Resolve<T>(string path, WorkflowRuntimeState state);

    /// <summary>
    /// Sets a value at the specified JSONPath.
    /// </summary>
    /// <param name="path">JSONPath expression (e.g., "$.variables.result").</param>
    /// <param name="value">Value to set.</param>
    /// <param name="state">Current workflow state.</param>
    void SetValue(string path, object? value, WorkflowRuntimeState state);

    /// <summary>
    /// Resolves input object, replacing JSONPath references with actual values.
    /// </summary>
    /// <param name="input">Input object which may contain JSONPath references.</param>
    /// <param name="state">Current workflow state.</param>
    /// <returns>Resolved input object.</returns>
    object? ResolveInput(object? input, WorkflowRuntimeState state);
}
