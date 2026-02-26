package com.cosmos.flink.source;

import org.apache.flink.api.connector.source.SourceSplit;
import java.io.Serializable;
import java.util.Objects;

/**
 * Split implementation for Cosmos DB source that represents a partition range
 */
public class CosmosDBSplit implements SourceSplit, Serializable {
    
    private static final long serialVersionUID = 1L;
    
    private final String splitId;
    private final String feedRange;
    private final String continuationToken;
    private final boolean isFinished;

    public CosmosDBSplit(String splitId, String feedRange, String continuationToken) {
        this(splitId, feedRange, continuationToken, false);
    }

    public CosmosDBSplit(String splitId, String feedRange, String continuationToken, boolean isFinished) {
        this.splitId = Objects.requireNonNull(splitId, "splitId cannot be null");
        this.feedRange = Objects.requireNonNull(feedRange, "feedRange cannot be null");
        this.continuationToken = continuationToken; // Can be null for initial splits
        this.isFinished = isFinished;
    }

    @Override
    public String splitId() {
        return splitId;
    }

    public String getFeedRange() {
        return feedRange;
    }

    public String getContinuationToken() {
        return continuationToken;
    }

    public boolean isFinished() {
        return isFinished;
    }

    /**
     * Create a new split with updated continuation token
     */
    public CosmosDBSplit withContinuationToken(String newContinuationToken) {
        return new CosmosDBSplit(this.splitId, this.feedRange, newContinuationToken, this.isFinished);
    }

    /**
     * Mark this split as finished
     */
    public CosmosDBSplit finished() {
        return new CosmosDBSplit(this.splitId, this.feedRange, this.continuationToken, true);
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        CosmosDBSplit that = (CosmosDBSplit) o;
        return isFinished == that.isFinished &&
                Objects.equals(splitId, that.splitId) &&
                Objects.equals(feedRange, that.feedRange) &&
                Objects.equals(continuationToken, that.continuationToken);
    }

    @Override
    public int hashCode() {
        return Objects.hash(splitId, feedRange, continuationToken, isFinished);
    }

    @Override
    public String toString() {
        return "CosmosDBSplit{" +
                "splitId='" + splitId + '\'' +
                ", feedRange='" + feedRange + '\'' +
                ", continuationToken='" + continuationToken + '\'' +
                ", isFinished=" + isFinished +
                '}';
    }
}