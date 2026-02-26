using System.Text.Json;
using Microsoft.Azure.Functions.Worker;
using Microsoft.Extensions.Logging;
using Orchestration.Core.Contracts;

namespace Orchestration.Functions.Activities.Database;

/// <summary>
/// Input for creating a record.
/// </summary>
public sealed class CreateRecordInput
{
    public required string RecordType { get; init; }
    public required string IdempotencyKey { get; init; }
    public required Dictionary<string, object?> Data { get; init; }
}

/// <summary>
/// Output from creating a record.
/// </summary>
public sealed record CreateRecordOutput
{
    public required string RecordId { get; init; }
    public required string RecordType { get; init; }
    public bool WasExisting { get; init; }
    public DateTimeOffset CreatedAt { get; init; }
}

/// <summary>
/// Activity that creates a new record with idempotency support.
/// </summary>
public class CreateRecordActivity
{
    private readonly IWorkflowRepository _repository;
    private readonly ILogger<CreateRecordActivity> _logger;

    public CreateRecordActivity(IWorkflowRepository repository, ILogger<CreateRecordActivity> logger)
    {
        _repository = repository;
        _logger = logger;
    }

    [Function(nameof(CreateRecordActivity))]
    public async Task<CreateRecordOutput> Run([ActivityTrigger] CreateRecordInput input)
    {
        _logger.LogInformation(
            "Creating record of type {RecordType} with idempotency key {IdempotencyKey}.",
            input.RecordType, input.IdempotencyKey);

        // Check for existing idempotent result
        var existing = await _repository.GetIdempotencyRecordAsync<CreateRecordOutput>(input.IdempotencyKey);
        if (existing != null)
        {
            _logger.LogInformation(
                "Found existing record for idempotency key {IdempotencyKey}. Returning cached result.",
                input.IdempotencyKey);
            return existing with { WasExisting = true };
        }

        // Create the record
        var recordId = Guid.NewGuid().ToString();
        var record = new Dictionary<string, object?>(input.Data)
        {
            ["id"] = recordId,
            ["recordType"] = input.RecordType,
            ["createdAt"] = DateTimeOffset.UtcNow,
            ["idempotencyKey"] = input.IdempotencyKey
        };

        await _repository.CreateRecordAsync(record, input.IdempotencyKey);

        var result = new CreateRecordOutput
        {
            RecordId = recordId,
            RecordType = input.RecordType,
            WasExisting = false,
            CreatedAt = DateTimeOffset.UtcNow
        };

        // Store idempotency record
        await _repository.SaveIdempotencyRecordAsync(input.IdempotencyKey, result);

        _logger.LogInformation(
            "Created record {RecordId} of type {RecordType}.",
            recordId, input.RecordType);

        return result;
    }
}
