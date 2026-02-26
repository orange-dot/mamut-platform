package simulation

import (
	"context"
	"math/rand"
	"sync"
	"time"

	"github.com/azure-durable/iot-simulator/internal/config"
	"github.com/azure-durable/iot-simulator/internal/devices"
	"github.com/azure-durable/iot-simulator/internal/events"
	"github.com/rs/zerolog/log"
)

// SimulationState represents the current state of the simulation
type SimulationState string

const (
	StateStopped  SimulationState = "stopped"
	StateRunning  SimulationState = "running"
	StatePaused   SimulationState = "paused"
)

// Engine orchestrates the device simulation
type Engine struct {
	config    *config.Config
	registry  *devices.Registry
	publisher events.EventPublisher
	state     SimulationState
	mode      Mode
	metrics   *EngineMetrics

	mu        sync.RWMutex
	ctx       context.Context
	cancel    context.CancelFunc
	wg        sync.WaitGroup
}

// EngineMetrics tracks simulation statistics
type EngineMetrics struct {
	EventsGenerated   int64
	EventsPublished   int64
	EventsFailed      int64
	CyclesCompleted   int64
	StartedAt         time.Time
	LastCycleAt       time.Time
	mu                sync.RWMutex
}

// NewEngine creates a new simulation engine
func NewEngine(cfg *config.Config, registry *devices.Registry, publisher events.EventPublisher) *Engine {
	return &Engine{
		config:    cfg,
		registry:  registry,
		publisher: publisher,
		state:     StateStopped,
		mode:      ModeFromString(cfg.Simulator.Mode),
		metrics:   &EngineMetrics{},
	}
}

// Start begins the simulation
func (e *Engine) Start(ctx context.Context) error {
	e.mu.Lock()
	defer e.mu.Unlock()

	if e.state == StateRunning {
		return nil // Already running
	}

	e.ctx, e.cancel = context.WithCancel(ctx)
	e.state = StateRunning
	e.metrics.StartedAt = time.Now()

	// Connect all enabled devices
	for _, device := range e.registry.GetEnabled() {
		device.Connect(e.ctx)
	}

	// Start the simulation loop
	e.wg.Add(1)
	go e.runLoop()

	// Start event publisher goroutine
	e.wg.Add(1)
	go e.publishEvents()

	log.Info().
		Str("mode", string(e.mode)).
		Int("devices", e.registry.EnabledCount()).
		Int("intervalMs", e.config.Simulator.IntervalMs).
		Msg("Simulation started")

	return nil
}

// Stop halts the simulation
func (e *Engine) Stop() error {
	e.mu.Lock()
	defer e.mu.Unlock()

	if e.state != StateRunning {
		return nil
	}

	e.cancel()
	e.wg.Wait()
	e.state = StateStopped

	// Disconnect all devices
	for _, device := range e.registry.GetAll() {
		device.Disconnect(context.Background())
	}

	log.Info().Msg("Simulation stopped")
	return nil
}

// Pause pauses the simulation
func (e *Engine) Pause() {
	e.mu.Lock()
	defer e.mu.Unlock()
	if e.state == StateRunning {
		e.state = StatePaused
		log.Info().Msg("Simulation paused")
	}
}

// Resume resumes a paused simulation
func (e *Engine) Resume() {
	e.mu.Lock()
	defer e.mu.Unlock()
	if e.state == StatePaused {
		e.state = StateRunning
		log.Info().Msg("Simulation resumed")
	}
}

// GetState returns the current simulation state
func (e *Engine) GetState() SimulationState {
	e.mu.RLock()
	defer e.mu.RUnlock()
	return e.state
}

// GetMode returns the current simulation mode
func (e *Engine) GetMode() Mode {
	e.mu.RLock()
	defer e.mu.RUnlock()
	return e.mode
}

// SetMode changes the simulation mode
func (e *Engine) SetMode(mode Mode) {
	e.mu.Lock()
	defer e.mu.Unlock()
	e.mode = mode
	log.Info().Str("mode", string(mode)).Msg("Simulation mode changed")
}

// GetMetrics returns current engine metrics
func (e *Engine) GetMetrics() EngineMetrics {
	e.metrics.mu.RLock()
	defer e.metrics.mu.RUnlock()
	return EngineMetrics{
		EventsGenerated: e.metrics.EventsGenerated,
		EventsPublished: e.metrics.EventsPublished,
		EventsFailed:    e.metrics.EventsFailed,
		CyclesCompleted: e.metrics.CyclesCompleted,
		StartedAt:       e.metrics.StartedAt,
		LastCycleAt:     e.metrics.LastCycleAt,
	}
}

// TriggerEvent manually triggers an event for a device
func (e *Engine) TriggerEvent(deviceID string, eventType string, payload map[string]interface{}) error {
	device, err := e.registry.Get(deviceID)
	if err != nil {
		return err
	}

	event := device.EmitCustomEvent(eventType, payload)
	log.Debug().
		Str("deviceId", deviceID).
		Str("eventType", eventType).
		Str("eventId", event.EventID).
		Msg("Manual event triggered")

	return nil
}

func (e *Engine) runLoop() {
	defer e.wg.Done()

	interval := time.Duration(e.config.Simulator.IntervalMs) * time.Millisecond
	jitter := time.Duration(e.config.Simulator.JitterMs) * time.Millisecond

	ticker := time.NewTicker(interval)
	defer ticker.Stop()

	for {
		select {
		case <-e.ctx.Done():
			return

		case <-ticker.C:
			e.mu.RLock()
			state := e.state
			mode := e.mode
			e.mu.RUnlock()

			if state != StateRunning {
				continue
			}

			// Run simulation cycle based on mode
			switch mode {
			case ModeConstant:
				e.runConstantCycle()
			case ModeRandom:
				e.runRandomCycle(jitter)
			case ModeBurst:
				e.runBurstCycle()
			case ModeScenario:
				// Scenario mode is handled separately
				e.runConstantCycle()
			}

			e.metrics.mu.Lock()
			e.metrics.CyclesCompleted++
			e.metrics.LastCycleAt = time.Now()
			e.metrics.mu.Unlock()
		}
	}
}

func (e *Engine) runConstantCycle() {
	for _, device := range e.registry.GetEnabled() {
		if device.GetState() != devices.StateOnline {
			continue
		}

		device.EmitTelemetryEvent()
		device.CheckThresholds()

		e.metrics.mu.Lock()
		e.metrics.EventsGenerated++
		e.metrics.mu.Unlock()
	}
}

func (e *Engine) runRandomCycle(jitter time.Duration) {
	enabledDevices := e.registry.GetEnabled()

	for _, device := range enabledDevices {
		if device.GetState() != devices.StateOnline {
			continue
		}

		// Random jitter before emitting
		if jitter > 0 {
			time.Sleep(time.Duration(rand.Int63n(int64(jitter))))
		}

		// 90% chance to emit telemetry
		if rand.Float64() < 0.9 {
			device.EmitTelemetryEvent()
			e.metrics.mu.Lock()
			e.metrics.EventsGenerated++
			e.metrics.mu.Unlock()
		}

		device.CheckThresholds()
	}
}

func (e *Engine) runBurstCycle() {
	burstSize := e.config.Simulator.BurstSize
	burstInterval := time.Duration(e.config.Simulator.BurstIntervalMs) * time.Millisecond

	enabledDevices := e.registry.GetEnabled()

	for i := 0; i < burstSize; i++ {
		for _, device := range enabledDevices {
			if device.GetState() != devices.StateOnline {
				continue
			}

			device.EmitTelemetryEvent()

			e.metrics.mu.Lock()
			e.metrics.EventsGenerated++
			e.metrics.mu.Unlock()
		}

		if burstInterval > 0 && i < burstSize-1 {
			time.Sleep(burstInterval)
		}
	}
}

func (e *Engine) publishEvents() {
	defer e.wg.Done()

	publishTicker := time.NewTicker(100 * time.Millisecond)
	defer publishTicker.Stop()

	for {
		select {
		case <-e.ctx.Done():
			// Drain remaining events before exiting
			e.drainAndPublishAll()
			return

		case <-publishTicker.C:
			e.drainAndPublishAll()
		}
	}
}

func (e *Engine) drainAndPublishAll() {
	for _, device := range e.registry.GetAll() {
		events := device.DrainEvents()
		for _, event := range events {
			if err := e.publisher.Publish(e.ctx, event); err != nil {
				e.metrics.mu.Lock()
				e.metrics.EventsFailed++
				e.metrics.mu.Unlock()

				log.Warn().
					Err(err).
					Str("eventId", event.EventID).
					Msg("Failed to publish event")
			} else {
				e.metrics.mu.Lock()
				e.metrics.EventsPublished++
				e.metrics.mu.Unlock()
			}
		}
	}
}

// GetStatus returns comprehensive simulation status
func (e *Engine) GetStatus() SimulationStatus {
	e.mu.RLock()
	state := e.state
	mode := e.mode
	e.mu.RUnlock()

	metrics := e.GetMetrics()
	pubMetrics := e.publisher.GetMetrics()
	regStats := e.registry.GetStats()

	uptime := time.Duration(0)
	if !metrics.StartedAt.IsZero() {
		uptime = time.Since(metrics.StartedAt)
	}

	return SimulationStatus{
		State:           state,
		Mode:            mode,
		Uptime:          uptime,
		DevicesTotal:    regStats.Total,
		DevicesEnabled:  regStats.Enabled,
		DevicesOnline:   regStats.ByState[devices.StateOnline],
		EventsGenerated: metrics.EventsGenerated,
		EventsPublished: pubMetrics.EventsSent,
		EventsFailed:    metrics.EventsFailed + pubMetrics.EventsFailed,
		CyclesCompleted: metrics.CyclesCompleted,
		LastCycleAt:     metrics.LastCycleAt,
		IntervalMs:      e.config.Simulator.IntervalMs,
		PublisherConnected: e.publisher.IsConnected(),
	}
}

// SimulationStatus represents the current status of the simulation
type SimulationStatus struct {
	State              SimulationState `json:"state"`
	Mode               Mode            `json:"mode"`
	Uptime             time.Duration   `json:"uptime"`
	DevicesTotal       int             `json:"devicesTotal"`
	DevicesEnabled     int             `json:"devicesEnabled"`
	DevicesOnline      int             `json:"devicesOnline"`
	EventsGenerated    int64           `json:"eventsGenerated"`
	EventsPublished    int64           `json:"eventsPublished"`
	EventsFailed       int64           `json:"eventsFailed"`
	CyclesCompleted    int64           `json:"cyclesCompleted"`
	LastCycleAt        time.Time       `json:"lastCycleAt"`
	IntervalMs         int             `json:"intervalMs"`
	PublisherConnected bool            `json:"publisherConnected"`
}
