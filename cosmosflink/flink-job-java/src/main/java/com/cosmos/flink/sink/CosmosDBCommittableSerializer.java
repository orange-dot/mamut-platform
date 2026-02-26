package com.cosmos.flink.sink;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.datatype.jsr310.JavaTimeModule;
import org.apache.flink.core.io.SimpleVersionedSerializer;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.IOException;
import java.nio.charset.StandardCharsets;

/**
 * Serializer for CosmosDBCommittable objects used in checkpointing
 */
public class CosmosDBCommittableSerializer implements SimpleVersionedSerializer<CosmosDBCommittable> {
    
    private static final Logger LOG = LoggerFactory.getLogger(CosmosDBCommittableSerializer.class);
    private static final int VERSION = 1;
    
    private final ObjectMapper objectMapper;
    
    public CosmosDBCommittableSerializer() {
        this.objectMapper = new ObjectMapper();
        this.objectMapper.registerModule(new JavaTimeModule());
    }
    
    @Override
    public int getVersion() {
        return VERSION;
    }
    
    @Override
    public byte[] serialize(CosmosDBCommittable committable) throws IOException {
        try {
            String json = objectMapper.writeValueAsString(committable);
            return json.getBytes(StandardCharsets.UTF_8);
        } catch (JsonProcessingException e) {
            LOG.error("Failed to serialize CosmosDBCommittable: {}", committable, e);
            throw new IOException("Failed to serialize CosmosDBCommittable", e);
        }
    }
    
    @Override
    public CosmosDBCommittable deserialize(int version, byte[] serialized) throws IOException {
        if (version != VERSION) {
            throw new IOException("Unsupported version: " + version + ", expected: " + VERSION);
        }
        
        try {
            String json = new String(serialized, StandardCharsets.UTF_8);
            return objectMapper.readValue(json, CosmosDBCommittable.class);
        } catch (JsonProcessingException e) {
            LOG.error("Failed to deserialize CosmosDBCommittable from bytes", e);
            throw new IOException("Failed to deserialize CosmosDBCommittable", e);
        }
    }
}