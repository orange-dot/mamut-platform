package devices

import (
	"context"
	"math/rand"
	"sync"
	"time"

	"github.com/azure-durable/iot-simulator/internal/config"
	"github.com/azure-durable/iot-simulator/internal/events"
	"github.com/rs/zerolog/log"
)

// DeviceState represents the current state of a device
type DeviceState string

const (
	StateOffline      DeviceState = "offline"
	StateConnecting   DeviceState = "connecting"
	StateOnline       DeviceState = "online"
	StateError        DeviceState = "error"
	StateUpdating     DeviceState = "updating"
	StateMaintenance  DeviceState = "maintenance"
)

// Device represents a simulated IoT device
type Device struct {
	ID              string
	Name            string
	Type            string
	Location        string
	Tags            map[string]string
	State           DeviceState
	FirmwareVersion string
	LastSeen        time.Time
	CreatedAt       time.Time

	config     config.DeviceConfig
	telemetry  map[string]interface{}
	mu         sync.RWMutex
	enabled    bool
	stopChan   chan struct{}
	eventsChan chan *events.Event
}

// NewDevice creates a new simulated device from config
func NewDevice(cfg config.DeviceConfig) *Device {
	d := &Device{
		ID:              cfg.ID,
		Name:            cfg.Name,
		Type:            cfg.Type,
		Location:        cfg.Location,
		Tags:            cfg.Tags,
		State:           StateOffline,
		FirmwareVersion: "1.0.0",
		CreatedAt:       time.Now(),
		config:          cfg,
		telemetry:       make(map[string]interface{}),
		enabled:         cfg.Enabled,
		stopChan:        make(chan struct{}),
		eventsChan:      make(chan *events.Event, 100),
	}

	// Initialize telemetry with default values
	for _, t := range cfg.Telemetry {
		if t.DefaultValue != nil {
			d.telemetry[t.Field] = t.DefaultValue
		} else {
			d.telemetry[t.Field] = generateInitialValue(t)
		}
	}

	return d
}

// Connect simulates device connection
func (d *Device) Connect(ctx context.Context) error {
	d.mu.Lock()
	defer d.mu.Unlock()

	d.State = StateConnecting
	d.LastSeen = time.Now()

	// Simulate connection delay
	time.Sleep(time.Duration(100+rand.Intn(400)) * time.Millisecond)

	d.State = StateOnline
	d.enabled = true

	log.Info().Str("deviceId", d.ID).Msg("Device connected")

	// Emit connection event
	event := events.NewEvent(events.EventTypeDeviceConnected, d.ID, map[string]interface{}{
		"deviceType":      d.Type,
		"firmwareVersion": d.FirmwareVersion,
		"location":        d.Location,
	})
	d.queueEvent(event)

	return nil
}

// Disconnect simulates device disconnection
func (d *Device) Disconnect(ctx context.Context) error {
	d.mu.Lock()
	defer d.mu.Unlock()

	d.State = StateOffline
	d.enabled = false

	log.Info().Str("deviceId", d.ID).Msg("Device disconnected")

	// Emit disconnection event
	event := events.NewEvent(events.EventTypeDeviceDisconnected, d.ID, map[string]interface{}{
		"reason": "graceful_shutdown",
	})
	d.queueEvent(event)

	return nil
}

// GenerateTelemetry generates new telemetry readings
func (d *Device) GenerateTelemetry() map[string]interface{} {
	d.mu.Lock()
	defer d.mu.Unlock()

	d.LastSeen = time.Now()

	for _, t := range d.config.Telemetry {
		d.telemetry[t.Field] = generateValue(t, d.telemetry[t.Field])
	}

	// Return a copy
	result := make(map[string]interface{})
	for k, v := range d.telemetry {
		result[k] = v
	}
	return result
}

// EmitTelemetryEvent creates and queues a telemetry event
func (d *Device) EmitTelemetryEvent() *events.Event {
	telemetry := d.GenerateTelemetry()

	event := events.NewEvent(events.EventTypeTelemetryReceived, d.ID, telemetry).
		WithMetadata(map[string]string{
			"deviceType": d.Type,
			"location":   d.Location,
		})

	d.queueEvent(event)
	return event
}

// EmitCustomEvent creates and queues a custom event
func (d *Device) EmitCustomEvent(eventType string, payload map[string]interface{}) *events.Event {
	d.mu.Lock()
	d.LastSeen = time.Now()
	d.mu.Unlock()

	event := events.NewEvent(eventType, d.ID, payload).
		WithMetadata(map[string]string{
			"deviceType": d.Type,
		})

	d.queueEvent(event)
	return event
}

// CheckThresholds evaluates telemetry against configured thresholds
func (d *Device) CheckThresholds() []*events.Event {
	d.mu.RLock()
	defer d.mu.RUnlock()

	var thresholdEvents []*events.Event

	for _, eventCfg := range d.config.Events {
		if eventCfg.Condition == "" || eventCfg.Type == events.EventTypeTelemetryReceived {
			continue
		}

		// Simple threshold evaluation
		if d.evaluateCondition(eventCfg.Condition) {
			event := events.NewEvent(eventCfg.Type, d.ID, map[string]interface{}{
				"condition": eventCfg.Condition,
				"telemetry": d.telemetry,
			})
			thresholdEvents = append(thresholdEvents, event)
			d.queueEvent(event)
		}
	}

	return thresholdEvents
}

// GetTelemetry returns current telemetry values
func (d *Device) GetTelemetry() map[string]interface{} {
	d.mu.RLock()
	defer d.mu.RUnlock()

	result := make(map[string]interface{})
	for k, v := range d.telemetry {
		result[k] = v
	}
	return result
}

// SetTelemetryValue sets a specific telemetry field
func (d *Device) SetTelemetryValue(field string, value interface{}) {
	d.mu.Lock()
	defer d.mu.Unlock()
	d.telemetry[field] = value
}

// GetState returns the current device state
func (d *Device) GetState() DeviceState {
	d.mu.RLock()
	defer d.mu.RUnlock()
	return d.State
}

// SetState updates the device state
func (d *Device) SetState(state DeviceState) {
	d.mu.Lock()
	defer d.mu.Unlock()
	d.State = state
}

// IsEnabled returns whether the device is enabled
func (d *Device) IsEnabled() bool {
	d.mu.RLock()
	defer d.mu.RUnlock()
	return d.enabled
}

// Enable enables the device
func (d *Device) Enable() {
	d.mu.Lock()
	defer d.mu.Unlock()
	d.enabled = true
}

// Disable disables the device
func (d *Device) Disable() {
	d.mu.Lock()
	defer d.mu.Unlock()
	d.enabled = false
}

// Events returns the events channel for this device
func (d *Device) Events() <-chan *events.Event {
	return d.eventsChan
}

// DrainEvents drains and returns all pending events
func (d *Device) DrainEvents() []*events.Event {
	var drained []*events.Event
	for {
		select {
		case evt := <-d.eventsChan:
			drained = append(drained, evt)
		default:
			return drained
		}
	}
}

// GetInfo returns device information
func (d *Device) GetInfo() DeviceInfo {
	d.mu.RLock()
	defer d.mu.RUnlock()

	return DeviceInfo{
		ID:              d.ID,
		Name:            d.Name,
		Type:            d.Type,
		Location:        d.Location,
		State:           d.State,
		FirmwareVersion: d.FirmwareVersion,
		Enabled:         d.enabled,
		LastSeen:        d.LastSeen,
		CreatedAt:       d.CreatedAt,
		Tags:            d.Tags,
		Telemetry:       d.GetTelemetry(),
	}
}

func (d *Device) queueEvent(event *events.Event) {
	select {
	case d.eventsChan <- event:
	default:
		log.Warn().Str("deviceId", d.ID).Msg("Event channel full, dropping event")
	}
}

func (d *Device) evaluateCondition(condition string) bool {
	// Simple condition evaluation
	// Format: "field > value" or "field == value"
	// In production, use a proper expression evaluator

	// For demo, randomly trigger 5% of the time
	return rand.Float64() < 0.05
}

// DeviceInfo represents serializable device information
type DeviceInfo struct {
	ID              string                 `json:"id"`
	Name            string                 `json:"name"`
	Type            string                 `json:"type"`
	Location        string                 `json:"location"`
	State           DeviceState            `json:"state"`
	FirmwareVersion string                 `json:"firmwareVersion"`
	Enabled         bool                   `json:"enabled"`
	LastSeen        time.Time              `json:"lastSeen"`
	CreatedAt       time.Time              `json:"createdAt"`
	Tags            map[string]string      `json:"tags"`
	Telemetry       map[string]interface{} `json:"telemetry"`
}

func generateInitialValue(t config.TelemetryField) interface{} {
	switch t.Type {
	case "float":
		return t.Min + rand.Float64()*(t.Max-t.Min)
	case "int":
		return int(t.Min) + rand.Intn(int(t.Max-t.Min))
	case "bool":
		return false
	case "string":
		return ""
	default:
		return nil
	}
}

func generateValue(t config.TelemetryField, current interface{}) interface{} {
	switch t.Type {
	case "float":
		curr, ok := current.(float64)
		if !ok {
			return generateInitialValue(t)
		}

		// Add noise
		noise := 0.0
		if t.NoisePercent > 0 {
			noise = curr * (t.NoisePercent / 100) * (rand.Float64()*2 - 1)
		}

		// Add trend
		trend := t.TrendRate * (rand.Float64()*2 - 1)

		newValue := curr + noise + trend

		// Clamp to min/max
		if newValue < t.Min {
			newValue = t.Min
		}
		if newValue > t.Max {
			newValue = t.Max
		}

		return newValue

	case "int":
		curr, ok := current.(int)
		if !ok {
			return generateInitialValue(t)
		}

		// Add random change
		change := rand.Intn(3) - 1 // -1, 0, or 1
		newValue := curr + change

		// Clamp
		if newValue < int(t.Min) {
			newValue = int(t.Min)
		}
		if newValue > int(t.Max) {
			newValue = int(t.Max)
		}

		return newValue

	case "bool":
		// 10% chance to toggle
		if rand.Float64() < 0.1 {
			if curr, ok := current.(bool); ok {
				return !curr
			}
		}
		return current

	default:
		return current
	}
}
