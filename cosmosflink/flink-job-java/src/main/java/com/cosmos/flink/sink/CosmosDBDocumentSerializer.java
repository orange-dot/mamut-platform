package com.cosmos.flink.sink;

import com.fasterxml.jackson.databind.JsonNode;
import java.io.Serializable;

/**
 * Interface for serializing documents to JSON for Cosmos DB storage
 * @param <T> The type of the input document
 */
public interface CosmosDBDocumentSerializer<T> extends Serializable {
    
    /**
     * Serialize a document to JSON for Cosmos DB storage
     * 
     * @param document The document to serialize
     * @return JsonNode representation of the document
     * @throws Exception if serialization fails
     */
    JsonNode serialize(T document) throws Exception;
    
    /**
     * Extract the partition key value from the document
     * 
     * @param document The document to extract partition key from
     * @return The partition key value as a string
     * @throws Exception if partition key extraction fails
     */
    String getPartitionKey(T document) throws Exception;
    
    /**
     * Extract the document ID from the document
     * 
     * @param document The document to extract ID from
     * @return The document ID as a string
     * @throws Exception if ID extraction fails
     */
    String getDocumentId(T document) throws Exception;
}