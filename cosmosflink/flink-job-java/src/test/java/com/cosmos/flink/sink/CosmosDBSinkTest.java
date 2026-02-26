package com.cosmos.flink.sink;

import com.cosmos.flink.config.CosmosDBSinkConfig;
import com.cosmos.flink.model.ProcessedEvent;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for CosmosDBSink configuration and instantiation
 */
public class CosmosDBSinkTest {

    @Test
    public void testSinkConfigurationBuilder() {
        CosmosDBSinkConfig config = CosmosDBSinkConfig.builder()
                .cosmosUri("https://test.documents.azure.com:443/")
                .cosmosKey("test-key")
                .database("test-db")
                .container("test-container")
                .batchSize(50)
                .batchTimeoutMs(3000)
                .maxRetryAttempts(5)
                .consistencyLevel("SESSION")
                .build();

        assertEquals("https://test.documents.azure.com:443/", config.getCosmosUri());
        assertEquals("test-key", config.getCosmosKey());
        assertEquals("test-db", config.getDatabase());
        assertEquals("test-container", config.getContainer());
        assertEquals(50, config.getBatchSize());
        assertEquals(3000, config.getBatchTimeoutMs());
        assertEquals(5, config.getMaxRetryAttempts());
        assertEquals("SESSION", config.getConsistencyLevel());
    }

    @Test
    public void testSinkConfigurationDefaults() {
        CosmosDBSinkConfig config = CosmosDBSinkConfig.builder()
                .cosmosUri("https://test.documents.azure.com:443/")
                .cosmosKey("test-key")
                .database("test-db")
                .container("test-container")
                .build();

        assertEquals(25, config.getBatchSize());
        assertEquals(5000, config.getBatchTimeoutMs());
        assertEquals(3, config.getMaxRetryAttempts());
        assertEquals("EVENTUAL", config.getConsistencyLevel());
        assertEquals(10, config.getMaxConcurrency());
        assertFalse(config.isEnableContentResponseOnWrite());
    }

    @Test
    public void testSinkConfigurationValidation() {
        // Test missing required fields
        assertThrows(IllegalArgumentException.class, () -> {
            CosmosDBSinkConfig.builder().build();
        });

        assertThrows(IllegalArgumentException.class, () -> {
            CosmosDBSinkConfig.builder()
                    .cosmosUri("https://test.documents.azure.com:443/")
                    .build();
        });

        assertThrows(IllegalArgumentException.class, () -> {
            CosmosDBSinkConfig.builder()
                    .cosmosUri("https://test.documents.azure.com:443/")
                    .cosmosKey("test-key")
                    .build();
        });

        // Test invalid batch size
        assertThrows(IllegalArgumentException.class, () -> {
            CosmosDBSinkConfig.builder()
                    .cosmosUri("https://test.documents.azure.com:443/")
                    .cosmosKey("test-key")
                    .database("test-db")
                    .container("test-container")
                    .batchSize(0)
                    .build();
        });

        // Test invalid retry attempts
        assertThrows(IllegalArgumentException.class, () -> {
            CosmosDBSinkConfig.builder()
                    .cosmosUri("https://test.documents.azure.com:443/")
                    .cosmosKey("test-key")
                    .database("test-db")
                    .container("test-container")
                    .maxRetryAttempts(-1)
                    .build();
        });
    }

    @Test
    public void testSinkCreation() {
        CosmosDBSinkConfig config = CosmosDBSinkConfig.builder()
                .cosmosUri("https://test.documents.azure.com:443/")
                .cosmosKey("test-key")
                .database("test-db")
                .container("test-container")
                .build();

        JacksonCosmosSerializer<ProcessedEvent> serializer = 
                new JacksonCosmosSerializer<>(ProcessedEvent.class);

        assertDoesNotThrow(() -> {
            CosmosDBSink<ProcessedEvent> sink = CosmosDBSink.<ProcessedEvent>builder()
                    .setConfig(config)
                    .setSerializer(serializer)
                    .build();
            
            assertNotNull(sink);
        });
    }

    @Test
    public void testSinkBuilderValidation() {
        // Test missing config
        assertThrows(IllegalArgumentException.class, () -> {
            CosmosDBSink.<ProcessedEvent>builder()
                    .setSerializer(new JacksonCosmosSerializer<>(ProcessedEvent.class))
                    .build();
        });

        // Test missing serializer
        assertThrows(IllegalArgumentException.class, () -> {
            CosmosDBSinkConfig config = CosmosDBSinkConfig.builder()
                    .cosmosUri("https://test.documents.azure.com:443/")
                    .cosmosKey("test-key")
                    .database("test-db")
                    .container("test-container")
                    .build();

            CosmosDBSink.<ProcessedEvent>builder()
                    .setConfig(config)
                    .build();
        });
    }

    @Test
    public void testSerializerCreation() {
        assertDoesNotThrow(() -> {
            JacksonCosmosSerializer<ProcessedEvent> serializer = 
                    new JacksonCosmosSerializer<>(ProcessedEvent.class);
            assertNotNull(serializer);
        });

        assertDoesNotThrow(() -> {
            JacksonCosmosSerializer<ProcessedEvent> serializer = 
                    new JacksonCosmosSerializer<>("customerId", "id");
            assertNotNull(serializer);
        });
    }
}