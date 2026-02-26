package com.cosmos.flink.source;

import org.apache.flink.core.io.SimpleVersionedSerializer;
import org.apache.flink.core.memory.DataInputDeserializer;
import org.apache.flink.core.memory.DataOutputSerializer;

import java.io.IOException;
import java.util.HashMap;
import java.util.Map;

/**
 * Serializer for CosmosDBSourceState
 */
public class CosmosDBSourceStateSerializer implements SimpleVersionedSerializer<CosmosDBSourceState> {
    
    private static final int VERSION = 1;

    @Override
    public int getVersion() {
        return VERSION;
    }

    @Override
    public byte[] serialize(CosmosDBSourceState state) throws IOException {
        DataOutputSerializer out = new DataOutputSerializer(512);
        
        out.writeBoolean(state.isInitialized());
        
        Map<String, String> tokens = state.getContinuationTokens();
        out.writeInt(tokens.size());
        
        for (Map.Entry<String, String> entry : tokens.entrySet()) {
            out.writeUTF(entry.getKey());
            out.writeUTF(entry.getValue());
        }
        
        return out.getCopyOfBuffer();
    }

    @Override
    public CosmosDBSourceState deserialize(int version, byte[] serialized) throws IOException {
        if (version != VERSION) {
            throw new IOException("Unsupported version: " + version);
        }
        
        DataInputDeserializer in = new DataInputDeserializer(serialized);
        
        boolean isInitialized = in.readBoolean();
        
        int tokenCount = in.readInt();
        Map<String, String> tokens = new HashMap<>(tokenCount);
        
        for (int i = 0; i < tokenCount; i++) {
            String splitId = in.readUTF();
            String token = in.readUTF();
            tokens.put(splitId, token);
        }
        
        return new CosmosDBSourceState(tokens, isInitialized);
    }
}