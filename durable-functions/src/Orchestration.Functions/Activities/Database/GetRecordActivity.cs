using Microsoft.Azure.Functions.Worker;
using Microsoft.Extensions.Logging;
using Orchestration.Core.Contracts;

namespace Orchestration.Functions.Activities.Database;

/// <summary>
/// Input for getting a record.
/// </summary>
public sealed class GetRecordInput
{
    public required string RecordId { get; init; }
    public required string RecordType { get; init; }
}

/// <summary>
/// Output from getting a record.
/// </summary>
public sealed class GetRecordOutput
{
    public string? RecordId { get; init; }
    public bool Found { get; init; }
    public Dictionary<string, object?>? Data { get; init; }
}

/// <summary>
/// Activity that retrieves a record from the database.
/// </summary>
public class GetRecordActivity
{
    private readonly IWorkflowRepository _repository;
    private readonly ILogger<GetRecordActivity> _logger;

    public GetRecordActivity(IWorkflowRepository repository, ILogger<GetRecordActivity> logger)
    {
        _repository = repository;
        _logger = logger;
    }

    [Function(nameof(GetRecordActivity))]
    public async Task<GetRecordOutput> Run([ActivityTrigger] GetRecordInput input)
    {
        _logger.LogInformation(
            "Getting record {RecordId} of type {RecordType}.",
            input.RecordId, input.RecordType);

        var record = await _repository.GetRecordAsync<Dictionary<string, object?>>(input.RecordId);

        if (record == null)
        {
            _logger.LogInformation("Record {RecordId} not found.", input.RecordId);
            return new GetRecordOutput
            {
                RecordId = input.RecordId,
                Found = false
            };
        }

        return new GetRecordOutput
        {
            RecordId = input.RecordId,
            Found = true,
            Data = record
        };
    }
}
