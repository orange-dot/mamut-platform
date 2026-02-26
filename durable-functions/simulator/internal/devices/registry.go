package devices

import (
	"fmt"
	"sync"

	"github.com/azure-durable/iot-simulator/internal/config"
	"github.com/rs/zerolog/log"
)

// Registry manages all simulated devices
type Registry struct {
	devices map[string]*Device
	mu      sync.RWMutex
}

// NewRegistry creates a new device registry
func NewRegistry() *Registry {
	return &Registry{
		devices: make(map[string]*Device),
	}
}

// LoadFromConfig loads devices from configuration
func (r *Registry) LoadFromConfig(cfg *config.Config) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	for _, deviceCfg := range cfg.Devices {
		device := NewDevice(deviceCfg)
		r.devices[device.ID] = device
		log.Info().
			Str("deviceId", device.ID).
			Str("type", device.Type).
			Bool("enabled", device.IsEnabled()).
			Msg("Device registered")
	}

	log.Info().Int("count", len(r.devices)).Msg("Devices loaded from config")
	return nil
}

// Register adds a device to the registry
func (r *Registry) Register(device *Device) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	if _, exists := r.devices[device.ID]; exists {
		return fmt.Errorf("device with ID %s already exists", device.ID)
	}

	r.devices[device.ID] = device
	log.Info().Str("deviceId", device.ID).Msg("Device registered")
	return nil
}

// Unregister removes a device from the registry
func (r *Registry) Unregister(deviceID string) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	if _, exists := r.devices[deviceID]; !exists {
		return fmt.Errorf("device with ID %s not found", deviceID)
	}

	delete(r.devices, deviceID)
	log.Info().Str("deviceId", deviceID).Msg("Device unregistered")
	return nil
}

// Get retrieves a device by ID
func (r *Registry) Get(deviceID string) (*Device, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	device, exists := r.devices[deviceID]
	if !exists {
		return nil, fmt.Errorf("device with ID %s not found", deviceID)
	}

	return device, nil
}

// GetAll returns all registered devices
func (r *Registry) GetAll() []*Device {
	r.mu.RLock()
	defer r.mu.RUnlock()

	devices := make([]*Device, 0, len(r.devices))
	for _, d := range r.devices {
		devices = append(devices, d)
	}
	return devices
}

// GetEnabled returns all enabled devices
func (r *Registry) GetEnabled() []*Device {
	r.mu.RLock()
	defer r.mu.RUnlock()

	var devices []*Device
	for _, d := range r.devices {
		if d.IsEnabled() {
			devices = append(devices, d)
		}
	}
	return devices
}

// GetByType returns all devices of a specific type
func (r *Registry) GetByType(deviceType string) []*Device {
	r.mu.RLock()
	defer r.mu.RUnlock()

	var devices []*Device
	for _, d := range r.devices {
		if d.Type == deviceType {
			devices = append(devices, d)
		}
	}
	return devices
}

// GetByState returns all devices in a specific state
func (r *Registry) GetByState(state DeviceState) []*Device {
	r.mu.RLock()
	defer r.mu.RUnlock()

	var devices []*Device
	for _, d := range r.devices {
		if d.GetState() == state {
			devices = append(devices, d)
		}
	}
	return devices
}

// Count returns the number of registered devices
func (r *Registry) Count() int {
	r.mu.RLock()
	defer r.mu.RUnlock()
	return len(r.devices)
}

// EnabledCount returns the number of enabled devices
func (r *Registry) EnabledCount() int {
	r.mu.RLock()
	defer r.mu.RUnlock()

	count := 0
	for _, d := range r.devices {
		if d.IsEnabled() {
			count++
		}
	}
	return count
}

// EnableAll enables all devices
func (r *Registry) EnableAll() {
	r.mu.RLock()
	defer r.mu.RUnlock()

	for _, d := range r.devices {
		d.Enable()
	}
	log.Info().Int("count", len(r.devices)).Msg("All devices enabled")
}

// DisableAll disables all devices
func (r *Registry) DisableAll() {
	r.mu.RLock()
	defer r.mu.RUnlock()

	for _, d := range r.devices {
		d.Disable()
	}
	log.Info().Int("count", len(r.devices)).Msg("All devices disabled")
}

// GetAllInfo returns info for all devices
func (r *Registry) GetAllInfo() []DeviceInfo {
	devices := r.GetAll()
	info := make([]DeviceInfo, len(devices))
	for i, d := range devices {
		info[i] = d.GetInfo()
	}
	return info
}

// GetStats returns registry statistics
func (r *Registry) GetStats() RegistryStats {
	r.mu.RLock()
	defer r.mu.RUnlock()

	stats := RegistryStats{
		Total:    len(r.devices),
		ByType:   make(map[string]int),
		ByState:  make(map[DeviceState]int),
	}

	for _, d := range r.devices {
		if d.IsEnabled() {
			stats.Enabled++
		}
		stats.ByType[d.Type]++
		stats.ByState[d.GetState()]++
	}

	return stats
}

// RegistryStats holds statistics about the device registry
type RegistryStats struct {
	Total   int                  `json:"total"`
	Enabled int                  `json:"enabled"`
	ByType  map[string]int       `json:"byType"`
	ByState map[DeviceState]int  `json:"byState"`
}
