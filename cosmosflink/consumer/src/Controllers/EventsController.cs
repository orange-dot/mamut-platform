using Microsoft.AspNetCore.Mvc;
using CosmosFlinkConsumer.Models;
using CosmosFlinkConsumer.Services;
using Microsoft.Extensions.Logging;
using System.Text.Json;

namespace CosmosFlinkConsumer.Controllers;

[ApiController]
[Route("api/[controller]")]
public class EventsController : ControllerBase
{
    private readonly EventProcessingService _eventProcessingService;
    private readonly ILogger<EventsController> _logger;

    public EventsController(EventProcessingService eventProcessingService, ILogger<EventsController> logger)
    {
        _eventProcessingService = eventProcessingService;
        _logger = logger;
    }

    [HttpPost]
    public async Task<IActionResult> ReceiveEvent([FromBody] ProcessedEvent processedEvent)
    {
        try
        {
            if (processedEvent == null)
            {
                _logger.LogWarning("Received null event");
                return BadRequest("Event cannot be null");
            }

            await _eventProcessingService.ProcessEventAsync(processedEvent);
            
            _logger.LogDebug("Successfully processed event {EventId} from customer {CustomerId}", 
                processedEvent.Id, processedEvent.CustomerId);
            
            return Ok(new { status = "processed", eventId = processedEvent.Id });
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error processing event");
            return StatusCode(500, new { error = "Internal server error", message = ex.Message });
        }
    }

    [HttpPost("batch")]
    public async Task<IActionResult> ReceiveEventBatch([FromBody] EventBatch eventBatch)
    {
        try
        {
            if (eventBatch == null || !eventBatch.Events.Any())
            {
                _logger.LogWarning("Received empty event batch");
                return BadRequest("Event batch cannot be null or empty");
            }

            await _eventProcessingService.ProcessEventBatchAsync(eventBatch);
            
            _logger.LogInformation("Successfully processed event batch {BatchId} with {Count} events", 
                eventBatch.BatchId, eventBatch.Count);
            
            return Ok(new 
            { 
                status = "processed", 
                batchId = eventBatch.BatchId, 
                count = eventBatch.Count,
                processedAt = DateTime.UtcNow
            });
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error processing event batch");
            return StatusCode(500, new { error = "Internal server error", message = ex.Message });
        }
    }

    [HttpGet("health")]
    public IActionResult Health()
    {
        var stats = _eventProcessingService.GetStatistics();
        return Ok(new
        {
            status = "healthy",
            timestamp = DateTime.UtcNow,
            statistics = stats
        });
    }

    [HttpGet("statistics")]
    public IActionResult GetStatistics()
    {
        var stats = _eventProcessingService.GetStatistics();
        return Ok(stats);
    }
}