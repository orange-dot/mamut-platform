package com.cosmos.flink.sink;

import com.cosmos.flink.config.CosmosDBSinkConfig;
import org.apache.flink.api.connector.sink2.Committer;
import org.apache.flink.api.connector.sink2.Sink;
import org.apache.flink.api.connector.sink2.SinkWriter;
import org.apache.flink.api.connector.sink2.TwoPhaseCommittingSink;
import org.apache.flink.core.io.SimpleVersionedSerializer;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.IOException;
import java.util.Optional;

/**
 * Main Sink implementation for Azure Cosmos DB with Flink Sink v2 API
 * Provides exactly-once semantics through two-phase commit protocol
 * 
 * @param <T> The type of the input elements
 */
public class CosmosDBSink<T> implements TwoPhaseCommittingSink<T, CosmosDBCommittable> {
    
    private static final Logger LOG = LoggerFactory.getLogger(CosmosDBSink.class);
    private static final long serialVersionUID = 1L;
    
    private final CosmosDBSinkConfig config;
    private final CosmosDBDocumentSerializer<T> serializer;
    
    public CosmosDBSink(
            CosmosDBSinkConfig config,
            CosmosDBDocumentSerializer<T> serializer) {
        this.config = config;
        this.serializer = serializer;
        
        LOG.info("Initialized CosmosDBSink with config: {}", config);
    }
    
    @Override
    public TwoPhaseCommittingSink.PrecommittingSinkWriter<T, CosmosDBCommittable> createWriter(InitContext context) throws IOException {
        LOG.info("Creating CosmosDBWriter for subtask {}", context.getSubtaskId());
        return new CosmosDBWriter<>(config, serializer, context);
    }
    
    @Override
    public Committer<CosmosDBCommittable> createCommitter() throws IOException {
        LOG.info("Creating CosmosDBCommitter");
        return new CosmosDBCommitter();
    }
    
    @Override
    public SimpleVersionedSerializer<CosmosDBCommittable> getCommittableSerializer() {
        return new CosmosDBCommittableSerializer();
    }
    
    /**
     * Builder for CosmosDBSink
     */
    public static class Builder<T> {
        private CosmosDBSinkConfig config;
        private CosmosDBDocumentSerializer<T> serializer;
        
        public Builder<T> setConfig(CosmosDBSinkConfig config) {
            this.config = config;
            return this;
        }
        
        public Builder<T> setSerializer(CosmosDBDocumentSerializer<T> serializer) {
            this.serializer = serializer;
            return this;
        }
        
        public CosmosDBSink<T> build() {
            if (config == null) {
                throw new IllegalArgumentException("config is required");
            }
            if (serializer == null) {
                throw new IllegalArgumentException("serializer is required");
            }
            return new CosmosDBSink<>(config, serializer);
        }
    }
    
    public static <T> Builder<T> builder() {
        return new Builder<>();
    }
}