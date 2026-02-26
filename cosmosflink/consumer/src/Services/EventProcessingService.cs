using CosmosFlinkConsumer.Models;
using CosmosFlinkConsumer.Configuration;
using Microsoft.Extensions.Logging;
using System.Collections.Concurrent;
using System.Text.Json;

namespace CosmosFlinkConsumer.Services;

public class EventProcessingService
{
    private readonly ConsumerConfiguration _config;
    private readonly ILogger<EventProcessingService> _logger;
    private readonly ConcurrentQueue<ProcessedEvent> _eventQueue;
    private readonly ConcurrentDictionary<string, int> _customerEventCounts;
    private readonly ConcurrentDictionary<string, int> _regionEventCounts;
    private readonly Timer _statisticsTimer;
    
    private long _totalEventsProcessed;
    private long _totalBatchesProcessed;
    private long _totalProcessingTimeMs;
    private DateTime _startTime;
    private DateTime _lastEventTime;

    public EventProcessingService(ConsumerConfiguration config, ILogger<EventProcessingService> logger)
    {
        _config = config;
        _logger = logger;
        _eventQueue = new ConcurrentQueue<ProcessedEvent>();
        _customerEventCounts = new ConcurrentDictionary<string, int>();
        _regionEventCounts = new ConcurrentDictionary<string, int>();
        _startTime = DateTime.UtcNow;
        _lastEventTime = DateTime.UtcNow;

        // Setup periodic statistics logging
        _statisticsTimer = new Timer(LogStatistics, null, TimeSpan.FromSeconds(30), TimeSpan.FromSeconds(30));
        
        _logger.LogInformation("EventProcessingService initialized with identifier: {ConsumerIdentifier}", 
            _config.ConsumerIdentifier);
    }

    public async Task ProcessEventAsync(ProcessedEvent processedEvent)
    {
        var startTime = DateTime.UtcNow;
        
        try
        {
            // Validate event
            if (string.IsNullOrEmpty(processedEvent.Id))
            {
                _logger.LogWarning("Received event with empty ID");
                return;
            }

            // Enqueue for processing
            _eventQueue.Enqueue(processedEvent);
            
            // Update statistics
            Interlocked.Increment(ref _totalEventsProcessed);
            _lastEventTime = DateTime.UtcNow;
            
            // Update customer and region counters
            if (!string.IsNullOrEmpty(processedEvent.CustomerId))
            {
                _customerEventCounts.AddOrUpdate(processedEvent.CustomerId, 1, (key, value) => value + 1);
            }
            
            if (!string.IsNullOrEmpty(processedEvent.Region))
            {
                _regionEventCounts.AddOrUpdate(processedEvent.Region, 1, (key, value) => value + 1);
            }

            // Log event details if detailed logging is enabled
            if (_config.EnableDetailedLogging)
            {
                _logger.LogInformation("Processed event: ID={EventId}, Customer={CustomerId}, Region={Region}, Amount={Amount} {Currency}, Pipeline={Pipeline}",
                    processedEvent.Id, processedEvent.CustomerId, processedEvent.Region, 
                    processedEvent.Amount, processedEvent.Currency, processedEvent.Pipeline);
            }

            // Simulate processing time for demonstration
            await Task.Delay(10);
            
            var processingTime = (DateTime.UtcNow - startTime).TotalMilliseconds;
            Interlocked.Add(ref _totalProcessingTimeMs, (long)processingTime);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error processing event {EventId}", processedEvent.Id);
            throw;
        }
    }

    public async Task ProcessEventBatchAsync(EventBatch eventBatch)
    {
        var startTime = DateTime.UtcNow;
        
        try
        {
            _logger.LogInformation("Processing batch {BatchId} with {Count} events", 
                eventBatch.BatchId, eventBatch.Count);

            var tasks = eventBatch.Events.Select(ProcessEventAsync).ToList();
            await Task.WhenAll(tasks);
            
            Interlocked.Increment(ref _totalBatchesProcessed);
            
            var processingTime = (DateTime.UtcNow - startTime).TotalMilliseconds;
            _logger.LogInformation("Completed batch {BatchId} processing in {ProcessingTime:F2}ms", 
                eventBatch.BatchId, processingTime);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error processing event batch {BatchId}", eventBatch.BatchId);
            throw;
        }
    }

    public object GetStatistics()
    {
        var currentTime = DateTime.UtcNow;
        var uptime = currentTime - _startTime;
        var eventsPerSecond = _totalEventsProcessed > 0 ? _totalEventsProcessed / uptime.TotalSeconds : 0;
        var averageProcessingTime = _totalEventsProcessed > 0 ? (double)_totalProcessingTimeMs / _totalEventsProcessed : 0;

        return new
        {
            consumerIdentifier = _config.ConsumerIdentifier,
            startTime = _startTime,
            lastEventTime = _lastEventTime,
            uptime = new
            {
                days = uptime.Days,
                hours = uptime.Hours,
                minutes = uptime.Minutes,
                seconds = uptime.Seconds,
                totalSeconds = uptime.TotalSeconds
            },
            counters = new
            {
                totalEventsProcessed = _totalEventsProcessed,
                totalBatchesProcessed = _totalBatchesProcessed,
                queueSize = _eventQueue.Count,
                eventsPerSecond = Math.Round(eventsPerSecond, 2),
                averageProcessingTimeMs = Math.Round(averageProcessingTime, 2)
            },
            customerDistribution = _customerEventCounts.ToDictionary(kvp => kvp.Key, kvp => kvp.Value),
            regionDistribution = _regionEventCounts.ToDictionary(kvp => kvp.Key, kvp => kvp.Value),
            configuration = new
            {
                httpPort = _config.HttpPort,
                tcpPort = _config.TcpPort,
                serializationFormat = _config.SerializationFormat,
                maxBatchSize = _config.MaxBatchSize,
                enableDetailedLogging = _config.EnableDetailedLogging
            }
        };
    }

    private void LogStatistics(object? state)
    {
        var stats = GetStatistics();
        var statsJson = JsonSerializer.Serialize(stats, new JsonSerializerOptions 
        { 
            WriteIndented = true 
        });
        
        _logger.LogInformation("Consumer Statistics: {Statistics}", statsJson);
    }

    public void Dispose()
    {
        _statisticsTimer?.Dispose();
    }
}