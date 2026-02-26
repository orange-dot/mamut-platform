package com.cosmos.flink.source;

import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

/**
 * State for the Cosmos DB source to track splits and continuation tokens
 */
public class CosmosDBSourceState {
    
    private final Map<String, String> continuationTokens;
    private final boolean isInitialized;

    public CosmosDBSourceState() {
        this.continuationTokens = new ConcurrentHashMap<>();
        this.isInitialized = false;
    }

    public CosmosDBSourceState(Map<String, String> continuationTokens, boolean isInitialized) {
        this.continuationTokens = new ConcurrentHashMap<>(continuationTokens);
        this.isInitialized = isInitialized;
    }

    /**
     * Get continuation token for a specific split
     */
    public String getContinuationToken(String splitId) {
        return continuationTokens.get(splitId);
    }

    /**
     * Update continuation token for a split
     */
    public void updateContinuationToken(String splitId, String continuationToken) {
        if (continuationToken != null) {
            continuationTokens.put(splitId, continuationToken);
        }
    }

    /**
     * Get all continuation tokens
     */
    public Map<String, String> getContinuationTokens() {
        return new ConcurrentHashMap<>(continuationTokens);
    }

    /**
     * Check if source has been initialized
     */
    public boolean isInitialized() {
        return isInitialized;
    }

    /**
     * Create a new state marking as initialized
     */
    public CosmosDBSourceState asInitialized() {
        return new CosmosDBSourceState(this.continuationTokens, true);
    }

    /**
     * Remove continuation token for a split (when split is finished)
     */
    public void removeContinuationToken(String splitId) {
        continuationTokens.remove(splitId);
    }

    @Override
    public String toString() {
        return "CosmosDBSourceState{" +
                "continuationTokens=" + continuationTokens.keySet() +
                ", isInitialized=" + isInitialized +
                '}';
    }
}