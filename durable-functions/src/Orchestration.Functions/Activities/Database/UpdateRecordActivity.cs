using Microsoft.Azure.Functions.Worker;
using Microsoft.Extensions.Logging;
using Orchestration.Core.Contracts;

namespace Orchestration.Functions.Activities.Database;

/// <summary>
/// Input for updating a record.
/// </summary>
public sealed class UpdateRecordInput
{
    public required string RecordId { get; init; }
    public required string RecordType { get; init; }
    public required Dictionary<string, object?> Updates { get; init; }
    public string? IdempotencyKey { get; init; }
}

/// <summary>
/// Output from updating a record.
/// </summary>
public sealed class UpdateRecordOutput
{
    public required string RecordId { get; init; }
    public bool Success { get; init; }
    public DateTimeOffset UpdatedAt { get; init; }
    public Dictionary<string, object?>? PreviousValues { get; init; }
}

/// <summary>
/// Activity that updates an existing record.
/// </summary>
public class UpdateRecordActivity
{
    private readonly IWorkflowRepository _repository;
    private readonly ILogger<UpdateRecordActivity> _logger;

    public UpdateRecordActivity(IWorkflowRepository repository, ILogger<UpdateRecordActivity> logger)
    {
        _repository = repository;
        _logger = logger;
    }

    [Function(nameof(UpdateRecordActivity))]
    public async Task<UpdateRecordOutput> Run([ActivityTrigger] UpdateRecordInput input)
    {
        _logger.LogInformation(
            "Updating record {RecordId} of type {RecordType}.",
            input.RecordId, input.RecordType);

        // Check idempotency if key provided
        if (!string.IsNullOrEmpty(input.IdempotencyKey))
        {
            var existing = await _repository.GetIdempotencyRecordAsync<UpdateRecordOutput>(input.IdempotencyKey);
            if (existing != null)
            {
                _logger.LogInformation(
                    "Found existing update for idempotency key {IdempotencyKey}.",
                    input.IdempotencyKey);
                return existing;
            }
        }

        // Get current record for compensation support
        var currentRecord = await _repository.GetRecordAsync<Dictionary<string, object?>>(input.RecordId);
        if (currentRecord == null)
        {
            throw new InvalidOperationException($"Record {input.RecordId} not found.");
        }

        // Store previous values for potential rollback
        var previousValues = new Dictionary<string, object?>();
        foreach (var key in input.Updates.Keys)
        {
            if (currentRecord.TryGetValue(key, out var value))
            {
                previousValues[key] = value;
            }
        }

        // Apply updates
        foreach (var kvp in input.Updates)
        {
            currentRecord[kvp.Key] = kvp.Value;
        }
        currentRecord["updatedAt"] = DateTimeOffset.UtcNow;

        await _repository.UpdateRecordAsync(currentRecord);

        var result = new UpdateRecordOutput
        {
            RecordId = input.RecordId,
            Success = true,
            UpdatedAt = DateTimeOffset.UtcNow,
            PreviousValues = previousValues
        };

        // Store idempotency record if key provided
        if (!string.IsNullOrEmpty(input.IdempotencyKey))
        {
            await _repository.SaveIdempotencyRecordAsync(input.IdempotencyKey, result);
        }

        _logger.LogInformation("Updated record {RecordId}.", input.RecordId);

        return result;
    }
}
