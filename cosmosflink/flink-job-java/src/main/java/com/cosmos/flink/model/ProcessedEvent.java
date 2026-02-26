package com.cosmos.flink.model;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.math.BigDecimal;
import java.time.Instant;
import java.util.Map;
import java.util.Objects;

/**
 * Processed event model for the secondary pipeline output
 */
@JsonIgnoreProperties(ignoreUnknown = true)
public class ProcessedEvent {
    
    @JsonProperty("id")
    private String id;
    
    @JsonProperty("customerId")
    private String customerId;
    
    @JsonProperty("amount")
    private BigDecimal amount;
    
    @JsonProperty("currency")
    private String currency;
    
    @JsonProperty("createdAt")
    private Instant createdAt;
    
    @JsonProperty("processedAt")
    private Instant processedAt;
    
    @JsonProperty("source")
    private String source;
    
    @JsonProperty("region")
    private String region;
    
    @JsonProperty("status")
    private String status;
    
    @JsonProperty("processingDurationMs")
    private long processingDurationMs;
    
    @JsonProperty("enrichments")
    private Map<String, Object> enrichments;
    
    @JsonProperty("metadata")
    private Map<String, Object> metadata;
    
    @JsonProperty("pipeline")
    private String pipeline;
    
    @JsonProperty("version")
    private String version;

    // Default constructor for Jackson
    public ProcessedEvent() {}

    // Static factory method to create from OrderCreated
    public static ProcessedEvent fromOrderCreated(OrderCreated order, String pipeline) {
        ProcessedEvent processed = new ProcessedEvent();
        processed.id = order.getId();
        processed.customerId = order.getCustomerId();
        processed.amount = order.getAmount();
        processed.currency = order.getCurrency();
        processed.createdAt = order.getCreatedAt();
        processed.processedAt = Instant.now();
        processed.source = order.getSource();
        processed.region = order.getRegion();
        processed.status = "Processed";
        processed.metadata = order.getMetadata();
        processed.pipeline = pipeline;
        processed.version = "1.0";
        return processed;
    }

    // Getters and setters
    public String getId() { return id; }
    public void setId(String id) { this.id = id; }

    public String getCustomerId() { return customerId; }
    public void setCustomerId(String customerId) { this.customerId = customerId; }

    public BigDecimal getAmount() { return amount; }
    public void setAmount(BigDecimal amount) { this.amount = amount; }

    public String getCurrency() { return currency; }
    public void setCurrency(String currency) { this.currency = currency; }

    public Instant getCreatedAt() { return createdAt; }
    public void setCreatedAt(Instant createdAt) { this.createdAt = createdAt; }

    public Instant getProcessedAt() { return processedAt; }
    public void setProcessedAt(Instant processedAt) { this.processedAt = processedAt; }

    public String getSource() { return source; }
    public void setSource(String source) { this.source = source; }

    public String getRegion() { return region; }
    public void setRegion(String region) { this.region = region; }

    public String getStatus() { return status; }
    public void setStatus(String status) { this.status = status; }

    public long getProcessingDurationMs() { return processingDurationMs; }
    public void setProcessingDurationMs(long processingDurationMs) { this.processingDurationMs = processingDurationMs; }

    public Map<String, Object> getEnrichments() { return enrichments; }
    public void setEnrichments(Map<String, Object> enrichments) { this.enrichments = enrichments; }

    public Map<String, Object> getMetadata() { return metadata; }
    public void setMetadata(Map<String, Object> metadata) { this.metadata = metadata; }

    public String getPipeline() { return pipeline; }
    public void setPipeline(String pipeline) { this.pipeline = pipeline; }

    public String getVersion() { return version; }
    public void setVersion(String version) { this.version = version; }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        ProcessedEvent that = (ProcessedEvent) o;
        return Objects.equals(id, that.id);
    }

    @Override
    public int hashCode() {
        return Objects.hash(id);
    }

    @Override
    public String toString() {
        return "ProcessedEvent{" +
                "id='" + id + '\'' +
                ", customerId='" + customerId + '\'' +
                ", amount=" + amount +
                ", currency='" + currency + '\'' +
                ", createdAt=" + createdAt +
                ", processedAt=" + processedAt +
                ", source='" + source + '\'' +
                ", region='" + region + '\'' +
                ", status='" + status + '\'' +
                ", pipeline='" + pipeline + '\'' +
                ", version='" + version + '\'' +
                '}';
    }
}