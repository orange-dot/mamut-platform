package simulation

import (
	"context"
	"fmt"
	"os"
	"time"

	"github.com/azure-durable/iot-simulator/internal/devices"
	"github.com/azure-durable/iot-simulator/internal/events"
	"github.com/rs/zerolog/log"
	"gopkg.in/yaml.v3"
)

// Scenario represents a predefined simulation scenario
type Scenario struct {
	Name        string        `yaml:"name"`
	Description string        `yaml:"description"`
	Steps       []ScenarioStep `yaml:"steps"`
}

// ScenarioStep represents a single step in a scenario
type ScenarioStep struct {
	Name      string                 `yaml:"name"`
	DeviceID  string                 `yaml:"deviceId"`
	EventType string                 `yaml:"eventType"`
	Payload   map[string]interface{} `yaml:"payload"`
	DelayMs   int                    `yaml:"delayMs"`
	// Conditional execution
	Condition string `yaml:"condition,omitempty"`
	// Loop execution
	Repeat int `yaml:"repeat,omitempty"`
}

// ScenarioRunner executes predefined scenarios
type ScenarioRunner struct {
	registry  *devices.Registry
	publisher events.EventPublisher
	scenarios map[string]*Scenario
}

// NewScenarioRunner creates a new scenario runner
func NewScenarioRunner(registry *devices.Registry, publisher events.EventPublisher) *ScenarioRunner {
	return &ScenarioRunner{
		registry:  registry,
		publisher: publisher,
		scenarios: make(map[string]*Scenario),
	}
}

// LoadScenario loads a scenario from a YAML file
func (r *ScenarioRunner) LoadScenario(filepath string) error {
	data, err := os.ReadFile(filepath)
	if err != nil {
		return fmt.Errorf("failed to read scenario file: %w", err)
	}

	var scenario Scenario
	if err := yaml.Unmarshal(data, &scenario); err != nil {
		return fmt.Errorf("failed to parse scenario file: %w", err)
	}

	r.scenarios[scenario.Name] = &scenario
	log.Info().
		Str("name", scenario.Name).
		Int("steps", len(scenario.Steps)).
		Msg("Scenario loaded")

	return nil
}

// LoadScenariosFromDir loads all scenarios from a directory
func (r *ScenarioRunner) LoadScenariosFromDir(dirPath string) error {
	entries, err := os.ReadDir(dirPath)
	if err != nil {
		return fmt.Errorf("failed to read scenarios directory: %w", err)
	}

	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}
		if ext := entry.Name()[len(entry.Name())-5:]; ext != ".yaml" && ext != ".yml" {
			continue
		}

		if err := r.LoadScenario(dirPath + "/" + entry.Name()); err != nil {
			log.Warn().Err(err).Str("file", entry.Name()).Msg("Failed to load scenario")
		}
	}

	return nil
}

// GetScenarios returns all loaded scenarios
func (r *ScenarioRunner) GetScenarios() []ScenarioInfo {
	var scenarios []ScenarioInfo
	for name, s := range r.scenarios {
		scenarios = append(scenarios, ScenarioInfo{
			Name:        name,
			Description: s.Description,
			StepCount:   len(s.Steps),
		})
	}
	return scenarios
}

// RunScenario executes a scenario by name
func (r *ScenarioRunner) RunScenario(ctx context.Context, name string) (*ScenarioResult, error) {
	scenario, exists := r.scenarios[name]
	if !exists {
		return nil, fmt.Errorf("scenario %s not found", name)
	}

	log.Info().
		Str("scenario", name).
		Int("steps", len(scenario.Steps)).
		Msg("Starting scenario execution")

	result := &ScenarioResult{
		ScenarioName: name,
		StartedAt:    time.Now(),
		StepResults:  make([]StepResult, 0),
	}

	for i, step := range scenario.Steps {
		select {
		case <-ctx.Done():
			result.Cancelled = true
			result.CompletedAt = time.Now()
			return result, ctx.Err()
		default:
		}

		// Apply delay before step
		if step.DelayMs > 0 {
			time.Sleep(time.Duration(step.DelayMs) * time.Millisecond)
		}

		stepResult := r.executeStep(ctx, &step)
		stepResult.StepIndex = i
		result.StepResults = append(result.StepResults, stepResult)

		if stepResult.Error != "" {
			log.Warn().
				Str("step", step.Name).
				Str("error", stepResult.Error).
				Msg("Scenario step failed")
		} else {
			log.Debug().
				Str("step", step.Name).
				Str("eventId", stepResult.EventID).
				Msg("Scenario step completed")
		}
	}

	result.CompletedAt = time.Now()
	result.Success = true

	for _, sr := range result.StepResults {
		if sr.Error != "" {
			result.Success = false
			break
		}
	}

	log.Info().
		Str("scenario", name).
		Bool("success", result.Success).
		Int("steps", len(result.StepResults)).
		Dur("duration", result.CompletedAt.Sub(result.StartedAt)).
		Msg("Scenario execution completed")

	return result, nil
}

func (r *ScenarioRunner) executeStep(ctx context.Context, step *ScenarioStep) StepResult {
	result := StepResult{
		StepName:  step.Name,
		DeviceID:  step.DeviceID,
		EventType: step.EventType,
		StartedAt: time.Now(),
	}

	repeat := 1
	if step.Repeat > 0 {
		repeat = step.Repeat
	}

	for i := 0; i < repeat; i++ {
		device, err := r.registry.Get(step.DeviceID)
		if err != nil {
			result.Error = fmt.Sprintf("device not found: %s", step.DeviceID)
			result.CompletedAt = time.Now()
			return result
		}

		// Create and emit the event
		event := events.NewEvent(step.EventType, step.DeviceID, step.Payload)

		if err := r.publisher.Publish(ctx, event); err != nil {
			result.Error = fmt.Sprintf("failed to publish event: %v", err)
			result.CompletedAt = time.Now()
			return result
		}

		result.EventID = event.EventID

		// Also queue on device for consistency
		device.EmitCustomEvent(step.EventType, step.Payload)
	}

	result.CompletedAt = time.Now()
	return result
}

// ScenarioInfo provides basic scenario information
type ScenarioInfo struct {
	Name        string `json:"name"`
	Description string `json:"description"`
	StepCount   int    `json:"stepCount"`
}

// ScenarioResult holds the result of a scenario execution
type ScenarioResult struct {
	ScenarioName string       `json:"scenarioName"`
	StartedAt    time.Time    `json:"startedAt"`
	CompletedAt  time.Time    `json:"completedAt"`
	Success      bool         `json:"success"`
	Cancelled    bool         `json:"cancelled"`
	StepResults  []StepResult `json:"stepResults"`
}

// StepResult holds the result of a single step execution
type StepResult struct {
	StepIndex   int       `json:"stepIndex"`
	StepName    string    `json:"stepName"`
	DeviceID    string    `json:"deviceId"`
	EventType   string    `json:"eventType"`
	EventID     string    `json:"eventId,omitempty"`
	Error       string    `json:"error,omitempty"`
	StartedAt   time.Time `json:"startedAt"`
	CompletedAt time.Time `json:"completedAt"`
}

// PredefinedScenarios returns built-in scenario definitions
func PredefinedScenarios() map[string]*Scenario {
	return map[string]*Scenario{
		"device-onboarding": {
			Name:        "device-onboarding",
			Description: "Simulates a device onboarding flow",
			Steps: []ScenarioStep{
				{Name: "Connect Device", DeviceID: "sensor-001", EventType: events.EventTypeDeviceConnected, Payload: map[string]interface{}{"deviceType": "sensor", "firmwareVersion": "1.0.0"}},
				{Name: "Send Telemetry 1", DeviceID: "sensor-001", EventType: events.EventTypeTelemetryReceived, Payload: map[string]interface{}{"temperature": 22.5, "humidity": 45.0}, DelayMs: 2000},
				{Name: "Send Telemetry 2", DeviceID: "sensor-001", EventType: events.EventTypeTelemetryReceived, Payload: map[string]interface{}{"temperature": 22.8, "humidity": 44.5}, DelayMs: 2000},
				{Name: "Send Telemetry 3", DeviceID: "sensor-001", EventType: events.EventTypeTelemetryReceived, Payload: map[string]interface{}{"temperature": 23.0, "humidity": 44.0}, DelayMs: 2000},
				{Name: "Activate Device", DeviceID: "sensor-001", EventType: events.EventTypeDeviceActivated, Payload: map[string]interface{}{"activatedBy": "system"}, DelayMs: 1000},
			},
		},
		"anomaly-detection": {
			Name:        "anomaly-detection",
			Description: "Simulates normal telemetry followed by an anomaly",
			Steps: []ScenarioStep{
				{Name: "Normal Reading 1", DeviceID: "sensor-001", EventType: events.EventTypeTelemetryReceived, Payload: map[string]interface{}{"temperature": 25.0}},
				{Name: "Normal Reading 2", DeviceID: "sensor-001", EventType: events.EventTypeTelemetryReceived, Payload: map[string]interface{}{"temperature": 25.5}, DelayMs: 2000},
				{Name: "Normal Reading 3", DeviceID: "sensor-001", EventType: events.EventTypeTelemetryReceived, Payload: map[string]interface{}{"temperature": 26.0}, DelayMs: 2000},
				{Name: "Spike - Anomaly", DeviceID: "sensor-001", EventType: events.EventTypeTelemetryReceived, Payload: map[string]interface{}{"temperature": 150.0}, DelayMs: 2000},
				{Name: "Threshold Exceeded", DeviceID: "sensor-001", EventType: events.EventTypeThresholdExceeded, Payload: map[string]interface{}{"threshold": 80, "actual": 150, "field": "temperature"}, DelayMs: 500},
				{Name: "Anomaly Detected", DeviceID: "sensor-001", EventType: events.EventTypeAnomalyDetected, Payload: map[string]interface{}{"severity": "critical", "anomalyType": "temperature_spike"}, DelayMs: 500},
			},
		},
		"firmware-update": {
			Name:        "firmware-update",
			Description: "Simulates a firmware update flow with progress and completion",
			Steps: []ScenarioStep{
				{Name: "Download Started", DeviceID: "hvac-001", EventType: "FirmwareDownloadStarted", Payload: map[string]interface{}{"version": "2.0.0", "packageSize": 5242880}},
				{Name: "Download Progress 25%", DeviceID: "hvac-001", EventType: "FirmwareDownloadProgress", Payload: map[string]interface{}{"progress": 25}, DelayMs: 3000},
				{Name: "Download Progress 50%", DeviceID: "hvac-001", EventType: "FirmwareDownloadProgress", Payload: map[string]interface{}{"progress": 50}, DelayMs: 3000},
				{Name: "Download Progress 75%", DeviceID: "hvac-001", EventType: "FirmwareDownloadProgress", Payload: map[string]interface{}{"progress": 75}, DelayMs: 3000},
				{Name: "Download Complete", DeviceID: "hvac-001", EventType: events.EventTypeFirmwareDownloaded, Payload: map[string]interface{}{"version": "2.0.0"}, DelayMs: 3000},
				{Name: "Install Started", DeviceID: "hvac-001", EventType: "FirmwareInstallStarted", Payload: map[string]interface{}{"version": "2.0.0"}, DelayMs: 2000},
				{Name: "Install Complete", DeviceID: "hvac-001", EventType: events.EventTypeFirmwareInstalled, Payload: map[string]interface{}{"version": "2.0.0"}, DelayMs: 5000},
				{Name: "Device Online", DeviceID: "hvac-001", EventType: events.EventTypeDeviceOnline, Payload: map[string]interface{}{"firmwareVersion": "2.0.0"}, DelayMs: 5000},
			},
		},
		"firmware-rollback": {
			Name:        "firmware-rollback",
			Description: "Simulates a failed firmware update with rollback",
			Steps: []ScenarioStep{
				{Name: "Download Complete", DeviceID: "hvac-001", EventType: events.EventTypeFirmwareDownloaded, Payload: map[string]interface{}{"version": "2.0.0"}},
				{Name: "Install Started", DeviceID: "hvac-001", EventType: "FirmwareInstallStarted", Payload: map[string]interface{}{"version": "2.0.0"}, DelayMs: 2000},
				{Name: "Install Failed", DeviceID: "hvac-001", EventType: events.EventTypeFirmwareUpdateFailed, Payload: map[string]interface{}{"version": "2.0.0", "error": "checksum_mismatch"}, DelayMs: 3000},
				{Name: "Rollback Started", DeviceID: "hvac-001", EventType: "FirmwareRollbackStarted", Payload: map[string]interface{}{"targetVersion": "1.0.0"}, DelayMs: 1000},
				{Name: "Rollback Complete", DeviceID: "hvac-001", EventType: events.EventTypeFirmwareRollbackComplete, Payload: map[string]interface{}{"version": "1.0.0"}, DelayMs: 5000},
				{Name: "Device Online", DeviceID: "hvac-001", EventType: events.EventTypeDeviceOnline, Payload: map[string]interface{}{"firmwareVersion": "1.0.0"}, DelayMs: 3000},
			},
		},
	}
}

// RegisterPredefinedScenarios loads all built-in scenarios
func (r *ScenarioRunner) RegisterPredefinedScenarios() {
	for name, scenario := range PredefinedScenarios() {
		r.scenarios[name] = scenario
		log.Debug().Str("name", name).Msg("Registered predefined scenario")
	}
}
