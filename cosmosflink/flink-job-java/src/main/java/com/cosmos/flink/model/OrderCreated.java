package com.cosmos.flink.model;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.math.BigDecimal;
import java.time.Instant;
import java.util.List;
import java.util.Map;
import java.util.Objects;

/**
 * Order created event model that matches the .NET producer schema
 */
@JsonIgnoreProperties(ignoreUnknown = true)
public class OrderCreated {
    
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
    
    @JsonProperty("source")
    private String source;
    
    @JsonProperty("region")
    private String region;
    
    @JsonProperty("orderItems")
    private List<OrderItem> orderItems;
    
    @JsonProperty("status")
    private String status;
    
    @JsonProperty("metadata")
    private Map<String, Object> metadata;

    // Default constructor for Jackson
    public OrderCreated() {}

    // Builder pattern for creating instances
    public static Builder builder() {
        return new Builder();
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

    public String getSource() { return source; }
    public void setSource(String source) { this.source = source; }

    public String getRegion() { return region; }
    public void setRegion(String region) { this.region = region; }

    public List<OrderItem> getOrderItems() { return orderItems; }
    public void setOrderItems(List<OrderItem> orderItems) { this.orderItems = orderItems; }

    public String getStatus() { return status; }
    public void setStatus(String status) { this.status = status; }

    public Map<String, Object> getMetadata() { return metadata; }
    public void setMetadata(Map<String, Object> metadata) { this.metadata = metadata; }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        OrderCreated that = (OrderCreated) o;
        return Objects.equals(id, that.id);
    }

    @Override
    public int hashCode() {
        return Objects.hash(id);
    }

    @Override
    public String toString() {
        return "OrderCreated{" +
                "id='" + id + '\'' +
                ", customerId='" + customerId + '\'' +
                ", amount=" + amount +
                ", currency='" + currency + '\'' +
                ", createdAt=" + createdAt +
                ", source='" + source + '\'' +
                ", region='" + region + '\'' +
                ", status='" + status + '\'' +
                '}';
    }

    // Builder class
    public static class Builder {
        private final OrderCreated order = new OrderCreated();

        public Builder id(String id) { order.id = id; return this; }
        public Builder customerId(String customerId) { order.customerId = customerId; return this; }
        public Builder amount(BigDecimal amount) { order.amount = amount; return this; }
        public Builder currency(String currency) { order.currency = currency; return this; }
        public Builder createdAt(Instant createdAt) { order.createdAt = createdAt; return this; }
        public Builder source(String source) { order.source = source; return this; }
        public Builder region(String region) { order.region = region; return this; }
        public Builder orderItems(List<OrderItem> orderItems) { order.orderItems = orderItems; return this; }
        public Builder status(String status) { order.status = status; return this; }
        public Builder metadata(Map<String, Object> metadata) { order.metadata = metadata; return this; }

        public OrderCreated build() { return order; }
    }
}