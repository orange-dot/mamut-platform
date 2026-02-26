package com.cosmos.flink.source;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;

/**
 * Generic Jackson-based deserializer for Cosmos DB documents
 */
public class JacksonCosmosDeserializer<T> implements CosmosDBDocumentDeserializer<T> {
    
    private static final long serialVersionUID = 1L;
    
    private final ObjectMapper objectMapper;
    private final Class<T> targetType;

    public JacksonCosmosDeserializer(Class<T> targetType) {
        this.targetType = targetType;
        this.objectMapper = new ObjectMapper();
        this.objectMapper.findAndRegisterModules();
    }

    @Override
    public T deserialize(JsonNode document) throws Exception {
        return objectMapper.treeToValue(document, targetType);
    }

    @Override
    public Class<T> getTargetType() {
        return targetType;
    }
}