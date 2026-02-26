package main

import (
	"context"
	"flag"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/azure-durable/iot-simulator/internal/api"
	"github.com/azure-durable/iot-simulator/internal/config"
	"github.com/azure-durable/iot-simulator/internal/devices"
	"github.com/azure-durable/iot-simulator/internal/events"
	"github.com/azure-durable/iot-simulator/internal/simulation"
	"github.com/rs/zerolog"
	"github.com/rs/zerolog/log"
)

func main() {
	// Parse flags
	configPath := flag.String("config", "", "Path to configuration file")
	flag.Parse()

	// Load configuration
	cfg, err := config.Load(*configPath)
	if err != nil {
		log.Fatal().Err(err).Msg("Failed to load configuration")
	}

	// Setup logging
	setupLogging(cfg.Logging)

	log.Info().
		Str("mode", cfg.Simulator.Mode).
		Int("intervalMs", cfg.Simulator.IntervalMs).
		Int("devices", len(cfg.Devices)).
		Bool("autoStart", cfg.Simulator.AutoStart).
		Msg("IoT Device Simulator starting")

	// Create event publisher
	var publisher events.EventPublisher
	if cfg.ServiceBus.UseMock {
		log.Info().Msg("Using mock publisher (events will be logged only)")
		publisher = events.NewMockPublisher()
	} else if cfg.ServiceBus.UseEmulator {
		log.Info().
			Str("host", cfg.ServiceBus.EmulatorHost).
			Str("queue", cfg.ServiceBus.QueueName).
			Msg("Using emulator publisher (direct AMQP to Docker)")
		publisher, err = events.NewEmulatorPublisher(cfg.ServiceBus.EmulatorHost, cfg.ServiceBus.QueueName)
		if err != nil {
			log.Fatal().Err(err).Msg("Failed to create emulator publisher")
		}
	} else {
		publisher, err = events.NewPublisher(cfg.ServiceBus.ConnectionString, cfg.ServiceBus.QueueName)
		if err != nil {
			log.Fatal().Err(err).Msg("Failed to create Service Bus publisher")
		}
	}

	// Create device registry
	registry := devices.NewRegistry()
	if err := registry.LoadFromConfig(cfg); err != nil {
		log.Fatal().Err(err).Msg("Failed to load devices")
	}

	// Create simulation engine
	engine := simulation.NewEngine(cfg, registry, publisher)

	// Create scenario runner
	scenarioRunner := simulation.NewScenarioRunner(registry, publisher)
	scenarioRunner.RegisterPredefinedScenarios()

	// Load custom scenarios if directory exists
	if _, err := os.Stat("configs/scenarios"); err == nil {
		if err := scenarioRunner.LoadScenariosFromDir("configs/scenarios"); err != nil {
			log.Warn().Err(err).Msg("Failed to load custom scenarios")
		}
	}

	// Create API server
	server := api.NewServer(cfg, engine, registry, scenarioRunner)

	// Setup graceful shutdown
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	// Auto-start simulation if configured
	if cfg.Simulator.AutoStart {
		log.Info().Msg("Auto-starting simulation")
		if err := engine.Start(ctx); err != nil {
			log.Error().Err(err).Msg("Failed to auto-start simulation")
		}
	}

	// Start API server in goroutine
	go func() {
		if err := server.Start(); err != nil && err.Error() != "http: Server closed" {
			log.Fatal().Err(err).Msg("API server error")
		}
	}()

	log.Info().Int("port", cfg.API.Port).Msg("API server started")

	// Wait for shutdown signal
	<-ctx.Done()
	log.Info().Msg("Shutdown signal received")

	// Graceful shutdown
	shutdownCtx, cancel := context.WithTimeout(context.Background(), cfg.API.ShutdownTimeout)
	defer cancel()

	// Stop simulation
	if err := engine.Stop(); err != nil {
		log.Error().Err(err).Msg("Error stopping simulation")
	}

	// Shutdown API server
	if err := server.Shutdown(shutdownCtx); err != nil {
		log.Error().Err(err).Msg("Error shutting down API server")
	}

	// Close publisher
	if closer, ok := publisher.(interface{ Close(context.Context) error }); ok {
		if err := closer.Close(shutdownCtx); err != nil {
			log.Error().Err(err).Msg("Error closing publisher")
		}
	}

	log.Info().Msg("Simulator shutdown complete")
}

func setupLogging(cfg config.LoggingConfig) {
	// Set log level
	level, err := zerolog.ParseLevel(cfg.Level)
	if err != nil {
		level = zerolog.InfoLevel
	}
	zerolog.SetGlobalLevel(level)

	// Configure output format
	if cfg.Format == "console" {
		log.Logger = log.Output(zerolog.ConsoleWriter{
			Out:        os.Stdout,
			TimeFormat: time.RFC3339,
		})
	} else {
		// JSON format (default)
		zerolog.TimeFieldFormat = time.RFC3339
	}
}
