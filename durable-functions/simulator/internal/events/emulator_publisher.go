package events

import (
	"context"
	"encoding/json"
	"fmt"
	"sync"
	"time"

	"github.com/Azure/go-amqp"
	"github.com/rs/zerolog/log"
)

// EmulatorPublisher connects directly to Service Bus emulator via AMQP
// This bypasses the Azure SDK's localhost restriction for UseDevelopmentEmulator
type EmulatorPublisher struct {
	conn      *amqp.Conn
	session   *amqp.Session
	sender    *amqp.Sender
	host      string
	queueName string
	mu        sync.RWMutex
	connected bool
	metrics   *PublisherMetrics
}

// NewEmulatorPublisher creates a publisher that connects directly to Service Bus emulator
// host should be in format "servicebus-emulator:5672"
func NewEmulatorPublisher(host, queueName string) (*EmulatorPublisher, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	// Connect to AMQP endpoint
	amqpAddr := fmt.Sprintf("amqp://%s", host)
	log.Info().Str("address", amqpAddr).Msg("Connecting to Service Bus emulator via AMQP")

	conn, err := amqp.Dial(ctx, amqpAddr, &amqp.ConnOptions{
		SASLType: amqp.SASLTypeAnonymous(),
	})
	if err != nil {
		return nil, fmt.Errorf("failed to connect to Service Bus emulator at %s: %w", host, err)
	}

	session, err := conn.NewSession(ctx, nil)
	if err != nil {
		conn.Close()
		return nil, fmt.Errorf("failed to create AMQP session: %w", err)
	}

	// Create sender for the queue
	sender, err := session.NewSender(ctx, queueName, nil)
	if err != nil {
		session.Close(ctx)
		conn.Close()
		return nil, fmt.Errorf("failed to create sender for queue %s: %w", queueName, err)
	}

	p := &EmulatorPublisher{
		conn:      conn,
		session:   session,
		sender:    sender,
		host:      host,
		queueName: queueName,
		connected: true,
		metrics:   &PublisherMetrics{},
	}

	log.Info().
		Str("host", host).
		Str("queue", queueName).
		Msg("Service Bus emulator publisher initialized")

	return p, nil
}

// Publish sends an event to Service Bus emulator
func (p *EmulatorPublisher) Publish(ctx context.Context, event *Event) error {
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

	msg := &amqp.Message{
		Data: [][]byte{data},
		Properties: &amqp.MessageProperties{
			MessageID:     event.EventID,
			CorrelationID: &event.CorrelationID,
			ContentType:   toPtr("application/json"),
		},
		ApplicationProperties: map[string]interface{}{
			"eventType": event.EventType,
			"entityId":  event.EntityID,
		},
	}

	if err := p.sender.Send(ctx, msg, nil); err != nil {
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
		Msg("Event published to emulator")

	return nil
}

// GetMetrics returns current publisher metrics
func (p *EmulatorPublisher) GetMetrics() PublisherMetrics {
	p.metrics.mu.RLock()
	defer p.metrics.mu.RUnlock()
	return PublisherMetrics{
		EventsSent:   p.metrics.EventsSent,
		EventsFailed: p.metrics.EventsFailed,
		LastSentAt:   p.metrics.LastSentAt,
	}
}

// IsConnected returns the connection status
func (p *EmulatorPublisher) IsConnected() bool {
	p.mu.RLock()
	defer p.mu.RUnlock()
	return p.connected
}

// Close gracefully closes the publisher
func (p *EmulatorPublisher) Close(ctx context.Context) error {
	p.mu.Lock()
	defer p.mu.Unlock()

	p.connected = false

	if p.sender != nil {
		if err := p.sender.Close(ctx); err != nil {
			log.Warn().Err(err).Msg("Error closing sender")
		}
	}

	if p.session != nil {
		if err := p.session.Close(ctx); err != nil {
			log.Warn().Err(err).Msg("Error closing session")
		}
	}

	if p.conn != nil {
		if err := p.conn.Close(); err != nil {
			return fmt.Errorf("failed to close connection: %w", err)
		}
	}

	log.Info().Msg("Service Bus emulator publisher closed")
	return nil
}
