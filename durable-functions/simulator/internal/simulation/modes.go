package simulation

// Mode represents a simulation mode
type Mode string

const (
	// ModeConstant emits events at fixed intervals
	ModeConstant Mode = "constant"

	// ModeRandom adds jitter and randomness to event emission
	ModeRandom Mode = "random"

	// ModeScenario follows predefined scenario scripts
	ModeScenario Mode = "scenario"

	// ModeBurst emits many events rapidly for stress testing
	ModeBurst Mode = "burst"
)

// ModeFromString converts a string to Mode
func ModeFromString(s string) Mode {
	switch s {
	case "constant":
		return ModeConstant
	case "random":
		return ModeRandom
	case "scenario":
		return ModeScenario
	case "burst":
		return ModeBurst
	default:
		return ModeRandom
	}
}

// IsValid checks if the mode is valid
func (m Mode) IsValid() bool {
	switch m {
	case ModeConstant, ModeRandom, ModeScenario, ModeBurst:
		return true
	default:
		return false
	}
}

// String returns the string representation
func (m Mode) String() string {
	return string(m)
}
