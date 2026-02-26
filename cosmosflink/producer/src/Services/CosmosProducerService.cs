using Microsoft.Azure.Cosmos;
using CosmosFlinkProducer.Models;
using CosmosFlinkProducer.Configuration;
using Microsoft.Extensions.Logging;
using System.Text.Json;

namespace CosmosFlinkProducer.Services;

public class EventGenerator
{
    private readonly Random _random = new();
    private readonly string[] _customerIds = { "CUST001", "CUST002", "CUST003", "CUST004", "CUST005", 
                                              "CUST006", "CUST007", "CUST008", "CUST009", "CUST010" };
    private readonly string[] _currencies = { "USD", "EUR", "GBP", "JPY", "CAD" };
    private readonly string[] _productCategories = { "Electronics", "Clothing", "Books", "Home", "Sports" };
    private readonly string[] _productNames = { 
        "Laptop", "Smartphone", "Headphones", "T-Shirt", "Jeans", "Novel", "Cookbook", 
        "Coffee Maker", "Desk Chair", "Running Shoes", "Basketball", "Yoga Mat" 
    };

    public OrderCreated GenerateOrder(string sourceIdentifier, string region)
    {
        var customerId = _customerIds[_random.Next(_customerIds.Length)];
        var currency = _currencies[_random.Next(_currencies.Length)];
        var itemCount = _random.Next(1, 5);
        
        var orderItems = new List<OrderItem>();
        decimal totalAmount = 0;
        
        for (int i = 0; i < itemCount; i++)
        {
            var productName = _productNames[_random.Next(_productNames.Length)];
            var category = _productCategories[_random.Next(_productCategories.Length)];
            var quantity = _random.Next(1, 4);
            var unitPrice = Math.Round((decimal)(_random.NextDouble() * 200 + 10), 2);
            
            orderItems.Add(new OrderItem
            {
                ProductId = $"PROD{_random.Next(1000, 9999)}",
                ProductName = productName,
                Quantity = quantity,
                UnitPrice = unitPrice,
                Category = category
            });
            
            totalAmount += quantity * unitPrice;
        }

        return new OrderCreated
        {
            Id = Guid.NewGuid().ToString(),
            CustomerId = customerId,
            Amount = Math.Round(totalAmount, 2),
            Currency = currency,
            CreatedAt = DateTime.UtcNow,
            Source = sourceIdentifier,
            Region = region,
            OrderItems = orderItems,
            Status = "Created",
            Metadata = new Dictionary<string, object>
            {
                ["generatedAt"] = DateTime.UtcNow,
                ["version"] = "1.0",
                ["correlationId"] = Guid.NewGuid().ToString(),
                ["channel"] = "web"
            }
        };
    }
}

public class CosmosProducerService
{
    private readonly CosmosClient _cosmosClient;
    private readonly Container _sourceContainer;
    private readonly ProducerConfiguration _config;
    private readonly EventGenerator _eventGenerator;
    private readonly ILogger<CosmosProducerService> _logger;
    private readonly SemaphoreSlim _rateLimiter;

    public CosmosProducerService(ProducerConfiguration config, ILogger<CosmosProducerService> logger)
    {
        _config = config;
        _logger = logger;
        _eventGenerator = new EventGenerator();
        _rateLimiter = new SemaphoreSlim(_config.RateMessagesPerSecond, _config.RateMessagesPerSecond);

        var cosmosClientOptions = new CosmosClientOptions
        {
            MaxRetryAttemptsOnRateLimitedRequests = _config.MaxRetryAttempts,
            MaxRetryWaitTimeOnRateLimitedRequests = TimeSpan.FromMilliseconds(_config.BackoffMs),
            RequestTimeout = TimeSpan.FromMilliseconds(_config.RequestTimeoutMs),
            MaxRequestsPerTcpConnection = _config.MaxConcurrency,
            MaxTcpConnectionsPerEndpoint = _config.MaxConcurrency,
            ConnectionMode = ConnectionMode.Direct,
            Serializer = new CosmosSystemTextJsonSerializer(new JsonSerializerOptions
            {
                PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
                WriteIndented = false
            })
        };

        _cosmosClient = new CosmosClient(_config.CosmosUri, _config.CosmosKey, cosmosClientOptions);
        _sourceContainer = _cosmosClient.GetContainer(_config.CosmosDatabase, _config.CosmosSourceContainer);

        _logger.LogInformation("CosmosProducerService initialized with URI: {CosmosUri}, Database: {Database}, Container: {Container}",
            _config.CosmosUri, _config.CosmosDatabase, _config.CosmosSourceContainer);
    }

    public async Task StartProducingAsync(CancellationToken cancellationToken)
    {
        _logger.LogInformation("Starting event production at {Rate} messages/second", _config.RateMessagesPerSecond);

        var tasks = new List<Task>();
        var batchTasks = new List<Task<ItemResponse<OrderCreated>>>();
        var intervalMs = 1000.0 / _config.RateMessagesPerSecond;
        var lastExecutionTime = DateTime.UtcNow;

        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                await _rateLimiter.WaitAsync(cancellationToken);

                var order = _eventGenerator.GenerateOrder(_config.SourceIdentifier, _config.Region);
                
                var writeTask = WriteOrderAsync(order, cancellationToken);
                batchTasks.Add(writeTask);

                // Process batches when reaching batch size or at regular intervals
                if (batchTasks.Count >= _config.BatchSize || 
                    DateTime.UtcNow - lastExecutionTime > TimeSpan.FromSeconds(5))
                {
                    var completedTasks = batchTasks.ToList();
                    batchTasks.Clear();
                    lastExecutionTime = DateTime.UtcNow;

                    // Process batch asynchronously
                    tasks.Add(ProcessBatchAsync(completedTasks));

                    // Clean up completed tasks
                    tasks.RemoveAll(t => t.IsCompleted);
                }

                // Rate limiting
                var elapsed = DateTime.UtcNow - lastExecutionTime;
                var delay = TimeSpan.FromMilliseconds(intervalMs) - elapsed;
                if (delay > TimeSpan.Zero)
                {
                    await Task.Delay(delay, cancellationToken);
                }

                _rateLimiter.Release();
            }
            catch (OperationCanceledException)
            {
                break;
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error in producer loop");
                await Task.Delay(TimeSpan.FromSeconds(1), cancellationToken);
            }
        }

        // Wait for remaining tasks to complete
        if (batchTasks.Any())
        {
            tasks.Add(ProcessBatchAsync(batchTasks));
        }

        await Task.WhenAll(tasks);
        _logger.LogInformation("Event production stopped");
    }

    private async Task<ItemResponse<OrderCreated>> WriteOrderAsync(OrderCreated order, CancellationToken cancellationToken)
    {
        try
        {
            var response = await _sourceContainer.UpsertItemAsync(
                order,
                new PartitionKey(order.CustomerId),
                cancellationToken: cancellationToken);

            return response;
        }
        catch (CosmosException ex) when (ex.StatusCode == System.Net.HttpStatusCode.TooManyRequests)
        {
            _logger.LogWarning("Rate limited when writing order {OrderId}, retrying after {RetryAfter}ms", 
                order.Id, ex.RetryAfter?.TotalMilliseconds ?? _config.BackoffMs);
            
            await Task.Delay(ex.RetryAfter ?? TimeSpan.FromMilliseconds(_config.BackoffMs), cancellationToken);
            return await WriteOrderAsync(order, cancellationToken);
        }
    }

    private async Task ProcessBatchAsync(List<Task<ItemResponse<OrderCreated>>> batchTasks)
    {
        try
        {
            var responses = await Task.WhenAll(batchTasks);
            var successCount = 0;
            var totalRUs = 0.0;

            foreach (var response in responses)
            {
                if (response.StatusCode == System.Net.HttpStatusCode.OK || 
                    response.StatusCode == System.Net.HttpStatusCode.Created)
                {
                    successCount++;
                    totalRUs += response.RequestCharge;
                }
            }

            _logger.LogInformation("Batch completed: {SuccessCount}/{TotalCount} successful, {TotalRUs:F2} RUs consumed",
                successCount, responses.Length, totalRUs);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error processing batch");
        }
    }

    public void Dispose()
    {
        _cosmosClient?.Dispose();
        _rateLimiter?.Dispose();
    }
}

public class CosmosSystemTextJsonSerializer : CosmosSerializer
{
    private readonly JsonSerializerOptions _serializerOptions;

    public CosmosSystemTextJsonSerializer(JsonSerializerOptions serializerOptions)
    {
        _serializerOptions = serializerOptions;
    }

    public override T FromStream<T>(Stream stream)
    {
        using (stream)
        {
            if (stream.CanSeek && stream.Length == 0)
            {
                return default(T);
            }

            if (typeof(Stream).IsAssignableFrom(typeof(T)))
            {
                return (T)(object)stream;
            }

            return JsonSerializer.Deserialize<T>(stream, _serializerOptions);
        }
    }

    public override Stream ToStream<T>(T input)
    {
        var streamPayload = new MemoryStream();
        JsonSerializer.Serialize(streamPayload, input, _serializerOptions);
        streamPayload.Position = 0;
        return streamPayload;
    }
}