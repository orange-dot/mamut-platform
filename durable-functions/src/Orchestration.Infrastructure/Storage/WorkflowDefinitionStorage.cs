using System.Collections.Concurrent;
using System.Text.Json;
using Azure.Storage.Blobs;
using Azure.Storage.Blobs.Models;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using Orchestration.Core.Workflow;
using Orchestration.Core.Workflow.StateTypes;

namespace Orchestration.Infrastructure.Storage;

/// <summary>
/// Blob storage implementation for workflow definitions with caching.
/// </summary>
public class WorkflowDefinitionStorage : IWorkflowDefinitionStorage
{
    private readonly BlobContainerClient? _containerClient;
    private readonly ILogger<WorkflowDefinitionStorage> _logger;
    private readonly ConcurrentDictionary<string, CachedDefinition> _cache = new();
    private readonly TimeSpan _cacheExpiration = TimeSpan.FromMinutes(5);

    // Built-in workflow definitions for development/testing
    private readonly Dictionary<string, WorkflowDefinition> _builtInDefinitions;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = true
    };

    public WorkflowDefinitionStorage(
        IConfiguration configuration,
        ILogger<WorkflowDefinitionStorage> logger)
    {
        _logger = logger;

        var connectionString = configuration["WorkflowStorageConnection"];
        var containerName = configuration["WorkflowStorageContainer"] ?? "workflow-definitions";

        if (!string.IsNullOrEmpty(connectionString) && connectionString != "UseDevelopmentStorage=true")
        {
            var blobServiceClient = new BlobServiceClient(connectionString);
            _containerClient = blobServiceClient.GetBlobContainerClient(containerName);
        }

        _builtInDefinitions = CreateBuiltInDefinitions();
    }

    /// <inheritdoc />
    public async Task<WorkflowDefinition> GetAsync(string workflowType, string? version = null)
    {
        var cacheKey = $"{workflowType}:{version ?? "latest"}";

        // Check cache
        if (_cache.TryGetValue(cacheKey, out var cached) && !cached.IsExpired)
        {
            _logger.LogDebug("Returning cached workflow definition {WorkflowType}", workflowType);
            return cached.Definition;
        }

        // Try blob storage first
        if (_containerClient != null)
        {
            try
            {
                var blobName = string.IsNullOrEmpty(version)
                    ? $"{workflowType}/latest.json"
                    : $"{workflowType}/{version}.json";

                var blobClient = _containerClient.GetBlobClient(blobName);
                if (await blobClient.ExistsAsync())
                {
                    var response = await blobClient.DownloadContentAsync();
                    var definition = JsonSerializer.Deserialize<WorkflowDefinition>(
                        response.Value.Content.ToString(),
                        JsonOptions);

                    if (definition != null)
                    {
                        _cache[cacheKey] = new CachedDefinition(definition, _cacheExpiration);
                        return definition;
                    }
                }
            }
            catch (Exception ex)
            {
                _logger.LogWarning(ex, "Failed to load workflow from blob storage, falling back to built-in");
            }
        }

        // Fall back to built-in definitions
        if (_builtInDefinitions.TryGetValue(workflowType, out var builtIn))
        {
            _cache[cacheKey] = new CachedDefinition(builtIn, _cacheExpiration);
            return builtIn;
        }

        throw new InvalidOperationException($"Workflow definition '{workflowType}' not found");
    }

    /// <inheritdoc />
    public async Task SaveAsync(WorkflowDefinition definition)
    {
        if (_containerClient == null)
        {
            _logger.LogWarning("Blob storage not configured, cannot save workflow definition");
            return;
        }

        await _containerClient.CreateIfNotExistsAsync();

        var json = JsonSerializer.Serialize(definition, JsonOptions);

        // Save versioned copy
        var versionedBlobName = $"{definition.Id}/{definition.Version}.json";
        var versionedBlobClient = _containerClient.GetBlobClient(versionedBlobName);
        await versionedBlobClient.UploadAsync(
            BinaryData.FromString(json),
            overwrite: true);

        // Update latest pointer
        var latestBlobName = $"{definition.Id}/latest.json";
        var latestBlobClient = _containerClient.GetBlobClient(latestBlobName);
        await latestBlobClient.UploadAsync(
            BinaryData.FromString(json),
            overwrite: true);

        // Invalidate cache
        _cache.TryRemove($"{definition.Id}:latest", out _);
        _cache.TryRemove($"{definition.Id}:{definition.Version}", out _);

        _logger.LogInformation(
            "Saved workflow definition {WorkflowId} version {Version}",
            definition.Id, definition.Version);
    }

    /// <inheritdoc />
    public async Task<IReadOnlyList<string>> ListVersionsAsync(string workflowType)
    {
        var versions = new List<string>();

        if (_containerClient != null)
        {
            try
            {
                if (await _containerClient.ExistsAsync())
                {
                    var prefix = $"{workflowType}/";
                    await foreach (var blob in _containerClient.GetBlobsAsync(prefix: prefix))
                    {
                        var fileName = blob.Name.Replace(prefix, "").Replace(".json", "");
                        if (fileName != "latest")
                        {
                            versions.Add(fileName);
                        }
                    }
                }
            }
            catch (Exception ex)
            {
                _logger.LogWarning(ex, "Failed to list versions for {WorkflowType}", workflowType);
            }
        }

        if (versions.Count == 0 && _builtInDefinitions.ContainsKey(workflowType))
        {
            versions.Add(_builtInDefinitions[workflowType].Version);
        }

        return versions;
    }

    /// <inheritdoc />
    public async Task<IReadOnlyList<string>> ListWorkflowTypesAsync()
    {
        var types = new HashSet<string>(_builtInDefinitions.Keys);

        if (_containerClient != null)
        {
            try
            {
                // Check if container exists first
                if (await _containerClient.ExistsAsync())
                {
                    await foreach (var blob in _containerClient.GetBlobsByHierarchyAsync(delimiter: "/"))
                    {
                        if (blob.IsPrefix)
                        {
                            types.Add(blob.Prefix.TrimEnd('/'));
                        }
                    }
                }
            }
            catch (Exception ex)
            {
                _logger.LogWarning(ex, "Failed to list workflow types from blob storage, returning built-in only");
            }
        }

        return types.ToList();
    }

    /// <inheritdoc />
    public async Task DeleteAsync(string workflowType, string version)
    {
        if (_containerClient == null)
        {
            return;
        }

        var blobName = $"{workflowType}/{version}.json";
        var blobClient = _containerClient.GetBlobClient(blobName);
        await blobClient.DeleteIfExistsAsync();

        _cache.TryRemove($"{workflowType}:{version}", out _);

        _logger.LogInformation(
            "Deleted workflow definition {WorkflowType} version {Version}",
            workflowType, version);
    }

    private static Dictionary<string, WorkflowDefinition> CreateBuiltInDefinitions()
    {
        return new Dictionary<string, WorkflowDefinition>
        {
            ["DeviceOnboarding"] = new WorkflowDefinition
            {
                Id = "DeviceOnboarding",
                Name = "Device Onboarding Workflow",
                Version = "1.0.0",
                Description = "Handles device onboarding with event accumulation",
                StartAt = "WaitForEvents",
                Config = new WorkflowConfiguration
                {
                    TimeoutSeconds = 3600,
                    CompensationState = "Compensate",
                    DefaultRetryPolicy = new RetryPolicy
                    {
                        MaxAttempts = 3,
                        InitialIntervalSeconds = 5,
                        BackoffCoefficient = 2.0
                    }
                },
                States = new Dictionary<string, WorkflowStateDefinition>
                {
                    ["WaitForEvents"] = new WaitStateDefinition
                    {
                        Comment = "Wait for enough events or timeout",
                        ExternalEvent = new ExternalEventWait
                        {
                            EventName = "NewEvent",
                            TimeoutSeconds = 300,
                            TimeoutNext = "GetAccumulatedEvents",
                            ResultPath = "$.variables.lastEvent"
                        },
                        Next = "CheckEventCount"
                    },
                    ["CheckEventCount"] = new TaskStateDefinition
                    {
                        Activity = "GetEntityEventsActivity",
                        Input = new Dictionary<string, object?>
                        {
                            ["entityId"] = "$.input.entityId"
                        },
                        ResultPath = "$.stepResults.events",
                        Next = "EvaluateEventCount"
                    },
                    ["EvaluateEventCount"] = new ChoiceStateDefinition
                    {
                        Choices = new List<ChoiceRule>
                        {
                            new()
                            {
                                Condition = new ComparisonCondition
                                {
                                    Variable = "$.stepResults.events.eventCount",
                                    ComparisonType = ComparisonType.GreaterThanOrEquals,
                                    Value = 3
                                },
                                Next = "GetAccumulatedEvents"
                            }
                        },
                        Default = "WaitForEvents"
                    },
                    ["GetAccumulatedEvents"] = new TaskStateDefinition
                    {
                        Activity = "GetEntityEventsActivity",
                        Input = new Dictionary<string, object?>
                        {
                            ["entityId"] = "$.input.entityId"
                        },
                        ResultPath = "$.stepResults.allEvents",
                        Next = "CreateRecord"
                    },
                    ["CreateRecord"] = new TaskStateDefinition
                    {
                        Activity = "CreateRecordActivity",
                        Input = new Dictionary<string, object?>
                        {
                            ["recordType"] = "Onboarding",
                            ["idempotencyKey"] = "$.input.idempotencyKey",
                            ["data"] = new Dictionary<string, object?>
                            {
                                ["entityId"] = "$.input.entityId",
                                ["events"] = "$.stepResults.allEvents.events"
                            }
                        },
                        ResultPath = "$.stepResults.record",
                        CompensateWith = "CompensateCreateRecordActivity",
                        Next = "ClearEntity",
                        Catch = new List<CatchDefinition>
                        {
                            new()
                            {
                                Errors = new List<string> { "States.ALL" },
                                ResultPath = "$.error",
                                Next = "Compensate"
                            }
                        }
                    },
                    ["ClearEntity"] = new TaskStateDefinition
                    {
                        Activity = "ClearEntityActivity",
                        Input = new Dictionary<string, object?>
                        {
                            ["entityId"] = "$.input.entityId"
                        },
                        Next = "Complete"
                    },
                    ["Complete"] = new SucceedStateDefinition
                    {
                        Output = new Dictionary<string, object?>
                        {
                            ["success"] = true,
                            ["recordId"] = "$.stepResults.record.recordId"
                        }
                    },
                    ["Compensate"] = new CompensationStateDefinition
                    {
                        ContinueOnError = true,
                        Steps = new List<CompensationStep>
                        {
                            new()
                            {
                                Name = "RollbackRecord",
                                Activity = "CompensateCreateRecordActivity",
                                Input = new Dictionary<string, object?>
                                {
                                    ["recordId"] = "$.stepResults.record.recordId",
                                    ["recordType"] = "Onboarding"
                                },
                                Condition = "$.stepResults.record.recordId"
                            }
                        },
                        FinalState = "Failed"
                    },
                    ["Failed"] = new FailStateDefinition
                    {
                        Error = "WorkflowFailed",
                        Cause = "Workflow failed and compensation was executed"
                    }
                }
            }
        };
    }

    private class CachedDefinition
    {
        public WorkflowDefinition Definition { get; }
        public DateTimeOffset ExpiresAt { get; }
        public bool IsExpired => DateTimeOffset.UtcNow > ExpiresAt;

        public CachedDefinition(WorkflowDefinition definition, TimeSpan expiration)
        {
            Definition = definition;
            ExpiresAt = DateTimeOffset.UtcNow.Add(expiration);
        }
    }
}
