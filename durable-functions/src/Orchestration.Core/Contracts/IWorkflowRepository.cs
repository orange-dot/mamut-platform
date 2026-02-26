using Orchestration.Core.Models;

namespace Orchestration.Core.Contracts;

/// <summary>
/// Repository for persisting workflow-related data.
/// </summary>
public interface IWorkflowRepository
{
    /// <summary>
    /// Saves an idempotency record to prevent duplicate processing.
    /// </summary>
    Task SaveIdempotencyRecordAsync(string key, object result, CancellationToken cancellationToken = default);

    /// <summary>
    /// Retrieves an existing idempotency record.
    /// </summary>
    Task<T?> GetIdempotencyRecordAsync<T>(string key, CancellationToken cancellationToken = default);

    /// <summary>
    /// Checks if an idempotency record exists.
    /// </summary>
    Task<bool> IdempotencyRecordExistsAsync(string key, CancellationToken cancellationToken = default);

    /// <summary>
    /// Creates a new record in the database.
    /// </summary>
    Task<T> CreateRecordAsync<T>(T record, string idempotencyKey, CancellationToken cancellationToken = default) where T : class;

    /// <summary>
    /// Updates an existing record.
    /// </summary>
    Task<T> UpdateRecordAsync<T>(T record, CancellationToken cancellationToken = default) where T : class;

    /// <summary>
    /// Retrieves a record by its ID.
    /// </summary>
    Task<T?> GetRecordAsync<T>(string id, CancellationToken cancellationToken = default) where T : class;

    /// <summary>
    /// Deletes a record (for compensation).
    /// </summary>
    Task DeleteRecordAsync<T>(string id, CancellationToken cancellationToken = default) where T : class;
}
