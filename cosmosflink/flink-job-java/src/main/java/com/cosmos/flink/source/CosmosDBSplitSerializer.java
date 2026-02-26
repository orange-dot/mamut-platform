package com.cosmos.flink.source;

import org.apache.flink.core.io.SimpleVersionedSerializer;
import org.apache.flink.core.memory.DataInputDeserializer;
import org.apache.flink.core.memory.DataOutputSerializer;

import java.io.IOException;

/**
 * Serializer for CosmosDBSplit
 */
public class CosmosDBSplitSerializer implements SimpleVersionedSerializer<CosmosDBSplit> {
    
    private static final int VERSION = 1;

    @Override
    public int getVersion() {
        return VERSION;
    }

    @Override
    public byte[] serialize(CosmosDBSplit split) throws IOException {
        DataOutputSerializer out = new DataOutputSerializer(256);
        
        out.writeUTF(split.splitId());
        out.writeUTF(split.getFeedRange());
        
        // Handle nullable continuation token
        if (split.getContinuationToken() != null) {
            out.writeBoolean(true);
            out.writeUTF(split.getContinuationToken());
        } else {
            out.writeBoolean(false);
        }
        
        out.writeBoolean(split.isFinished());
        
        return out.getCopyOfBuffer();
    }

    @Override
    public CosmosDBSplit deserialize(int version, byte[] serialized) throws IOException {
        if (version != VERSION) {
            throw new IOException("Unsupported version: " + version);
        }
        
        DataInputDeserializer in = new DataInputDeserializer(serialized);
        
        String splitId = in.readUTF();
        String feedRange = in.readUTF();
        
        String continuationToken = null;
        if (in.readBoolean()) {
            continuationToken = in.readUTF();
        }
        
        boolean isFinished = in.readBoolean();
        
        return new CosmosDBSplit(splitId, feedRange, continuationToken, isFinished);
    }
}