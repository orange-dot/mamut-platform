package com.cosmos.flink.job;

import com.cosmos.flink.config.CosmosDBSinkConfig;
import com.cosmos.flink.config.CosmosDBSourceConfig;
import com.cosmos.flink.model.OrderCreated;
import com.cosmos.flink.model.ProcessedEvent;
import com.cosmos.flink.sink.CosmosDBSink;
import com.cosmos.flink.sink.JacksonCosmosSerializer;
import com.cosmos.flink.source.CosmosDBSource;
import com.cosmos.flink.source.JacksonCosmosDeserializer;
import org.apache.flink.api.common.eventtime.WatermarkStrategy;
import org.apache.flink.streaming.api.datastream.DataStream;
import org.apache.flink.streaming.api.environment.StreamExecutionEnvironment;
import org.apache.flink.streaming.api.functions.sink.PrintSinkFunction;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Primary Flink job demonstrating Cosmos DB source integration
 * Reads from events-source container and processes orders
 */
public class FlinkCosmosPrimaryJob {
    
    private static final Logger LOG = LoggerFactory.getLogger(FlinkCosmosPrimaryJob.class);

    public static void main(String[] args) throws Exception {
        
        LOG.info("Starting Flink Cosmos Primary Job");
        
        // Create execution environment
        StreamExecutionEnvironment env = StreamExecutionEnvironment.getExecutionEnvironment();
        
        // Enable checkpointing for exactly-once processing
        env.enableCheckpointing(30000); // 30 seconds
        
        // Load configuration from environment variables
        CosmosDBSourceConfig sourceConfig = loadSourceConfig();
        CosmosDBSinkConfig sinkConfig = loadSinkConfig();
        
        LOG.info("Loaded source configuration: {}", sourceConfig);
        LOG.info("Loaded sink configuration: {}", sinkConfig);
        
        // Create Cosmos DB source
        CosmosDBSource<OrderCreated> cosmosSource = CosmosDBSource.<OrderCreated>builder()
                .setConfig(sourceConfig)
                .setDeserializer(new JacksonCosmosDeserializer<>(OrderCreated.class))
                .build();
        
        // Create data stream from Cosmos DB
        DataStream<OrderCreated> orderStream = env
                .fromSource(cosmosSource, WatermarkStrategy.noWatermarks(), "CosmosDB Source")
                .setParallelism(2); // Use 2 parallel instances
        
        // Transform orders to processed events
        DataStream<ProcessedEvent> processedStream = orderStream
                .map(order -> {
                    LOG.info("Processing order: {}", order.getId());
                    ProcessedEvent processed = ProcessedEvent.fromOrderCreated(order, "primary-pipeline");
                    processed.setProcessingDurationMs(System.currentTimeMillis() - order.getCreatedAt().toEpochMilli());
                    return processed;
                })
                .name("Order Processor");
        
        // Create Cosmos DB sink for processed events
        CosmosDBSink<ProcessedEvent> cosmosSink = CosmosDBSink.<ProcessedEvent>builder()
                .setConfig(sinkConfig)
                .setSerializer(new JacksonCosmosSerializer<>(ProcessedEvent.class))
                .build();
        
        // Write processed events to Cosmos DB
        processedStream.sinkTo(cosmosSink)
                .name("Cosmos DB Sink");
        
        // Also print for demo purposes (can be removed in production)
        processedStream.addSink(new PrintSinkFunction<>())
                .name("Print Sink");
        
        LOG.info("Starting job execution");
        
        // Execute the job
        env.execute("Flink Cosmos Primary Pipeline");
    }

    private static CosmosDBSourceConfig loadSourceConfig() {
        return CosmosDBSourceConfig.builder()
                .cosmosUri(getEnvOrDefault("COSMOS_URI", "https://localhost:8081"))
                .cosmosKey(getEnvOrDefault("COSMOS_KEY", ""))
                .database(getEnvOrDefault("COSMOS_DB", "cosmosflink-demo"))
                .container(getEnvOrDefault("COSMOS_SOURCE_CONTAINER", "events-source"))
                .leaseContainer(getEnvOrDefault("COSMOS_LEASE_CONTAINER", "cosmos-leases"))
                .hostName(getEnvOrDefault("CFP_HOST_NAME", "flink-primary-" + System.currentTimeMillis()))
                .leasePrefix(getEnvOrDefault("CFP_LEASE_PREFIX", "primary"))
                .sourceMode(CosmosDBSourceConfig.SourceMode.CHANGE_FEED)
                .maxRetryAttempts(Integer.parseInt(getEnvOrDefault("COSMOS_MAX_RETRY_ATTEMPTS", "3")))
                .maxConcurrency(Integer.parseInt(getEnvOrDefault("COSMOS_MAX_CONCURRENCY", "10")))
                .backoffMs(Long.parseLong(getEnvOrDefault("COSMOS_BACKOFF_MS", "1000")))
                .requestTimeoutMs(Long.parseLong(getEnvOrDefault("COSMOS_REQUEST_TIMEOUT_MS", "10000")))
                .pollIntervalMs(Long.parseLong(getEnvOrDefault("POLL_INTERVAL_MS", "1000")))
                .maxBatchSize(Integer.parseInt(getEnvOrDefault("MAX_BATCH_SIZE", "100")))
                .build();
    }

    private static CosmosDBSinkConfig loadSinkConfig() {
        return CosmosDBSinkConfig.builder()
                .cosmosUri(getEnvOrDefault("COSMOS_URI", "https://localhost:8081"))
                .cosmosKey(getEnvOrDefault("COSMOS_KEY", ""))
                .database(getEnvOrDefault("COSMOS_DB", "cosmosflink-demo"))
                .container(getEnvOrDefault("COSMOS_SINK_CONTAINER", "events-processed"))
                .maxRetryAttempts(Integer.parseInt(getEnvOrDefault("COSMOS_MAX_RETRY_ATTEMPTS", "3")))
                .maxConcurrency(Integer.parseInt(getEnvOrDefault("COSMOS_MAX_CONCURRENCY", "10")))
                .backoffMs(Long.parseLong(getEnvOrDefault("COSMOS_BACKOFF_MS", "1000")))
                .requestTimeoutMs(Long.parseLong(getEnvOrDefault("COSMOS_REQUEST_TIMEOUT_MS", "10000")))
                .batchSize(Integer.parseInt(getEnvOrDefault("SINK_BATCH_SIZE", "25")))
                .batchTimeoutMs(Long.parseLong(getEnvOrDefault("SINK_BATCH_TIMEOUT_MS", "5000")))
                .maxInFlightRequests(Integer.parseInt(getEnvOrDefault("SINK_MAX_INFLIGHT_REQUESTS", "5")))
                .enableContentResponseOnWrite(Boolean.parseBoolean(getEnvOrDefault("SINK_ENABLE_CONTENT_RESPONSE", "false")))
                .consistencyLevel(getEnvOrDefault("SINK_CONSISTENCY_LEVEL", "EVENTUAL"))
                .bulkExecutionThreshold(Integer.parseInt(getEnvOrDefault("SINK_BULK_THRESHOLD", "100")))
                .bulkExecutionTimeoutMs(Long.parseLong(getEnvOrDefault("SINK_BULK_TIMEOUT_MS", "30000")))
                .build();
    }

    private static String getEnvOrDefault(String key, String defaultValue) {
        String value = System.getenv(key);
        return value != null ? value : defaultValue;
    }
}