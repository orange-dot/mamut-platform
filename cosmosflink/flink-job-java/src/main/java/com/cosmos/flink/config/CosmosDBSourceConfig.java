package com.cosmos.flink.config;

import java.io.Serializable;

/**
 * Configuration class for Cosmos DB source connector
 */
public class CosmosDBSourceConfig implements Serializable {
    
    private static final long serialVersionUID = 1L;
    
    private final String cosmosUri;
    private final String cosmosKey;
    private final String database;
    private final String container;
    private final String leaseContainer;
    private final String hostName;
    private final String leasePrefix;
    private final SourceMode sourceMode;
    private final String preferredRegions;
    private final int maxRetryAttempts;
    private final int maxConcurrency;
    private final long backoffMs;
    private final long requestTimeoutMs;
    private final long pollIntervalMs;
    private final int maxBatchSize;

    public enum SourceMode {
        CHANGE_FEED,  // Unbounded streaming from change feed
        QUERY         // Bounded batch from query
    }

    private CosmosDBSourceConfig(Builder builder) {
        this.cosmosUri = builder.cosmosUri;
        this.cosmosKey = builder.cosmosKey;
        this.database = builder.database;
        this.container = builder.container;
        this.leaseContainer = builder.leaseContainer;
        this.hostName = builder.hostName;
        this.leasePrefix = builder.leasePrefix;
        this.sourceMode = builder.sourceMode;
        this.preferredRegions = builder.preferredRegions;
        this.maxRetryAttempts = builder.maxRetryAttempts;
        this.maxConcurrency = builder.maxConcurrency;
        this.backoffMs = builder.backoffMs;
        this.requestTimeoutMs = builder.requestTimeoutMs;
        this.pollIntervalMs = builder.pollIntervalMs;
        this.maxBatchSize = builder.maxBatchSize;
    }

    public static Builder builder() {
        return new Builder();
    }

    // Getters
    public String getCosmosUri() { return cosmosUri; }
    public String getCosmosKey() { return cosmosKey; }
    public String getDatabase() { return database; }
    public String getContainer() { return container; }
    public String getLeaseContainer() { return leaseContainer; }
    public String getHostName() { return hostName; }
    public String getLeasePrefix() { return leasePrefix; }
    public SourceMode getSourceMode() { return sourceMode; }
    public String getPreferredRegions() { return preferredRegions; }
    public int getMaxRetryAttempts() { return maxRetryAttempts; }
    public int getMaxConcurrency() { return maxConcurrency; }
    public long getBackoffMs() { return backoffMs; }
    public long getRequestTimeoutMs() { return requestTimeoutMs; }
    public long getPollIntervalMs() { return pollIntervalMs; }
    public int getMaxBatchSize() { return maxBatchSize; }

    public static class Builder {
        private String cosmosUri;
        private String cosmosKey;
        private String database;
        private String container;
        private String leaseContainer;
        private String hostName = "flink-cosmos-source";
        private String leasePrefix = "flink";
        private SourceMode sourceMode = SourceMode.CHANGE_FEED;
        private String preferredRegions;
        private int maxRetryAttempts = 3;
        private int maxConcurrency = 10;
        private long backoffMs = 1000;
        private long requestTimeoutMs = 10000;
        private long pollIntervalMs = 1000;
        private int maxBatchSize = 100;

        public Builder cosmosUri(String cosmosUri) { this.cosmosUri = cosmosUri; return this; }
        public Builder cosmosKey(String cosmosKey) { this.cosmosKey = cosmosKey; return this; }
        public Builder database(String database) { this.database = database; return this; }
        public Builder container(String container) { this.container = container; return this; }
        public Builder leaseContainer(String leaseContainer) { this.leaseContainer = leaseContainer; return this; }
        public Builder hostName(String hostName) { this.hostName = hostName; return this; }
        public Builder leasePrefix(String leasePrefix) { this.leasePrefix = leasePrefix; return this; }
        public Builder sourceMode(SourceMode sourceMode) { this.sourceMode = sourceMode; return this; }
        public Builder preferredRegions(String preferredRegions) { this.preferredRegions = preferredRegions; return this; }
        public Builder maxRetryAttempts(int maxRetryAttempts) { this.maxRetryAttempts = maxRetryAttempts; return this; }
        public Builder maxConcurrency(int maxConcurrency) { this.maxConcurrency = maxConcurrency; return this; }
        public Builder backoffMs(long backoffMs) { this.backoffMs = backoffMs; return this; }
        public Builder requestTimeoutMs(long requestTimeoutMs) { this.requestTimeoutMs = requestTimeoutMs; return this; }
        public Builder pollIntervalMs(long pollIntervalMs) { this.pollIntervalMs = pollIntervalMs; return this; }
        public Builder maxBatchSize(int maxBatchSize) { this.maxBatchSize = maxBatchSize; return this; }

        public CosmosDBSourceConfig build() {
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
            if (sourceMode == SourceMode.CHANGE_FEED && (leaseContainer == null || leaseContainer.isEmpty())) {
                throw new IllegalArgumentException("leaseContainer is required for CHANGE_FEED mode");
            }
            return new CosmosDBSourceConfig(this);
        }
    }

    @Override
    public String toString() {
        return "CosmosDBSourceConfig{" +
                "cosmosUri='" + cosmosUri + '\'' +
                ", database='" + database + '\'' +
                ", container='" + container + '\'' +
                ", leaseContainer='" + leaseContainer + '\'' +
                ", hostName='" + hostName + '\'' +
                ", leasePrefix='" + leasePrefix + '\'' +
                ", sourceMode=" + sourceMode +
                ", maxRetryAttempts=" + maxRetryAttempts +
                ", maxConcurrency=" + maxConcurrency +
                ", backoffMs=" + backoffMs +
                ", requestTimeoutMs=" + requestTimeoutMs +
                ", pollIntervalMs=" + pollIntervalMs +
                ", maxBatchSize=" + maxBatchSize +
                '}';
    }
}