package api

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"time"

	"github.com/azure-durable/iot-simulator/internal/config"
	"github.com/azure-durable/iot-simulator/internal/devices"
	"github.com/azure-durable/iot-simulator/internal/simulation"
	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
	"github.com/rs/zerolog/log"
)

// Server is the REST API server
type Server struct {
	config         *config.Config
	router         *chi.Mux
	httpServer     *http.Server
	engine         *simulation.Engine
	registry       *devices.Registry
	scenarioRunner *simulation.ScenarioRunner
}

// NewServer creates a new API server
func NewServer(
	cfg *config.Config,
	engine *simulation.Engine,
	registry *devices.Registry,
	scenarioRunner *simulation.ScenarioRunner,
) *Server {
	s := &Server{
		config:         cfg,
		engine:         engine,
		registry:       registry,
		scenarioRunner: scenarioRunner,
	}

	s.setupRouter()
	return s
}

func (s *Server) setupRouter() {
	r := chi.NewRouter()

	// Middleware
	r.Use(middleware.RequestID)
	r.Use(middleware.RealIP)
	r.Use(middleware.Logger)
	r.Use(middleware.Recoverer)
	r.Use(middleware.Timeout(30 * time.Second))
	r.Use(corsMiddleware)

	// Routes
	r.Get("/health", s.healthCheck)

	r.Route("/api/v1", func(r chi.Router) {
		// Simulation control
		r.Route("/simulation", func(r chi.Router) {
			r.Get("/status", s.getSimulationStatus)
			r.Post("/start", s.startSimulation)
			r.Post("/stop", s.stopSimulation)
			r.Post("/pause", s.pauseSimulation)
			r.Post("/resume", s.resumeSimulation)
			r.Put("/mode", s.setSimulationMode)
		})

		// Device management
		r.Route("/devices", func(r chi.Router) {
			r.Get("/", s.listDevices)
			r.Get("/stats", s.getDeviceStats)
			r.Get("/{deviceId}", s.getDevice)
			r.Post("/{deviceId}/enable", s.enableDevice)
			r.Post("/{deviceId}/disable", s.disableDevice)
			r.Post("/{deviceId}/connect", s.connectDevice)
			r.Post("/{deviceId}/disconnect", s.disconnectDevice)
			r.Get("/{deviceId}/telemetry", s.getDeviceTelemetry)
		})

		// Event injection
		r.Route("/events", func(r chi.Router) {
			r.Post("/send", s.sendEvent)
		})

		// Scenario management
		r.Route("/scenarios", func(r chi.Router) {
			r.Get("/", s.listScenarios)
			r.Post("/run", s.runScenario)
		})
	})

	s.router = r
}

// Start starts the HTTP server
func (s *Server) Start() error {
	addr := fmt.Sprintf(":%d", s.config.API.Port)

	s.httpServer = &http.Server{
		Addr:         addr,
		Handler:      s.router,
		ReadTimeout:  s.config.API.ReadTimeout,
		WriteTimeout: s.config.API.WriteTimeout,
	}

	log.Info().Str("addr", addr).Msg("Starting API server")
	return s.httpServer.ListenAndServe()
}

// Shutdown gracefully shuts down the server
func (s *Server) Shutdown(ctx context.Context) error {
	return s.httpServer.Shutdown(ctx)
}

// Health check
func (s *Server) healthCheck(w http.ResponseWriter, r *http.Request) {
	status := s.engine.GetStatus()

	health := map[string]interface{}{
		"status":    "healthy",
		"timestamp": time.Now().UTC(),
		"simulation": map[string]interface{}{
			"state":           status.State,
			"devicesOnline":   status.DevicesOnline,
			"publisherConnected": status.PublisherConnected,
		},
	}

	writeJSON(w, http.StatusOK, health)
}

// Simulation handlers
func (s *Server) getSimulationStatus(w http.ResponseWriter, r *http.Request) {
	status := s.engine.GetStatus()
	writeJSON(w, http.StatusOK, status)
}

func (s *Server) startSimulation(w http.ResponseWriter, r *http.Request) {
	if err := s.engine.Start(r.Context()); err != nil {
		writeError(w, http.StatusInternalServerError, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, map[string]string{"message": "Simulation started"})
}

func (s *Server) stopSimulation(w http.ResponseWriter, r *http.Request) {
	if err := s.engine.Stop(); err != nil {
		writeError(w, http.StatusInternalServerError, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, map[string]string{"message": "Simulation stopped"})
}

func (s *Server) pauseSimulation(w http.ResponseWriter, r *http.Request) {
	s.engine.Pause()
	writeJSON(w, http.StatusOK, map[string]string{"message": "Simulation paused"})
}

func (s *Server) resumeSimulation(w http.ResponseWriter, r *http.Request) {
	s.engine.Resume()
	writeJSON(w, http.StatusOK, map[string]string{"message": "Simulation resumed"})
}

func (s *Server) setSimulationMode(w http.ResponseWriter, r *http.Request) {
	var req struct {
		Mode string `json:"mode"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	mode := simulation.ModeFromString(req.Mode)
	if !mode.IsValid() {
		writeError(w, http.StatusBadRequest, "Invalid mode")
		return
	}

	s.engine.SetMode(mode)
	writeJSON(w, http.StatusOK, map[string]string{"message": "Mode updated", "mode": string(mode)})
}

// Device handlers
func (s *Server) listDevices(w http.ResponseWriter, r *http.Request) {
	devices := s.registry.GetAllInfo()
	writeJSON(w, http.StatusOK, devices)
}

func (s *Server) getDeviceStats(w http.ResponseWriter, r *http.Request) {
	stats := s.registry.GetStats()
	writeJSON(w, http.StatusOK, stats)
}

func (s *Server) getDevice(w http.ResponseWriter, r *http.Request) {
	deviceID := chi.URLParam(r, "deviceId")
	device, err := s.registry.Get(deviceID)
	if err != nil {
		writeError(w, http.StatusNotFound, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, device.GetInfo())
}

func (s *Server) enableDevice(w http.ResponseWriter, r *http.Request) {
	deviceID := chi.URLParam(r, "deviceId")
	device, err := s.registry.Get(deviceID)
	if err != nil {
		writeError(w, http.StatusNotFound, err.Error())
		return
	}
	device.Enable()
	writeJSON(w, http.StatusOK, map[string]string{"message": "Device enabled", "deviceId": deviceID})
}

func (s *Server) disableDevice(w http.ResponseWriter, r *http.Request) {
	deviceID := chi.URLParam(r, "deviceId")
	device, err := s.registry.Get(deviceID)
	if err != nil {
		writeError(w, http.StatusNotFound, err.Error())
		return
	}
	device.Disable()
	writeJSON(w, http.StatusOK, map[string]string{"message": "Device disabled", "deviceId": deviceID})
}

func (s *Server) connectDevice(w http.ResponseWriter, r *http.Request) {
	deviceID := chi.URLParam(r, "deviceId")
	device, err := s.registry.Get(deviceID)
	if err != nil {
		writeError(w, http.StatusNotFound, err.Error())
		return
	}
	if err := device.Connect(r.Context()); err != nil {
		writeError(w, http.StatusInternalServerError, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, map[string]string{"message": "Device connected", "deviceId": deviceID})
}

func (s *Server) disconnectDevice(w http.ResponseWriter, r *http.Request) {
	deviceID := chi.URLParam(r, "deviceId")
	device, err := s.registry.Get(deviceID)
	if err != nil {
		writeError(w, http.StatusNotFound, err.Error())
		return
	}
	if err := device.Disconnect(r.Context()); err != nil {
		writeError(w, http.StatusInternalServerError, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, map[string]string{"message": "Device disconnected", "deviceId": deviceID})
}

func (s *Server) getDeviceTelemetry(w http.ResponseWriter, r *http.Request) {
	deviceID := chi.URLParam(r, "deviceId")
	device, err := s.registry.Get(deviceID)
	if err != nil {
		writeError(w, http.StatusNotFound, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, device.GetTelemetry())
}

// Event handlers
func (s *Server) sendEvent(w http.ResponseWriter, r *http.Request) {
	var req struct {
		DeviceID  string                 `json:"deviceId"`
		EventType string                 `json:"eventType"`
		Payload   map[string]interface{} `json:"payload"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	if req.DeviceID == "" || req.EventType == "" {
		writeError(w, http.StatusBadRequest, "deviceId and eventType are required")
		return
	}

	if err := s.engine.TriggerEvent(req.DeviceID, req.EventType, req.Payload); err != nil {
		writeError(w, http.StatusBadRequest, err.Error())
		return
	}

	writeJSON(w, http.StatusOK, map[string]string{
		"message":   "Event queued",
		"deviceId":  req.DeviceID,
		"eventType": req.EventType,
	})
}

// Scenario handlers
func (s *Server) listScenarios(w http.ResponseWriter, r *http.Request) {
	scenarios := s.scenarioRunner.GetScenarios()
	writeJSON(w, http.StatusOK, scenarios)
}

func (s *Server) runScenario(w http.ResponseWriter, r *http.Request) {
	var req struct {
		Name string `json:"name"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	if req.Name == "" {
		writeError(w, http.StatusBadRequest, "Scenario name is required")
		return
	}

	result, err := s.scenarioRunner.RunScenario(r.Context(), req.Name)
	if err != nil {
		writeError(w, http.StatusBadRequest, err.Error())
		return
	}

	writeJSON(w, http.StatusOK, result)
}

// Helper functions
func writeJSON(w http.ResponseWriter, status int, data interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(data)
}

func writeError(w http.ResponseWriter, status int, message string) {
	writeJSON(w, status, map[string]string{"error": message})
}

func corsMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization")

		if r.Method == "OPTIONS" {
			w.WriteHeader(http.StatusOK)
			return
		}

		next.ServeHTTP(w, r)
	})
}
