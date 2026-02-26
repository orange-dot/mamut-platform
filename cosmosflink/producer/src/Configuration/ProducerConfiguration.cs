namespace CosmosFlinkProducer.Configuration;

public class ProducerConfiguration
{
    public string CosmosUri { get; set; } = string.Empty;
    public string CosmosKey { get; set; } = string.Empty;
    public string CosmosDatabase { get; set; } = string.Empty;
    public string CosmosSourceContainer { get; set; } = string.Empty;
    public string CosmosSourcePartitionKeyPath { get; set; } = "/customerId";
    
    public int RateMessagesPerSecond { get; set; } = 10;
    public int BatchSize { get; set; } = 100;
    public string Region { get; set; } = "eastus";
    
    public int MaxConcurrency { get; set; } = 10;
    public int MaxRetryAttempts { get; set; } = 3;
    public int BackoffMs { get; set; } = 1000;
    public int RequestTimeoutMs { get; set; } = 10000;
    
    public string SourceIdentifier { get; set; } = "cosmosflink-producer";
    
    public void Validate()
    {
        if (string.IsNullOrEmpty(CosmosUri))
            throw new ArgumentException("COSMOS_URI environment variable is required");
        
        if (string.IsNullOrEmpty(CosmosKey))
            throw new ArgumentException("COSMOS_KEY environment variable is required");
        
        if (string.IsNullOrEmpty(CosmosDatabase))
            throw new ArgumentException("COSMOS_DB environment variable is required");
        
        if (string.IsNullOrEmpty(CosmosSourceContainer))
            throw new ArgumentException("COSMOS_SOURCE_CONTAINER environment variable is required");
            
        if (RateMessagesPerSecond <= 0)
            throw new ArgumentException("PRODUCER_RATE_MSG_PER_SEC must be greater than 0");
    }
    
    public static ProducerConfiguration FromEnvironment()
    {
        return new ProducerConfiguration
        {
            CosmosUri = Environment.GetEnvironmentVariable("COSMOS_URI") ?? string.Empty,
            CosmosKey = Environment.GetEnvironmentVariable("COSMOS_KEY") ?? string.Empty,
            CosmosDatabase = Environment.GetEnvironmentVariable("COSMOS_DB") ?? string.Empty,
            CosmosSourceContainer = Environment.GetEnvironmentVariable("COSMOS_SOURCE_CONTAINER") ?? string.Empty,
            CosmosSourcePartitionKeyPath = Environment.GetEnvironmentVariable("COSMOS_SOURCE_PKEY_PATH") ?? "/customerId",
            
            RateMessagesPerSecond = int.TryParse(Environment.GetEnvironmentVariable("PRODUCER_RATE_MSG_PER_SEC"), out var rate) ? rate : 10,
            BatchSize = int.TryParse(Environment.GetEnvironmentVariable("PRODUCER_BATCH_SIZE"), out var batch) ? batch : 100,
            Region = Environment.GetEnvironmentVariable("PRODUCER_REGION") ?? "eastus",
            
            MaxConcurrency = int.TryParse(Environment.GetEnvironmentVariable("COSMOS_MAX_CONCURRENCY"), out var concurrency) ? concurrency : 10,
            MaxRetryAttempts = int.TryParse(Environment.GetEnvironmentVariable("COSMOS_MAX_RETRY_ATTEMPTS"), out var retries) ? retries : 3,
            BackoffMs = int.TryParse(Environment.GetEnvironmentVariable("COSMOS_BACKOFF_MS"), out var backoff) ? backoff : 1000,
            RequestTimeoutMs = int.TryParse(Environment.GetEnvironmentVariable("COSMOS_REQUEST_TIMEOUT_MS"), out var timeout) ? timeout : 10000,
            
            SourceIdentifier = Environment.GetEnvironmentVariable("SOURCE_IDENTIFIER") ?? "cosmosflink-producer"
        };
    }
}