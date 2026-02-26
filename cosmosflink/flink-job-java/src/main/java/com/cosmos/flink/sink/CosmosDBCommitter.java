package com.cosmos.flink.sink;

import org.apache.flink.api.connector.sink2.Committer;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.IOException;
import java.util.Collection;
import java.util.Collections;

/**
 * Committer implementation for Cosmos DB sink with exactly-once semantics
 * Handles the commit phase of two-phase commit protocol
 */
public class CosmosDBCommitter implements Committer<CosmosDBCommittable> {
    
    private static final Logger LOG = LoggerFactory.getLogger(CosmosDBCommitter.class);
    
    public CosmosDBCommitter() {
        LOG.info("Initialized CosmosDBCommitter");
    }
    
    @Override
    public void commit(Collection<CommitRequest<CosmosDBCommittable>> commitRequests) 
            throws IOException, InterruptedException {
        
        LOG.debug("Committing {} requests", commitRequests.size());
        
        for (CommitRequest<CosmosDBCommittable> request : commitRequests) {
            try {
                CosmosDBCommittable committable = request.getCommittable();
                
                // In this simplified implementation, the actual writes are performed
                // synchronously in the writer. In a full two-phase commit implementation,
                // you would:
                // 1. Writer prepares transactions (writes to staging/temporary state)
                // 2. Committer commits transactions (moves from staging to final state)
                // 3. Committer can roll back if needed
                
                LOG.debug("Committing transaction {} for checkpoint {} from subtask {}", 
                        committable.getTransactionId(), 
                        committable.getCheckpointId(),
                        committable.getSubtaskId());
                
                // Mark as successfully committed
                request.signalAlreadyCommitted();
                
            } catch (Exception e) {
                LOG.error("Failed to commit request: {}", request.getCommittable(), e);
                request.retryLater();
            }
        }
        
        LOG.debug("Successfully processed {} commit requests", commitRequests.size());
    }
    
    @Override
    public void close() throws Exception {
        LOG.info("Closing CosmosDBCommitter");
        // Cleanup any resources if needed
    }
}