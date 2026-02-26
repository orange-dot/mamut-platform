namespace CosmosFlinkConsumer.Configuration;

public class ConsumerConfiguration
{
    public string HttpHost { get; set; } = "0.0.0.0";
    public int HttpPort { get; set; } = 8080;
    public int TcpPort { get; set; } = 9092;
    public string SerializationFormat { get; set; } = "json";
    public int ConnectionPoolSize { get; set; } = 10;
    public int TimeoutMs { get; set; } = 5000;
    
    public string ConsumerIdentifier { get; set; } = "cosmosflink-consumer";
    public bool EnableDetailedLogging { get; set; } = true;
    public int MaxBatchSize { get; set; } = 100;
    public int FlushIntervalMs { get; set; } = 5000;
    
    public void Validate()
    {
        if (HttpPort <= 0 || HttpPort > 65535)
            throw new ArgumentException("CONSUMER_HTTP_PORT must be between 1 and 65535");
            
        if (TcpPort <= 0 || TcpPort > 65535)
            throw new ArgumentException("TCP_PORT must be between 1 and 65535");
            
        if (TimeoutMs <= 0)
            throw new ArgumentException("TIMEOUT_MS must be greater than 0");
    }
    
    public static ConsumerConfiguration FromEnvironment()
    {
        return new ConsumerConfiguration
        {
            HttpHost = Environment.GetEnvironmentVariable("CONSUMER_HTTP_HOST") ?? "0.0.0.0",
            HttpPort = int.TryParse(Environment.GetEnvironmentVariable("CONSUMER_HTTP_PORT"), out var httpPort) ? httpPort : 8080,
            TcpPort = int.TryParse(Environment.GetEnvironmentVariable("TCP_PORT"), out var tcpPort) ? tcpPort : 9092,
            SerializationFormat = Environment.GetEnvironmentVariable("SERIALIZATION_FORMAT") ?? "json",
            ConnectionPoolSize = int.TryParse(Environment.GetEnvironmentVariable("CONNECTION_POOL_SIZE"), out var poolSize) ? poolSize : 10,
            TimeoutMs = int.TryParse(Environment.GetEnvironmentVariable("TIMEOUT_MS"), out var timeout) ? timeout : 5000,
            
            ConsumerIdentifier = Environment.GetEnvironmentVariable("CONSUMER_IDENTIFIER") ?? "cosmosflink-consumer",
            EnableDetailedLogging = bool.TryParse(Environment.GetEnvironmentVariable("ENABLE_DETAILED_LOGGING"), out var detailedLogging) ? detailedLogging : true,
            MaxBatchSize = int.TryParse(Environment.GetEnvironmentVariable("MAX_BATCH_SIZE"), out var batchSize) ? batchSize : 100,
            FlushIntervalMs = int.TryParse(Environment.GetEnvironmentVariable("FLUSH_INTERVAL_MS"), out var flushInterval) ? flushInterval : 5000
        };
    }
}