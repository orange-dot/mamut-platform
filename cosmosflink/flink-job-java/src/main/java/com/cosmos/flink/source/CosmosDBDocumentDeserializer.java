package com.cosmos.flink.source;

import com.fasterxml.jackson.databind.JsonNode;
import java.io.Serializable;

/**
 * Interface for deserializing Cosmos DB documents
 */
public interface CosmosDBDocumentDeserializer<T> extends Serializable {
    
    /**
     * Deserialize a Cosmos DB document from JsonNode to target type
     * 
     * @param document The JSON document from Cosmos DB
     * @return Deserialized object
     * @throws Exception if deserialization fails
     */
    T deserialize(JsonNode document) throws Exception;
    
    /**
     * Get the target type information
     */
    Class<T> getTargetType();
}