package com.cosmos.flink.config;

import java.io.Serializable;

/**
 * Configuration class for Cosmos DB sink connector with batching and retry logic
 */
public class CosmosDBSinkConfig implements Serializable {
    
    private static final long serialVersionUID = 1L;
    
    private final String cosmosUri;
    private final String cosmosKey;
    private final String database;
    private final String container;
    private final String preferredRegions;
    private final int maxRetryAttempts;
    private final int maxConcurrency;
    private final long backoffMs;
    private final long requestTimeoutMs;
    private final int batchSize;
    private final long batchTimeoutMs;
    private final int maxInFlightRequests;
    private final boolean enableContentResponseOnWrite;
    private final String consistencyLevel;
    private final int bulkExecutionThreshold;
    private final long bulkExecutionTimeoutMs;

    private CosmosDBSinkConfig(Builder builder) {
        this.cosmosUri = builder.cosmosUri;
        this.cosmosKey = builder.cosmosKey;
        this.database = builder.database;
        this.container = builder.container;
        this.preferredRegions = builder.preferredRegions;
        this.maxRetryAttempts = builder.maxRetryAttempts;
        this.maxConcurrency = builder.maxConcurrency;
        this.backoffMs = builder.backoffMs;
        this.requestTimeoutMs = builder.requestTimeoutMs;
        this.batchSize = builder.batchSize;
        this.batchTimeoutMs = builder.batchTimeoutMs;
        this.maxInFlightRequests = builder.maxInFlightRequests;
        this.enableContentResponseOnWrite = builder.enableContentResponseOnWrite;
        this.consistencyLevel = builder.consistencyLevel;
        this.bulkExecutionThreshold = builder.bulkExecutionThreshold;
        this.bulkExecutionTimeoutMs = builder.bulkExecutionTimeoutMs;
    }

    public static Builder builder() {
        return new Builder();
    }

    // Getters
    public String getCosmosUri() { return cosmosUri; }
    public String getCosmosKey() { return cosmosKey; }
    public String getDatabase() { return database; }
    public String getContainer() { return container; }
    public String getPreferredRegions() { return preferredRegions; }
    public int getMaxRetryAttempts() { return maxRetryAttempts; }
    public int getMaxConcurrency() { return maxConcurrency; }
    public long getBackoffMs() { return backoffMs; }
    public long getRequestTimeoutMs() { return requestTimeoutMs; }
    public int getBatchSize() { return batchSize; }
    public long getBatchTimeoutMs() { return batchTimeoutMs; }
    public int getMaxInFlightRequests() { return maxInFlightRequests; }
    public boolean isEnableContentResponseOnWrite() { return enableContentResponseOnWrite; }
    public String getConsistencyLevel() { return consistencyLevel; }
    public int getBulkExecutionThreshold() { return bulkExecutionThreshold; }
    public long getBulkExecutionTimeoutMs() { return bulkExecutionTimeoutMs; }

    public static class Builder {
        private String cosmosUri;
        private String cosmosKey;
        private String database;
        private String container;
        private String preferredRegions;
        private int maxRetryAttempts = 3;
        private int maxConcurrency = 10;
        private long backoffMs = 1000;
        private long requestTimeoutMs = 10000;
        private int batchSize = 25;
        private long batchTimeoutMs = 5000;
        private int maxInFlightRequests = 5;
        private boolean enableContentResponseOnWrite = false;
        private String consistencyLevel = "EVENTUAL";
        private int bulkExecutionThreshold = 100;
        private long bulkExecutionTimeoutMs = 30000;

        public Builder cosmosUri(String cosmosUri) { this.cosmosUri = cosmosUri; return this; }
        public Builder cosmosKey(String cosmosKey) { this.cosmosKey = cosmosKey; return this; }
        public Builder database(String database) { this.database = database; return this; }
        public Builder container(String container) { this.container = container; return this; }
        public Builder preferredRegions(String preferredRegions) { this.preferredRegions = preferredRegions; return this; }
        public Builder maxRetryAttempts(int maxRetryAttempts) { this.maxRetryAttempts = maxRetryAttempts; return this; }
        public Builder maxConcurrency(int maxConcurrency) { this.maxConcurrency = maxConcurrency; return this; }
        public Builder backoffMs(long backoffMs) { this.backoffMs = backoffMs; return this; }
        public Builder requestTimeoutMs(long requestTimeoutMs) { this.requestTimeoutMs = requestTimeoutMs; return this; }
        public Builder batchSize(int batchSize) { this.batchSize = batchSize; return this; }
        public Builder batchTimeoutMs(long batchTimeoutMs) { this.batchTimeoutMs = batchTimeoutMs; return this; }
        public Builder maxInFlightRequests(int maxInFlightRequests) { this.maxInFlightRequests = maxInFlightRequests; return this; }
        public Builder enableContentResponseOnWrite(boolean enableContentResponseOnWrite) { 
            this.enableContentResponseOnWrite = enableContentResponseOnWrite; 
            return this; 
        }
        public Builder consistencyLevel(String consistencyLevel) { this.consistencyLevel = consistencyLevel; return this; }
        public Builder bulkExecutionThreshold(int bulkExecutionThreshold) { 
            this.bulkExecutionThreshold = bulkExecutionThreshold; 
            return this; 
        }
        public Builder bulkExecutionTimeoutMs(long bulkExecutionTimeoutMs) { 
            this.bulkExecutionTimeoutMs = bulkExecutionTimeoutMs; 
            return this; 
        }

        public CosmosDBSinkConfig build() {
            if (cosmosUri == null || cosmosUri.isEmpty()) {
                throw new IllegalArgumentException("cosmosUri is required");
            }
            if (cosmosKey == null || cosmosKey.isEmpty()) {
                throw new IllegalArgumentException("cosmosKey is required");
            }
            if (database == null || database.isEmpty()) {
                throw new IllegalArgumentException("database is required");
            }
            if (container == null || container.isEmpty()) {
                throw new IllegalArgumentException("container is required");
            }
            if (batchSize <= 0) {
                throw new IllegalArgumentException("batchSize must be positive");
            }
            if (maxRetryAttempts < 0) {
                throw new IllegalArgumentException("maxRetryAttempts must be non-negative");
            }
            return new CosmosDBSinkConfig(this);
        }
    }

    @Override
    public String toString() {
        return "CosmosDBSinkConfig{" +
                "cosmosUri='" + cosmosUri + '\'' +
                ", database='" + database + '\'' +
                ", container='" + container + '\'' +
                ", maxRetryAttempts=" + maxRetryAttempts +
                ", maxConcurrency=" + maxConcurrency +
                ", backoffMs=" + backoffMs +
                ", requestTimeoutMs=" + requestTimeoutMs +
                ", batchSize=" + batchSize +
                ", batchTimeoutMs=" + batchTimeoutMs +
                ", maxInFlightRequests=" + maxInFlightRequests +
                ", enableContentResponseOnWrite=" + enableContentResponseOnWrite +
                ", consistencyLevel='" + consistencyLevel + '\'' +
                ", bulkExecutionThreshold=" + bulkExecutionThreshold +
                ", bulkExecutionTimeoutMs=" + bulkExecutionTimeoutMs +
                '}';
    }
}