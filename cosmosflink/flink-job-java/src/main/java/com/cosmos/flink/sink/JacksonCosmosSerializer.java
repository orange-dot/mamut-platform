package com.cosmos.flink.sink;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.fasterxml.jackson.datatype.jsr310.JavaTimeModule;

/**
 * Jackson-based serializer for Cosmos DB documents
 * @param <T> The type of the input document
 */
public class JacksonCosmosSerializer<T> implements CosmosDBDocumentSerializer<T> {
    
    private static final long serialVersionUID = 1L;
    
    private final ObjectMapper objectMapper;
    private final String partitionKeyPath;
    private final String idPath;
    
    public JacksonCosmosSerializer(String partitionKeyPath, String idPath) {
        this.partitionKeyPath = partitionKeyPath;
        this.idPath = idPath;
        this.objectMapper = new ObjectMapper();
        this.objectMapper.registerModule(new JavaTimeModule());
    }
    
    public JacksonCosmosSerializer(Class<T> documentClass) {
        // Use common defaults for our model classes
        this("customerId", "id");
    }

    @Override
    public JsonNode serialize(T document) throws Exception {
        JsonNode jsonNode = objectMapper.valueToTree(document);
        
        // Ensure the document has required Cosmos DB fields
        if (jsonNode.isObject()) {
            ObjectNode objectNode = (ObjectNode) jsonNode;
            
            // Ensure id field is present
            if (!objectNode.has("id") && objectNode.has(idPath)) {
                objectNode.set("id", objectNode.get(idPath));
            }
            
            // Add or update timestamp fields if they don't exist
            if (!objectNode.has("_ts")) {
                objectNode.put("_ts", System.currentTimeMillis() / 1000);
            }
        }
        
        return jsonNode;
    }

    @Override
    public String getPartitionKey(T document) throws Exception {
        JsonNode jsonNode = objectMapper.valueToTree(document);
        JsonNode partitionKeyNode = jsonNode.at("/" + partitionKeyPath);
        
        if (partitionKeyNode.isMissingNode()) {
            throw new IllegalArgumentException("Partition key field '" + partitionKeyPath + "' not found in document");
        }
        
        return partitionKeyNode.asText();
    }

    @Override
    public String getDocumentId(T document) throws Exception {
        JsonNode jsonNode = objectMapper.valueToTree(document);
        JsonNode idNode = jsonNode.at("/" + idPath);
        
        if (idNode.isMissingNode()) {
            throw new IllegalArgumentException("ID field '" + idPath + "' not found in document");
        }
        
        return idNode.asText();
    }
}