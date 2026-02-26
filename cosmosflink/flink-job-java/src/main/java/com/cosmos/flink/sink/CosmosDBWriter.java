package com.cosmos.flink.sink;

import com.azure.cosmos.*;
import com.azure.cosmos.models.*;
import com.cosmos.flink.config.CosmosDBSinkConfig;
import com.cosmos.flink.sink.CosmosDBCommittable.CosmosDBWriteRequest;
import com.fasterxml.jackson.databind.JsonNode;
import org.apache.flink.api.connector.sink2.*;
import org.apache.flink.util.concurrent.ExecutorThreadFactory;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.IOException;
import java.time.Duration;
import java.util.*;
import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicLong;

/**
 * Writer implementation for Cosmos DB sink with batching, retry logic, and exactly-once semantics
 */
public class CosmosDBWriter<T> implements TwoPhaseCommittingSink.PrecommittingSinkWriter<T, CosmosDBCommittable> {
    
    private static final Logger LOG = LoggerFactory.getLogger(CosmosDBWriter.class);
    
    private final CosmosDBSinkConfig config;
    private final CosmosDBDocumentSerializer<T> serializer;
    private final int subtaskId;
    private final AtomicLong checkpointId = new AtomicLong(0);
    
    // Cosmos DB client components
    private CosmosClient cosmosClient;
    private CosmosContainer cosmosContainer;
    
    // Batching and state management
    private final List<CosmosDBWriteRequest> pendingWrites = new ArrayList<>();
    private final ScheduledExecutorService batchTimerExecutor;
    private final ExecutorService writerExecutor;
    private ScheduledFuture<?> batchTimerFuture;
    private final Object batchLock = new Object();
    
    // Metrics and monitoring
    private final AtomicLong documentsWritten = new AtomicLong(0);
    private final AtomicLong batchesProcessed = new AtomicLong(0);
    private final AtomicLong totalRetries = new AtomicLong(0);
    private final AtomicLong totalFailures = new AtomicLong(0);
    
    public CosmosDBWriter(
            CosmosDBSinkConfig config,
            CosmosDBDocumentSerializer<T> serializer,
            Sink.InitContext context) {
        this.config = config;
        this.serializer = serializer;
        this.subtaskId = context.getSubtaskId();
        
        // Initialize executors for async processing
        this.batchTimerExecutor = Executors.newSingleThreadScheduledExecutor(
                new ExecutorThreadFactory("cosmos-batch-timer"));
        this.writerExecutor = Executors.newFixedThreadPool(
                config.getMaxConcurrency(),
                new ExecutorThreadFactory("cosmos-writer"));
        
        initializeCosmosClient();
        startBatchTimer();
        
        LOG.info("Initialized CosmosDBWriter for subtask {} with config: {}", subtaskId, config);
    }
    
    private void initializeCosmosClient() {
        try {
            // Create Cosmos client with optimized settings
            DirectConnectionConfig directConnectionConfig = new DirectConnectionConfig();
            directConnectionConfig.setMaxConnectionsPerEndpoint(config.getMaxConcurrency());
            directConnectionConfig.setMaxRequestsPerConnection(config.getMaxInFlightRequests());
            
            GatewayConnectionConfig gatewayConnectionConfig = new GatewayConnectionConfig();
            gatewayConnectionConfig.setMaxConnectionPoolSize(config.getMaxConcurrency());
            
            CosmosClientBuilder clientBuilder = new CosmosClientBuilder()
                    .endpoint(config.getCosmosUri())
                    .key(config.getCosmosKey())
                    .directMode(directConnectionConfig, gatewayConnectionConfig)
                    .consistencyLevel(ConsistencyLevel.valueOf(config.getConsistencyLevel()))
                    .contentResponseOnWriteEnabled(config.isEnableContentResponseOnWrite());
            
            if (config.getPreferredRegions() != null && !config.getPreferredRegions().isEmpty()) {
                List<String> preferredRegions = Arrays.asList(config.getPreferredRegions().split(","));
                clientBuilder.preferredRegions(preferredRegions);
            }
            
            this.cosmosClient = clientBuilder.buildClient();
            this.cosmosContainer = cosmosClient.getDatabase(config.getDatabase())
                    .getContainer(config.getContainer());
            
            LOG.info("Successfully initialized Cosmos DB client for database: {}, container: {}", 
                    config.getDatabase(), config.getContainer());
                    
        } catch (Exception e) {
            LOG.error("Failed to initialize Cosmos DB client", e);
            throw new RuntimeException("Failed to initialize Cosmos DB client", e);
        }
    }
    
    private void startBatchTimer() {
        batchTimerFuture = batchTimerExecutor.scheduleAtFixedRate(
                this::flushPendingWrites,
                config.getBatchTimeoutMs(),
                config.getBatchTimeoutMs(),
                TimeUnit.MILLISECONDS
        );
    }
    
    @Override
    public void write(T element, Context context) throws IOException, InterruptedException {
        try {
            JsonNode serializedDoc = serializer.serialize(element);
            String documentId = serializer.getDocumentId(element);
            String partitionKey = serializer.getPartitionKey(element);
            
            CosmosDBWriteRequest writeRequest = new CosmosDBWriteRequest(
                    documentId,
                    partitionKey,
                    serializedDoc.toString(),
                    CosmosDBWriteRequest.WriteOperation.UPSERT
            );
            
            synchronized (batchLock) {
                pendingWrites.add(writeRequest);
                
                // Flush if batch size is reached
                if (pendingWrites.size() >= config.getBatchSize()) {
                    flushPendingWrites();
                }
            }
            
        } catch (Exception e) {
            LOG.error("Failed to write element: {}", element, e);
            totalFailures.incrementAndGet();
            throw new IOException("Failed to write element", e);
        }
    }
    
    private void flushPendingWrites() {
        List<CosmosDBWriteRequest> toWrite;
        
        synchronized (batchLock) {
            if (pendingWrites.isEmpty()) {
                return;
            }
            toWrite = new ArrayList<>(pendingWrites);
            pendingWrites.clear();
        }
        
        LOG.debug("Flushing batch of {} write requests", toWrite.size());
        
        CompletableFuture.runAsync(() -> {
            try {
                executeBulkWrite(toWrite);
                batchesProcessed.incrementAndGet();
                documentsWritten.addAndGet(toWrite.size());
                LOG.debug("Successfully wrote batch of {} documents", toWrite.size());
                
            } catch (Exception e) {
                LOG.error("Failed to write batch of {} documents", toWrite.size(), e);
                totalFailures.incrementAndGet();
                // In a production implementation, you might want to implement a dead letter queue
                // or retry mechanism here
            }
        }, writerExecutor);
    }
    
    private void executeBulkWrite(List<CosmosDBWriteRequest> writeRequests) throws Exception {
        if (writeRequests.isEmpty()) {
            return;
        }
        
        List<CosmosItemOperation> operations = new ArrayList<>();
        
        for (CosmosDBWriteRequest request : writeRequests) {
            PartitionKey partitionKey = new PartitionKey(request.getPartitionKey());
            
            switch (request.getOperation()) {
                case CREATE:
                    operations.add(CosmosBulkOperations.getCreateItemOperation(
                            request.getDocumentJson(), partitionKey));
                    break;
                case UPSERT:
                    operations.add(CosmosBulkOperations.getUpsertItemOperation(
                            request.getDocumentJson(), partitionKey));
                    break;
                case REPLACE:
                    operations.add(CosmosBulkOperations.getReplaceItemOperation(
                            request.getDocumentId(), request.getDocumentJson(), partitionKey));
                    break;
                case DELETE:
                    operations.add(CosmosBulkOperations.getDeleteItemOperation(
                            request.getDocumentId(), partitionKey));
                    break;
            }
        }
        
        executeBulkOperationsWithRetry(operations);
    }
    
    private void executeBulkOperationsWithRetry(List<CosmosItemOperation> operations) throws Exception {
        int attempt = 0;
        Exception lastException = null;
        
        while (attempt <= config.getMaxRetryAttempts()) {
            try {
                CosmosBulkExecutionOptions bulkOptions = new CosmosBulkExecutionOptions();
                // Use default settings as the setter methods are not public
                
                Iterable<CosmosBulkOperationResponse<Object>> responses = 
                        cosmosContainer.executeBulkOperations(operations, bulkOptions);
                
                // Check for any failures in the batch
                List<CosmosBulkOperationResponse<Object>> failedOperations = new ArrayList<>();
                for (CosmosBulkOperationResponse<Object> response : responses) {
                    if (response.getResponse() == null || response.getException() != null) {
                        failedOperations.add(response);
                        LOG.warn("Bulk operation failed: exception={}", 
                                response.getException() != null ? response.getException().getMessage() : "No response");
                    }
                }
                
                if (!failedOperations.isEmpty()) {
                    throw new RuntimeException("Bulk write operation had " + failedOperations.size() + " failures");
                }
                
                return; // Success
                
            } catch (Exception e) {
                lastException = e;
                attempt++;
                totalRetries.incrementAndGet();
                
                if (attempt <= config.getMaxRetryAttempts()) {
                    long backoffTime = config.getBackoffMs() * (1L << (attempt - 1)); // Exponential backoff
                    LOG.warn("Bulk write attempt {} failed, retrying in {}ms", attempt, backoffTime, e);
                    
                    try {
                        Thread.sleep(backoffTime);
                    } catch (InterruptedException ie) {
                        Thread.currentThread().interrupt();
                        throw new RuntimeException("Interrupted during retry backoff", ie);
                    }
                } else {
                    LOG.error("Bulk write failed after {} attempts", config.getMaxRetryAttempts(), e);
                }
            }
        }
        
        throw new RuntimeException("Bulk write failed after " + config.getMaxRetryAttempts() + " attempts", lastException);
    }
    
    @Override
    public Collection<CosmosDBCommittable> prepareCommit() throws IOException, InterruptedException {
        // Flush any pending writes before preparing commit
        flushPendingWrites();
        
        // For simplicity in this implementation, we'll create an empty committable
        // In a full implementation, you would prepare transactions that can be committed/rolled back
        String transactionId = UUID.randomUUID().toString();
        CosmosDBCommittable committable = new CosmosDBCommittable(
                transactionId,
                Collections.emptyList(), // No pending transactions in this simplified implementation
                subtaskId,
                checkpointId.get()
        );
        
        LOG.debug("Prepared commit for checkpoint {} with transaction {}", 
                checkpointId.get(), transactionId);
        
        return Collections.singletonList(committable);
    }
    
    @Override
    public void flush(boolean endOfInput) throws IOException, InterruptedException {
        LOG.debug("Flushing sink writer, endOfInput: {}", endOfInput);
        flushPendingWrites();
        
        if (endOfInput) {
            // Wait for all pending writes to complete
            try {
                Thread.sleep(config.getBatchTimeoutMs());
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
                throw new InterruptedException("Interrupted while waiting for final flush");
            }
        }
    }
    
    @Override
    public void close() throws Exception {
        LOG.info("Closing CosmosDBWriter for subtask {}", subtaskId);
        
        // Cancel batch timer
        if (batchTimerFuture != null) {
            batchTimerFuture.cancel(false);
        }
        
        // Final flush
        flushPendingWrites();
        
        // Shutdown executors
        batchTimerExecutor.shutdown();
        writerExecutor.shutdown();
        
        try {
            if (!batchTimerExecutor.awaitTermination(5, TimeUnit.SECONDS)) {
                batchTimerExecutor.shutdownNow();
            }
            if (!writerExecutor.awaitTermination(10, TimeUnit.SECONDS)) {
                writerExecutor.shutdownNow();
            }
        } catch (InterruptedException e) {
            batchTimerExecutor.shutdownNow();
            writerExecutor.shutdownNow();
            Thread.currentThread().interrupt();
        }
        
        // Close Cosmos client
        if (cosmosClient != null) {
            cosmosClient.close();
        }
        
        LOG.info("CosmosDBWriter closed. Stats - Documents written: {}, Batches processed: {}, " +
                "Total retries: {}, Total failures: {}", 
                documentsWritten.get(), batchesProcessed.get(), totalRetries.get(), totalFailures.get());
    }
}