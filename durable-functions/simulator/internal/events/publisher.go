package events

import (
	"context"
	"encoding/json"
	"fmt"
	"sync"
	"time"

	"github.com/Azure/azure-sdk-for-go/sdk/messaging/azservicebus"
	"github.com/rs/zerolog/log"
)

// Publisher handles publishing events to Azure Service Bus
type Publisher struct {
	client    *azservicebus.Client
	sender    *azservicebus.Sender
	queueName string
	mu        sync.RWMutex
	connected bool
	metrics   *PublisherMetrics
}

// PublisherMetrics tracks publishing statistics
type PublisherMetrics struct {
	EventsSent   int64
	EventsFailed int64
	LastSentAt   time.Time
	mu           sync.RWMutex
}

// NewPublisher creates a new Service Bus publisher
func NewPublisher(connectionString, queueName string) (*Publisher, error) {
	client, err := azservicebus.NewClientFromConnectionString(connectionString, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to create Service Bus client: %w", err)
	}

	sender, err := client.NewSender(queueName, nil)
	if err != nil {
		client.Close(context.Background())
		return nil, fmt.Errorf("failed to create sender for queue %s: %w", queueName, err)
	}

	p := &Publisher{
		client:    client,
		sender:    sender,
		queueName: queueName,
		connected: true,
		metrics:   &PublisherMetrics{},
	}

	log.Info().Str("queue", queueName).Msg("Service Bus publisher initialized")
	return p, nil
}

// Publish sends an event to Service Bus
func (p *Publisher) Publish(ctx context.Context, event *Event) error {
	p.mu.RLock()
	if !p.connected {
		p.mu.RUnlock()
		return fmt.Errorf("publisher not connected")
	}
	p.mu.RUnlock()

	data, err := json.Marshal(event)
	if err != nil {
		return fmt.Errorf("failed to marshal event: %w", err)
	}

	msg := &azservicebus.Message{
		Body:          data,
		ContentType:   toPtr("application/json"),
		MessageID:     toPtr(event.EventID),
		CorrelationID: toPtr(event.CorrelationID),
		ApplicationProperties: map[string]interface{}{
			"eventType": event.EventType,
			"entityId":  event.EntityID,
		},
	}

	if err := p.sender.SendMessage(ctx, msg, nil); err != nil {
		p.metrics.mu.Lock()
		p.metrics.EventsFailed++
		p.metrics.mu.Unlock()
		return fmt.Errorf("failed to send message: %w", err)
	}

	p.metrics.mu.Lock()
	p.metrics.EventsSent++
	p.metrics.LastSentAt = time.Now()
	p.metrics.mu.Unlock()

	log.Debug().
		Str("eventId", event.EventID).
		Str("eventType", event.EventType).
		Str("entityId", event.EntityID).
		Msg("Event published")

	return nil
}

// PublishBatch sends multiple events to Service Bus
func (p *Publisher) PublishBatch(ctx context.Context, events []*Event) error {
	p.mu.RLock()
	if !p.connected {
		p.mu.RUnlock()
		return fmt.Errorf("publisher not connected")
	}
	p.mu.RUnlock()

	batch, err := p.sender.NewMessageBatch(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to create message batch: %w", err)
	}

	for _, event := range events {
		data, err := json.Marshal(event)
		if err != nil {
			log.Warn().Err(err).Str("eventId", event.EventID).Msg("Failed to marshal event, skipping")
			continue
		}

		msg := &azservicebus.Message{
			Body:          data,
			ContentType:   toPtr("application/json"),
			MessageID:     toPtr(event.EventID),
			CorrelationID: toPtr(event.CorrelationID),
			ApplicationProperties: map[string]interface{}{
				"eventType": event.EventType,
				"entityId":  event.EntityID,
			},
		}

		if err := batch.AddMessage(msg, nil); err != nil {
			// Batch is full, send what we have
			if err := p.sender.SendMessageBatch(ctx, batch, nil); err != nil {
				p.metrics.mu.Lock()
				p.metrics.EventsFailed += int64(len(events))
				p.metrics.mu.Unlock()
				return fmt.Errorf("failed to send batch: %w", err)
			}

			// Create new batch and add the message
			batch, err = p.sender.NewMessageBatch(ctx, nil)
			if err != nil {
				return fmt.Errorf("failed to create new batch: %w", err)
			}
			batch.AddMessage(msg, nil)
		}
	}

	// Send remaining messages
	if batch.NumMessages() > 0 {
		if err := p.sender.SendMessageBatch(ctx, batch, nil); err != nil {
			return fmt.Errorf("failed to send final batch: %w", err)
		}
	}

	p.metrics.mu.Lock()
	p.metrics.EventsSent += int64(len(events))
	p.metrics.LastSentAt = time.Now()
	p.metrics.mu.Unlock()

	log.Debug().Int("count", len(events)).Msg("Event batch published")
	return nil
}

// GetMetrics returns current publisher metrics
func (p *Publisher) GetMetrics() PublisherMetrics {
	p.metrics.mu.RLock()
	defer p.metrics.mu.RUnlock()
	return PublisherMetrics{
		EventsSent:   p.metrics.EventsSent,
		EventsFailed: p.metrics.EventsFailed,
		LastSentAt:   p.metrics.LastSentAt,
	}
}

// IsConnected returns the connection status
func (p *Publisher) IsConnected() bool {
	p.mu.RLock()
	defer p.mu.RUnlock()
	return p.connected
}

// Close gracefully closes the publisher
func (p *Publisher) Close(ctx context.Context) error {
	p.mu.Lock()
	defer p.mu.Unlock()

	p.connected = false

	if p.sender != nil {
		if err := p.sender.Close(ctx); err != nil {
			log.Warn().Err(err).Msg("Error closing sender")
		}
	}

	if p.client != nil {
		if err := p.client.Close(ctx); err != nil {
			return fmt.Errorf("failed to close client: %w", err)
		}
	}

	log.Info().Msg("Service Bus publisher closed")
	return nil
}

func toPtr(s string) *string {
	return &s
}

// MockPublisher is a publisher that logs events instead of sending to Service Bus
// Useful for development and testing
type MockPublisher struct {
	events  []*Event
	mu      sync.Mutex
	metrics *PublisherMetrics
}

// NewMockPublisher creates a mock publisher for testing
func NewMockPublisher() *MockPublisher {
	return &MockPublisher{
		events:  make([]*Event, 0),
		metrics: &PublisherMetrics{},
	}
}

// Publish logs the event instead of sending
func (m *MockPublisher) Publish(ctx context.Context, event *Event) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	m.events = append(m.events, event)
	m.metrics.EventsSent++
	m.metrics.LastSentAt = time.Now()

	log.Info().
		Str("eventId", event.EventID).
		Str("eventType", event.EventType).
		Str("entityId", event.EntityID).
		Interface("payload", event.Payload).
		Msg("[MOCK] Event published")

	return nil
}

// GetEvents returns all captured events
func (m *MockPublisher) GetEvents() []*Event {
	m.mu.Lock()
	defer m.mu.Unlock()
	return append([]*Event{}, m.events...)
}

// Clear removes all captured events
func (m *MockPublisher) Clear() {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.events = make([]*Event, 0)
}

// GetMetrics returns mock metrics
func (m *MockPublisher) GetMetrics() PublisherMetrics {
	m.metrics.mu.RLock()
	defer m.metrics.mu.RUnlock()
	return PublisherMetrics{
		EventsSent:   m.metrics.EventsSent,
		EventsFailed: m.metrics.EventsFailed,
		LastSentAt:   m.metrics.LastSentAt,
	}
}

// IsConnected always returns true for mock
func (m *MockPublisher) IsConnected() bool {
	return true
}

// Close is a no-op for mock
func (m *MockPublisher) Close(ctx context.Context) error {
	return nil
}

// EventPublisher is the interface for publishing events
type EventPublisher interface {
	Publish(ctx context.Context, event *Event) error
	GetMetrics() PublisherMetrics
	IsConnected() bool
	Close(ctx context.Context) error
}
