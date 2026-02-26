using System.Text.Json;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;
using Orchestration.Core.Contracts;

namespace Orchestration.Infrastructure.Data.Repositories;

/// <summary>
/// Repository implementation for workflow-related data persistence.
/// </summary>
public class WorkflowRepository : IWorkflowRepository
{
    private readonly OrchestrationDbContext _context;
    private readonly ILogger<WorkflowRepository> _logger;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };

    public WorkflowRepository(OrchestrationDbContext context, ILogger<WorkflowRepository> logger)
    {
        _context = context;
        _logger = logger;
    }

    /// <inheritdoc />
    public async Task SaveIdempotencyRecordAsync(string key, object result, CancellationToken cancellationToken = default)
    {
        var json = JsonSerializer.Serialize(result, JsonOptions);

        var existing = await _context.IdempotencyRecords.FindAsync([key], cancellationToken);
        if (existing != null)
        {
            existing.ResultJson = json;
            existing.ExpiresAt = DateTimeOffset.UtcNow.AddDays(7);
        }
        else
        {
            _context.IdempotencyRecords.Add(new IdempotencyRecord
            {
                Key = key,
                ResultJson = json
            });
        }

        await _context.SaveChangesAsync(cancellationToken);
        _logger.LogDebug("Saved idempotency record for key {Key}", key);
    }

    /// <inheritdoc />
    public async Task<T?> GetIdempotencyRecordAsync<T>(string key, CancellationToken cancellationToken = default)
    {
        var record = await _context.IdempotencyRecords
            .AsNoTracking()
            .FirstOrDefaultAsync(r => r.Key == key && r.ExpiresAt > DateTimeOffset.UtcNow, cancellationToken);

        if (record == null)
            return default;

        return JsonSerializer.Deserialize<T>(record.ResultJson, JsonOptions);
    }

    /// <inheritdoc />
    public async Task<bool> IdempotencyRecordExistsAsync(string key, CancellationToken cancellationToken = default)
    {
        return await _context.IdempotencyRecords
            .AnyAsync(r => r.Key == key && r.ExpiresAt > DateTimeOffset.UtcNow, cancellationToken);
    }

    /// <inheritdoc />
    public async Task<T> CreateRecordAsync<T>(T record, string idempotencyKey, CancellationToken cancellationToken = default) where T : class
    {
        // This is a generic implementation - for specific record types,
        // you would add them to the appropriate DbSet
        if (record is OnboardingRecord onboardingRecord)
        {
            onboardingRecord.IdempotencyKey = idempotencyKey;
            _context.OnboardingRecords.Add(onboardingRecord);
            await _context.SaveChangesAsync(cancellationToken);
            _logger.LogInformation("Created OnboardingRecord {Id}", onboardingRecord.Id);
        }
        else if (record is Dictionary<string, object?> dict)
        {
            // Store as an onboarding record with JSON data
            var newRecord = new OnboardingRecord
            {
                EntityId = dict.TryGetValue("entityId", out var entityId)
                    ? entityId?.ToString() ?? "unknown"
                    : "unknown",
                IdempotencyKey = idempotencyKey,
                DataJson = JsonSerializer.Serialize(dict, JsonOptions)
            };
            _context.OnboardingRecords.Add(newRecord);
            await _context.SaveChangesAsync(cancellationToken);
            dict["id"] = newRecord.Id;
        }

        return record;
    }

    /// <inheritdoc />
    public async Task<T> UpdateRecordAsync<T>(T record, CancellationToken cancellationToken = default) where T : class
    {
        if (record is OnboardingRecord onboardingRecord)
        {
            var existing = await _context.OnboardingRecords.FindAsync([onboardingRecord.Id], cancellationToken);
            if (existing == null)
                throw new InvalidOperationException($"Record {onboardingRecord.Id} not found");

            existing.Status = onboardingRecord.Status;
            existing.DataJson = onboardingRecord.DataJson;
            existing.UpdatedAt = DateTimeOffset.UtcNow;
            await _context.SaveChangesAsync(cancellationToken);
            _logger.LogInformation("Updated OnboardingRecord {Id}", existing.Id);
        }
        else if (record is Dictionary<string, object?> dict && dict.TryGetValue("id", out var id))
        {
            var recordId = id?.ToString();
            if (string.IsNullOrEmpty(recordId))
                throw new InvalidOperationException("Record ID not found in dictionary");

            var existing = await _context.OnboardingRecords.FindAsync([recordId], cancellationToken);
            if (existing == null)
                throw new InvalidOperationException($"Record {recordId} not found");

            existing.DataJson = JsonSerializer.Serialize(dict, JsonOptions);
            existing.UpdatedAt = DateTimeOffset.UtcNow;
            await _context.SaveChangesAsync(cancellationToken);
        }

        return record;
    }

    /// <inheritdoc />
    public async Task<T?> GetRecordAsync<T>(string id, CancellationToken cancellationToken = default) where T : class
    {
        if (typeof(T) == typeof(OnboardingRecord))
        {
            var record = await _context.OnboardingRecords
                .AsNoTracking()
                .FirstOrDefaultAsync(r => r.Id == id, cancellationToken);
            return record as T;
        }

        if (typeof(T) == typeof(Dictionary<string, object?>))
        {
            var record = await _context.OnboardingRecords
                .AsNoTracking()
                .FirstOrDefaultAsync(r => r.Id == id, cancellationToken);

            if (record == null)
                return null;

            var dict = string.IsNullOrEmpty(record.DataJson)
                ? new Dictionary<string, object?>()
                : JsonSerializer.Deserialize<Dictionary<string, object?>>(record.DataJson, JsonOptions)
                  ?? new Dictionary<string, object?>();

            dict["id"] = record.Id;
            dict["entityId"] = record.EntityId;
            dict["status"] = record.Status;
            dict["createdAt"] = record.CreatedAt;
            dict["updatedAt"] = record.UpdatedAt;

            return dict as T;
        }

        return null;
    }

    /// <inheritdoc />
    public async Task DeleteRecordAsync<T>(string id, CancellationToken cancellationToken = default) where T : class
    {
        if (typeof(T) == typeof(OnboardingRecord) || typeof(T) == typeof(Dictionary<string, object?>))
        {
            var record = await _context.OnboardingRecords.FindAsync([id], cancellationToken);
            if (record != null)
            {
                _context.OnboardingRecords.Remove(record);
                await _context.SaveChangesAsync(cancellationToken);
                _logger.LogInformation("Deleted OnboardingRecord {Id}", id);
            }
        }
    }
}
