package events

import (
	"time"

	"github.com/google/uuid"
)

// Event represents an event to be published to Service Bus
// This matches the EventData.cs schema in the .NET project
type Event struct {
	EventID       string                 `json:"eventId"`
	EventType     string                 `json:"eventType"`
	EntityID      string                 `json:"entityId"`
	Payload       map[string]interface{} `json:"payload"`
	Timestamp     time.Time              `json:"timestamp"`
	CorrelationID string                 `json:"correlationId,omitempty"`
	Metadata      map[string]string      `json:"metadata,omitempty"`
}

// NewEvent creates a new Event with auto-generated ID and timestamp
func NewEvent(eventType, entityID string, payload map[string]interface{}) *Event {
	return &Event{
		EventID:   uuid.New().String(),
		EventType: eventType,
		EntityID:  entityID,
		Payload:   payload,
		Timestamp: time.Now().UTC(),
	}
}

// WithCorrelationID sets the correlation ID for the event
func (e *Event) WithCorrelationID(correlationID string) *Event {
	e.CorrelationID = correlationID
	return e
}

// WithMetadata sets metadata for the event
func (e *Event) WithMetadata(metadata map[string]string) *Event {
	e.Metadata = metadata
	return e
}

// Common event types for IoT scenarios
const (
	// Device lifecycle events
	EventTypeDeviceConnected    = "DeviceConnected"
	EventTypeDeviceDisconnected = "DeviceDisconnected"
	EventTypeDeviceActivated    = "DeviceActivated"
	EventTypeDeviceOnline       = "DeviceOnline"
	EventTypeDeviceOffline      = "DeviceOffline"

	// Telemetry events
	EventTypeTelemetryReceived  = "TelemetryReceived"
	EventTypeSensorReading      = "SensorReading"
	EventTypeThresholdExceeded  = "ThresholdExceeded"
	EventTypeAnomalyDetected    = "AnomalyDetected"

	// Firmware events
	EventTypeFirmwareDownloaded      = "FirmwareDownloaded"
	EventTypeFirmwareInstalled       = "FirmwareInstalled"
	EventTypeFirmwareUpdateFailed    = "FirmwareUpdateFailed"
	EventTypeFirmwareRollbackComplete = "FirmwareRollbackComplete"

	// Maintenance events
	EventTypeMaintenanceConfirmed = "MaintenanceConfirmed"
	EventTypeMaintenanceCompleted = "MaintenanceCompleted"

	// Incident events
	EventTypeIncidentAcknowledged = "IncidentAcknowledged"
	EventTypeIncidentResolved     = "IncidentResolved"

	// Fleet events
	EventTypeFleetCommandBatchComplete = "FleetCommandBatchComplete"
	EventTypeFleetRollbackComplete     = "FleetRollbackComplete"
)
