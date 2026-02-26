package com.cosmos.flink.sink;

import java.io.Serializable;
import java.util.List;
import java.util.Objects;

/**
 * Represents a committable batch of documents for Cosmos DB sink
 * Used in two-phase commit protocol for exactly-once semantics
 */
public class CosmosDBCommittable implements Serializable {
    
    private static final long serialVersionUID = 1L;
    
    private final String transactionId;
    private final List<CosmosDBWriteRequest> writeRequests;
    private final long timestamp;
    private final int subtaskId;
    private final long checkpointId;
    
    public CosmosDBCommittable(
            String transactionId, 
            List<CosmosDBWriteRequest> writeRequests,
            int subtaskId,
            long checkpointId) {
        this.transactionId = transactionId;
        this.writeRequests = writeRequests;
        this.timestamp = System.currentTimeMillis();
        this.subtaskId = subtaskId;
        this.checkpointId = checkpointId;
    }
    
    public String getTransactionId() {
        return transactionId;
    }
    
    public List<CosmosDBWriteRequest> getWriteRequests() {
        return writeRequests;
    }
    
    public long getTimestamp() {
        return timestamp;
    }
    
    public int getSubtaskId() {
        return subtaskId;
    }
    
    public long getCheckpointId() {
        return checkpointId;
    }
    
    public int size() {
        return writeRequests.size();
    }
    
    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        CosmosDBCommittable that = (CosmosDBCommittable) o;
        return checkpointId == that.checkpointId &&
                subtaskId == that.subtaskId &&
                Objects.equals(transactionId, that.transactionId);
    }
    
    @Override
    public int hashCode() {
        return Objects.hash(transactionId, subtaskId, checkpointId);
    }
    
    @Override
    public String toString() {
        return "CosmosDBCommittable{" +
                "transactionId='" + transactionId + '\'' +
                ", writeRequestsCount=" + writeRequests.size() +
                ", timestamp=" + timestamp +
                ", subtaskId=" + subtaskId +
                ", checkpointId=" + checkpointId +
                '}';
    }
    
    /**
     * Represents a single write request within a committable batch
     */
    public static class CosmosDBWriteRequest implements Serializable {
        
        private static final long serialVersionUID = 1L;
        
        private final String documentId;
        private final String partitionKey;
        private final String documentJson;
        private final WriteOperation operation;
        
        public enum WriteOperation {
            CREATE,
            UPSERT,
            REPLACE,
            DELETE
        }
        
        public CosmosDBWriteRequest(String documentId, String partitionKey, String documentJson, WriteOperation operation) {
            this.documentId = documentId;
            this.partitionKey = partitionKey;
            this.documentJson = documentJson;
            this.operation = operation;
        }
        
        public String getDocumentId() { return documentId; }
        public String getPartitionKey() { return partitionKey; }
        public String getDocumentJson() { return documentJson; }
        public WriteOperation getOperation() { return operation; }
        
        @Override
        public boolean equals(Object o) {
            if (this == o) return true;
            if (o == null || getClass() != o.getClass()) return false;
            CosmosDBWriteRequest that = (CosmosDBWriteRequest) o;
            return Objects.equals(documentId, that.documentId) &&
                    Objects.equals(partitionKey, that.partitionKey) &&
                    operation == that.operation;
        }
        
        @Override
        public int hashCode() {
            return Objects.hash(documentId, partitionKey, operation);
        }
        
        @Override
        public String toString() {
            return "CosmosDBWriteRequest{" +
                    "documentId='" + documentId + '\'' +
                    ", partitionKey='" + partitionKey + '\'' +
                    ", operation=" + operation +
                    '}';
        }
    }
}