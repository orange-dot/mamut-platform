package com.cosmos.flink.source;

import com.azure.cosmos.CosmosAsyncClient;
import com.azure.cosmos.CosmosAsyncContainer;
import com.azure.cosmos.CosmosAsyncDatabase;
import com.azure.cosmos.CosmosClientBuilder;
import com.azure.cosmos.models.FeedRange;
import com.cosmos.flink.config.CosmosDBSourceConfig;
import org.apache.flink.api.connector.source.SplitEnumerator;
import org.apache.flink.api.connector.source.SplitEnumeratorContext;
import org.apache.flink.api.connector.source.SplitsAssignment;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.IOException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * Split enumerator for discovering Cosmos DB partitions
 */
public class CosmosDBSplitEnumerator implements SplitEnumerator<CosmosDBSplit, CosmosDBSourceState> {
    
    private static final Logger LOG = LoggerFactory.getLogger(CosmosDBSplitEnumerator.class);
    
    private final SplitEnumeratorContext<CosmosDBSplit> context;
    private final CosmosDBSourceConfig config;
    private CosmosDBSourceState state;
    private CosmosAsyncClient cosmosClient;
    private CosmosAsyncContainer container;

    public CosmosDBSplitEnumerator(
            SplitEnumeratorContext<CosmosDBSplit> context,
            CosmosDBSourceConfig config,
            CosmosDBSourceState restoredState) {
        this.context = context;
        this.config = config;
        this.state = restoredState != null ? restoredState : new CosmosDBSourceState();
        
        initializeCosmosClient();
    }

    private void initializeCosmosClient() {
        try {
            this.cosmosClient = new CosmosClientBuilder()
                    .endpoint(config.getCosmosUri())
                    .key(config.getCosmosKey())
                    .buildAsyncClient();
            
            CosmosAsyncDatabase database = cosmosClient.getDatabase(config.getDatabase());
            this.container = database.getContainer(config.getContainer());
            
            LOG.info("Initialized Cosmos DB client for container: {}", config.getContainer());
        } catch (Exception e) {
            LOG.error("Failed to initialize Cosmos DB client", e);
            throw new RuntimeException("Failed to initialize Cosmos DB client", e);
        }
    }

    @Override
    public void start() {
        LOG.info("Starting CosmosDBSplitEnumerator");
        if (!state.isInitialized()) {
            discoverSplits();
        }
    }

    private void discoverSplits() {
        try {
            LOG.info("Discovering splits for container: {}", config.getContainer());
            
            // Get feed ranges (partitions) from the container
            List<FeedRange> feedRanges = container.getFeedRanges().block();
            
            if (feedRanges == null || feedRanges.isEmpty()) {
                LOG.warn("No feed ranges discovered for container: {}", config.getContainer());
                return;
            }
            
            LOG.info("Discovered {} feed ranges for parallel processing", feedRanges.size());
            
            List<CosmosDBSplit> splits = new ArrayList<>();
            for (int i = 0; i < feedRanges.size(); i++) {
                FeedRange feedRange = feedRanges.get(i);
                String splitId = "split-" + i;
                String continuationToken = state.getContinuationToken(splitId);
                
                CosmosDBSplit split = new CosmosDBSplit(splitId, feedRange.toString(), continuationToken);
                splits.add(split);
                
                LOG.debug("Created split: {} for feedRange: {} with continuationToken: {}", 
                    splitId, feedRange, continuationToken != null ? "present" : "null");
            }
            
            // Distribute splits across available readers
            int numReaders = context.currentParallelism();
            LOG.info("Distributing {} splits across {} readers", splits.size(), numReaders);
            
            Map<Integer, List<CosmosDBSplit>> assignments = new HashMap<>();
            for (int i = 0; i < splits.size(); i++) {
                int readerIndex = i % numReaders;
                assignments.computeIfAbsent(readerIndex, k -> new ArrayList<>()).add(splits.get(i));
            }
            
            // Assign splits to readers
            context.assignSplits(new SplitsAssignment<>(assignments));
            state = state.asInitialized();
            
            LOG.info("Successfully assigned splits to {} readers", assignments.size());
            assignments.forEach((readerId, readerSplits) -> 
                LOG.debug("Reader {}: {} splits", readerId, readerSplits.size()));
            
        } catch (Exception e) {
            LOG.error("Failed to discover splits", e);
            throw new RuntimeException("Failed to discover splits", e);
        }
    }

    @Override
    public void handleSplitRequest(int subtaskId, String requesterHostname) {
        LOG.debug("Split request received from subtask {} ({})", subtaskId, requesterHostname);
        
        // For change feed mode, we can assign splits dynamically if we have unassigned ones
        // This handles scenarios where new readers are added during runtime
        if (state.isInitialized()) {
            // Check if we have any unassigned splits or can create new ones
            List<CosmosDBSplit> availableSplits = getAvailableSplits();
            if (!availableSplits.isEmpty()) {
                Map<Integer, List<CosmosDBSplit>> assignments = new HashMap<>();
                assignments.put(subtaskId, availableSplits);
                context.assignSplits(new SplitsAssignment<>(assignments));
                
                LOG.info("Assigned {} splits to subtask {}", availableSplits.size(), subtaskId);
            } else {
                LOG.debug("No available splits for subtask {}", subtaskId);
            }
        }
    }
    
    /**
     * Get available splits that can be assigned to new readers
     */
    private List<CosmosDBSplit> getAvailableSplits() {
        // In a more sophisticated implementation, this could track which splits
        // are currently assigned and create new splits for unassigned partitions
        // For now, return empty list as splits are assigned at startup
        return new ArrayList<>();
    }

    @Override
    public void addSplitsBack(List<CosmosDBSplit> splits, int subtaskId) {
        LOG.info("Adding {} splits back for subtask {}", splits.size(), subtaskId);
        // Re-assign the splits back to available readers
        if (!splits.isEmpty()) {
            Map<Integer, List<CosmosDBSplit>> assignments = new HashMap<>();
            assignments.put(subtaskId, splits);
            context.assignSplits(new SplitsAssignment<>(assignments));
        }
    }

    @Override
    public void addReader(int subtaskId) {
        LOG.info("Adding reader for subtask {}", subtaskId);
        // If we haven't initialized yet, do it now
        if (!state.isInitialized()) {
            discoverSplits();
        }
    }

    @Override
    public CosmosDBSourceState snapshotState(long checkpointId) throws Exception {
        LOG.debug("Snapshotting state for checkpoint {}", checkpointId);
        return state;
    }

    @Override
    public void close() throws IOException {
        LOG.info("Closing CosmosDBSplitEnumerator");
        if (cosmosClient != null) {
            cosmosClient.close();
        }
    }
}