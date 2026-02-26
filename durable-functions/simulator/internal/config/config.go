package config

import (
	"fmt"
	"os"
	"strings"
	"time"

	"github.com/spf13/viper"
)

// Config holds all simulator configuration
type Config struct {
	ServiceBus ServiceBusConfig `mapstructure:"servicebus"`
	Simulator  SimulatorConfig  `mapstructure:"simulator"`
	API        APIConfig        `mapstructure:"api"`
	Devices    []DeviceConfig   `mapstructure:"devices"`
	Logging    LoggingConfig    `mapstructure:"logging"`
}

// ServiceBusConfig holds Service Bus connection settings
type ServiceBusConfig struct {
	ConnectionString string `mapstructure:"connectionString"`
	QueueName        string `mapstructure:"queueName"`
	UseMock          bool   `mapstructure:"useMock"`
	UseEmulator      bool   `mapstructure:"useEmulator"`  // Use direct AMQP for Docker emulator
	EmulatorHost     string `mapstructure:"emulatorHost"` // e.g., "servicebus-emulator:5672"
}

// SimulatorConfig holds simulator behavior settings
type SimulatorConfig struct {
	Mode            string        `mapstructure:"mode"` // constant, random, scenario, burst
	IntervalMs      int           `mapstructure:"intervalMs"`
	JitterMs        int           `mapstructure:"jitterMs"`
	BurstSize       int           `mapstructure:"burstSize"`
	BurstIntervalMs int           `mapstructure:"burstIntervalMs"`
	AutoStart       bool          `mapstructure:"autoStart"`
	ScenarioFile    string        `mapstructure:"scenarioFile"`
}

// APIConfig holds REST API settings
type APIConfig struct {
	Port            int           `mapstructure:"port"`
	ReadTimeout     time.Duration `mapstructure:"readTimeout"`
	WriteTimeout    time.Duration `mapstructure:"writeTimeout"`
	ShutdownTimeout time.Duration `mapstructure:"shutdownTimeout"`
}

// LoggingConfig holds logging settings
type LoggingConfig struct {
	Level  string `mapstructure:"level"`
	Format string `mapstructure:"format"` // json, console
}

// DeviceConfig holds configuration for a simulated device
type DeviceConfig struct {
	ID          string            `mapstructure:"id" yaml:"id"`
	Name        string            `mapstructure:"name" yaml:"name"`
	Type        string            `mapstructure:"type" yaml:"type"` // sensor, gateway, actuator
	Enabled     bool              `mapstructure:"enabled" yaml:"enabled"`
	Location    string            `mapstructure:"location" yaml:"location"`
	Tags        map[string]string `mapstructure:"tags" yaml:"tags"`
	Telemetry   []TelemetryField  `mapstructure:"telemetry" yaml:"telemetry"`
	Events      []EventConfig     `mapstructure:"events" yaml:"events"`
}

// TelemetryField defines a telemetry data point
type TelemetryField struct {
	Field        string  `mapstructure:"field" yaml:"field"`
	Type         string  `mapstructure:"type" yaml:"type"` // float, int, bool, string
	Min          float64 `mapstructure:"min" yaml:"min"`
	Max          float64 `mapstructure:"max" yaml:"max"`
	Unit         string  `mapstructure:"unit" yaml:"unit"`
	DefaultValue any     `mapstructure:"defaultValue" yaml:"defaultValue"`
	// For realistic simulation
	NoisePercent float64 `mapstructure:"noisePercent" yaml:"noisePercent"` // Random noise as percentage
	TrendRate    float64 `mapstructure:"trendRate" yaml:"trendRate"`       // Gradual drift rate
}

// EventConfig defines events a device can emit
type EventConfig struct {
	Type       string `mapstructure:"type" yaml:"type"`
	IntervalMs int    `mapstructure:"interval" yaml:"interval"` // 0 = manual trigger only
	Condition  string `mapstructure:"condition" yaml:"condition"`
}

// Load reads configuration from file and environment
func Load(configPath string) (*Config, error) {
	v := viper.New()

	// Set defaults
	setDefaults(v)

	// Read config file if specified
	if configPath != "" {
		v.SetConfigFile(configPath)
	} else {
		v.SetConfigName("default")
		v.SetConfigType("yaml")
		v.AddConfigPath("./configs")
		v.AddConfigPath("/app/configs")
		v.AddConfigPath(".")
	}

	if err := v.ReadInConfig(); err != nil {
		if _, ok := err.(viper.ConfigFileNotFoundError); !ok {
			return nil, fmt.Errorf("error reading config file: %w", err)
		}
		// Config file not found is ok, we'll use defaults and env vars
	}

	// Read environment variables
	v.SetEnvPrefix("SIMULATOR")
	v.SetEnvKeyReplacer(strings.NewReplacer(".", "_"))
	v.AutomaticEnv()

	// Override from specific env vars
	if connStr := os.Getenv("SERVICEBUS_CONNECTION_STRING"); connStr != "" {
		v.Set("servicebus.connectionString", connStr)
	}
	if mode := os.Getenv("SIMULATOR_MODE"); mode != "" {
		v.Set("simulator.mode", mode)
	}
	if interval := os.Getenv("SIMULATOR_INTERVAL_MS"); interval != "" {
		v.Set("simulator.intervalMs", interval)
	}

	var cfg Config
	if err := v.Unmarshal(&cfg); err != nil {
		return nil, fmt.Errorf("error unmarshaling config: %w", err)
	}

	// Validate required fields
	if err := cfg.Validate(); err != nil {
		return nil, err
	}

	return &cfg, nil
}

// Validate checks if the configuration is valid
func (c *Config) Validate() error {
	if !c.ServiceBus.UseMock && !c.ServiceBus.UseEmulator && c.ServiceBus.ConnectionString == "" {
		return fmt.Errorf("servicebus.connectionString is required when not using mock or emulator")
	}

	if c.ServiceBus.UseEmulator && c.ServiceBus.EmulatorHost == "" {
		return fmt.Errorf("servicebus.emulatorHost is required when using emulator mode")
	}

	if c.ServiceBus.QueueName == "" {
		return fmt.Errorf("servicebus.queueName is required")
	}

	validModes := map[string]bool{"constant": true, "random": true, "scenario": true, "burst": true}
	if !validModes[c.Simulator.Mode] {
		return fmt.Errorf("invalid simulator mode: %s (valid: constant, random, scenario, burst)", c.Simulator.Mode)
	}

	if c.Simulator.IntervalMs < 100 {
		return fmt.Errorf("simulator.intervalMs must be at least 100")
	}

	return nil
}

func setDefaults(v *viper.Viper) {
	// Service Bus defaults
	v.SetDefault("servicebus.queueName", "device-events")
	v.SetDefault("servicebus.useMock", false)
	v.SetDefault("servicebus.useEmulator", false)
	v.SetDefault("servicebus.emulatorHost", "servicebus-emulator:5672")

	// Simulator defaults
	v.SetDefault("simulator.mode", "random")
	v.SetDefault("simulator.intervalMs", 5000)
	v.SetDefault("simulator.jitterMs", 1000)
	v.SetDefault("simulator.burstSize", 10)
	v.SetDefault("simulator.burstIntervalMs", 100)
	v.SetDefault("simulator.autoStart", false)

	// API defaults
	v.SetDefault("api.port", 8080)
	v.SetDefault("api.readTimeout", "15s")
	v.SetDefault("api.writeTimeout", "15s")
	v.SetDefault("api.shutdownTimeout", "30s")

	// Logging defaults
	v.SetDefault("logging.level", "info")
	v.SetDefault("logging.format", "json")

	// Default devices if none configured
	v.SetDefault("devices", []DeviceConfig{
		{
			ID:       "sensor-001",
			Name:     "Building A Temperature Sensor",
			Type:     "sensor",
			Enabled:  true,
			Location: "Building A, Floor 1",
			Telemetry: []TelemetryField{
				{Field: "temperature", Type: "float", Min: 18.0, Max: 30.0, Unit: "celsius", NoisePercent: 2},
				{Field: "humidity", Type: "float", Min: 30.0, Max: 70.0, Unit: "percent", NoisePercent: 3},
			},
			Events: []EventConfig{
				{Type: "TelemetryReceived", IntervalMs: 5000},
				{Type: "ThresholdExceeded", Condition: "temperature > 28"},
			},
		},
		{
			ID:       "sensor-002",
			Name:     "Building A Motion Detector",
			Type:     "sensor",
			Enabled:  true,
			Location: "Building A, Entrance",
			Telemetry: []TelemetryField{
				{Field: "motionDetected", Type: "bool", DefaultValue: false},
				{Field: "batteryLevel", Type: "float", Min: 0, Max: 100, Unit: "percent"},
			},
			Events: []EventConfig{
				{Type: "TelemetryReceived", IntervalMs: 10000},
				{Type: "MotionDetected", Condition: "motionDetected == true"},
			},
		},
		{
			ID:       "meter-001",
			Name:     "Smart Power Meter",
			Type:     "sensor",
			Enabled:  true,
			Location: "Building A, Utility Room",
			Telemetry: []TelemetryField{
				{Field: "powerConsumption", Type: "float", Min: 0, Max: 5000, Unit: "watts", NoisePercent: 5},
				{Field: "voltage", Type: "float", Min: 110, Max: 130, Unit: "volts", NoisePercent: 1},
				{Field: "current", Type: "float", Min: 0, Max: 50, Unit: "amps", NoisePercent: 3},
			},
			Events: []EventConfig{
				{Type: "TelemetryReceived", IntervalMs: 3000},
				{Type: "ThresholdExceeded", Condition: "powerConsumption > 4500"},
			},
		},
		{
			ID:       "hvac-001",
			Name:     "HVAC Controller",
			Type:     "actuator",
			Enabled:  true,
			Location: "Building A, Rooftop",
			Telemetry: []TelemetryField{
				{Field: "setpoint", Type: "float", Min: 16, Max: 28, Unit: "celsius", DefaultValue: 22.0},
				{Field: "mode", Type: "string", DefaultValue: "auto"},
				{Field: "fanSpeed", Type: "int", Min: 0, Max: 100, Unit: "percent", DefaultValue: 50},
				{Field: "compressorStatus", Type: "bool", DefaultValue: true},
			},
			Events: []EventConfig{
				{Type: "TelemetryReceived", IntervalMs: 15000},
				{Type: "CommandReceived"},
			},
		},
	})
}

// GetDeviceByID finds a device config by ID
func (c *Config) GetDeviceByID(id string) *DeviceConfig {
	for i := range c.Devices {
		if c.Devices[i].ID == id {
			return &c.Devices[i]
		}
	}
	return nil
}

// GetEnabledDevices returns only enabled device configs
func (c *Config) GetEnabledDevices() []DeviceConfig {
	var enabled []DeviceConfig
	for _, d := range c.Devices {
		if d.Enabled {
			enabled = append(enabled, d)
		}
	}
	return enabled
}
