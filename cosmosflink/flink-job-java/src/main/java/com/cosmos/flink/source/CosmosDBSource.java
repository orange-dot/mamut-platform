package com.cosmos.flink.source;

import com.cosmos.flink.config.CosmosDBSourceConfig;
import org.apache.flink.api.connector.source.*;
import org.apache.flink.core.io.SimpleVersionedSerializer;

/**
 * FLIP-27 Source implementation for Azure Cosmos DB
 */
public class CosmosDBSource<T> implements Source<T, CosmosDBSplit, CosmosDBSourceState> {
    
    private static final long serialVersionUID = 1L;
    
    private final CosmosDBSourceConfig config;
    private final CosmosDBDocumentDeserializer<T> deserializer;

    public CosmosDBSource(
            CosmosDBSourceConfig config,
            CosmosDBDocumentDeserializer<T> deserializer) {
        this.config = config;
        this.deserializer = deserializer;
    }

    @Override
    public Boundedness getBoundedness() {
        return config.getSourceMode() == CosmosDBSourceConfig.SourceMode.CHANGE_FEED 
            ? Boundedness.CONTINUOUS_UNBOUNDED 
            : Boundedness.BOUNDED;
    }

    @Override
    public SplitEnumerator<CosmosDBSplit, CosmosDBSourceState> createEnumerator(
            SplitEnumeratorContext<CosmosDBSplit> context) {
        return new CosmosDBSplitEnumerator(context, config, null);
    }

    @Override
    public SplitEnumerator<CosmosDBSplit, CosmosDBSourceState> restoreEnumerator(
            SplitEnumeratorContext<CosmosDBSplit> context,
            CosmosDBSourceState checkpoint) {
        return new CosmosDBSplitEnumerator(context, config, checkpoint);
    }

    @Override
    public SimpleVersionedSerializer<CosmosDBSplit> getSplitSerializer() {
        return new CosmosDBSplitSerializer();
    }

    @Override
    public SimpleVersionedSerializer<CosmosDBSourceState> getEnumeratorCheckpointSerializer() {
        return new CosmosDBSourceStateSerializer();
    }

    @Override
    public SourceReader<T, CosmosDBSplit> createReader(SourceReaderContext context) {
        return new CosmosDBSourceReader<>(context, config, deserializer);
    }

    /**
     * Builder for CosmosDBSource
     */
    public static class Builder<T> {
        private CosmosDBSourceConfig config;
        private CosmosDBDocumentDeserializer<T> deserializer;

        public Builder<T> setConfig(CosmosDBSourceConfig config) {
            this.config = config;
            return this;
        }

        public Builder<T> setDeserializer(CosmosDBDocumentDeserializer<T> deserializer) {
            this.deserializer = deserializer;
            return this;
        }

        public CosmosDBSource<T> build() {
            if (config == null) {
                throw new IllegalArgumentException("config is required");
            }
            if (deserializer == null) {
                throw new IllegalArgumentException("deserializer is required");
            }
            return new CosmosDBSource<>(config, deserializer);
        }
    }

    public static <T> Builder<T> builder() {
        return new Builder<>();
    }
}