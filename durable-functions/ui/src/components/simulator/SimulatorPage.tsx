import { useState, useEffect, useCallback } from 'react';
import styles from './SimulatorPage.module.css';

interface Device {
  id: string;
  name: string;
  type: string;
  location: string;
  state: string;
  enabled: boolean;
  lastSeen: string;
  telemetry: Record<string, unknown>;
  tags: Record<string, string>;
}

interface SimulationStatus {
  state: string;
  mode: string;
  uptime: number;
  devicesTotal: number;
  devicesEnabled: number;
  devicesOnline: number;
  eventsGenerated: number;
  eventsPublished: number;
  eventsFailed: number;
  cyclesCompleted: number;
  lastCycleAt: string;
  intervalMs: number;
  publisherConnected: boolean;
}

interface Scenario {
  name: string;
  description: string;
  steps: number;
}

const SIMULATOR_API = '/simulator-api/api/v1';

export function SimulatorPage() {
  const [status, setStatus] = useState<SimulationStatus | null>(null);
  const [devices, setDevices] = useState<Device[]>([]);
  const [scenarios, setScenarios] = useState<Scenario[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [runningScenario, setRunningScenario] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    try {
      const [statusRes, devicesRes, scenariosRes] = await Promise.all([
        fetch(`${SIMULATOR_API}/simulation/status`),
        fetch(`${SIMULATOR_API}/devices`),
        fetch(`${SIMULATOR_API}/scenarios`),
      ]);

      if (!statusRes.ok || !devicesRes.ok || !scenariosRes.ok) {
        throw new Error('Failed to fetch simulator data');
      }

      setStatus(await statusRes.json());
      setDevices(await devicesRes.json());
      setScenarios(await scenariosRes.json());
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Connection failed');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 2000);
    return () => clearInterval(interval);
  }, [fetchData]);

  const handleSimulationControl = async (action: 'start' | 'stop' | 'pause' | 'resume') => {
    try {
      const res = await fetch(`${SIMULATOR_API}/simulation/${action}`, { method: 'POST' });
      if (!res.ok) throw new Error(`Failed to ${action} simulation`);
      fetchData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Action failed');
    }
  };

  const handleRunScenario = async (name: string) => {
    setRunningScenario(name);
    try {
      const res = await fetch(`${SIMULATOR_API}/scenarios/run`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name }),
      });
      if (!res.ok) throw new Error('Failed to run scenario');
      fetchData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Scenario failed');
    } finally {
      setRunningScenario(null);
    }
  };

  const formatUptime = (ms: number) => {
    const seconds = Math.floor(ms / 1000000000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    if (hours > 0) return `${hours}h ${minutes % 60}m`;
    if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
    return `${seconds}s`;
  };

  const formatTime = (isoString: string) => {
    if (!isoString) return '-';
    const date = new Date(isoString);
    return date.toLocaleTimeString();
  };

  if (loading) {
    return (
      <div className={styles.container}>
        <div className={styles.loading}>Connecting to IoT Simulator...</div>
      </div>
    );
  }

  if (error && !status) {
    return (
      <div className={styles.container}>
        <div className={styles.card}>
          <div className={styles.error}>
            <h3>Simulator Unavailable</h3>
            <p>{error}</p>
            <p className={styles.hint}>
              Make sure the IoT Simulator is running on port 8085
            </p>
            <button onClick={fetchData} className={styles.retryButton}>
              Retry Connection
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <h1>IoT Device Simulator</h1>
        <div className={styles.connectionStatus}>
          <span className={`${styles.dot} ${status?.publisherConnected ? styles.connected : styles.disconnected}`} />
          {status?.publisherConnected ? 'Connected to Service Bus' : 'Disconnected'}
        </div>
      </div>

      {error && <div className={styles.errorBanner}>{error}</div>}

      {/* Status Cards */}
      <div className={styles.statsGrid}>
        <div className={styles.statCard}>
          <div className={styles.statLabel}>Status</div>
          <div className={`${styles.statValue} ${styles[status?.state || 'stopped']}`}>
            {status?.state?.toUpperCase() || 'UNKNOWN'}
          </div>
        </div>
        <div className={styles.statCard}>
          <div className={styles.statLabel}>Mode</div>
          <div className={styles.statValue}>{status?.mode || '-'}</div>
        </div>
        <div className={styles.statCard}>
          <div className={styles.statLabel}>Devices Online</div>
          <div className={styles.statValue}>
            {status?.devicesOnline || 0} / {status?.devicesTotal || 0}
          </div>
        </div>
        <div className={styles.statCard}>
          <div className={styles.statLabel}>Events Published</div>
          <div className={styles.statValue}>{status?.eventsPublished || 0}</div>
        </div>
        <div className={styles.statCard}>
          <div className={styles.statLabel}>Events Failed</div>
          <div className={`${styles.statValue} ${(status?.eventsFailed || 0) > 0 ? styles.error : ''}`}>
            {status?.eventsFailed || 0}
          </div>
        </div>
        <div className={styles.statCard}>
          <div className={styles.statLabel}>Uptime</div>
          <div className={styles.statValue}>{formatUptime(status?.uptime || 0)}</div>
        </div>
      </div>

      {/* Controls */}
      <div className={styles.card}>
        <h2>Simulation Controls</h2>
        <div className={styles.controls}>
          <button
            className={`${styles.controlButton} ${styles.start}`}
            onClick={() => handleSimulationControl('start')}
            disabled={status?.state === 'running'}
          >
            Start
          </button>
          <button
            className={`${styles.controlButton} ${styles.stop}`}
            onClick={() => handleSimulationControl('stop')}
            disabled={status?.state === 'stopped'}
          >
            Stop
          </button>
          <button
            className={`${styles.controlButton} ${styles.pause}`}
            onClick={() => handleSimulationControl('pause')}
            disabled={status?.state !== 'running'}
          >
            Pause
          </button>
          <button
            className={`${styles.controlButton} ${styles.resume}`}
            onClick={() => handleSimulationControl('resume')}
            disabled={status?.state !== 'paused'}
          >
            Resume
          </button>
        </div>
      </div>

      {/* Devices Grid */}
      <div className={styles.card}>
        <h2>Simulated Devices ({devices.length})</h2>
        <div className={styles.devicesGrid}>
          {devices.map((device) => (
            <div key={device.id} className={styles.deviceCard}>
              <div className={styles.deviceHeader}>
                <span className={`${styles.deviceDot} ${styles[device.state]}`} />
                <span className={styles.deviceName}>{device.name}</span>
                <span className={styles.deviceType}>{device.type}</span>
              </div>
              <div className={styles.deviceId}>{device.id}</div>
              <div className={styles.deviceLocation}>{device.location}</div>
              <div className={styles.deviceTelemetry}>
                {Object.entries(device.telemetry || {}).slice(0, 3).map(([key, value]) => (
                  <div key={key} className={styles.telemetryItem}>
                    <span className={styles.telemetryKey}>{key}:</span>
                    <span className={styles.telemetryValue}>
                      {typeof value === 'number' ? value.toFixed(1) : String(value)}
                    </span>
                  </div>
                ))}
              </div>
              <div className={styles.deviceFooter}>
                Last seen: {formatTime(device.lastSeen)}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Scenarios */}
      <div className={styles.card}>
        <h2>Test Scenarios</h2>
        <div className={styles.scenariosList}>
          {scenarios.map((scenario) => (
            <div key={scenario.name} className={styles.scenarioItem}>
              <div className={styles.scenarioInfo}>
                <div className={styles.scenarioName}>{scenario.name}</div>
                <div className={styles.scenarioDescription}>
                  {scenario.description} ({scenario.steps} steps)
                </div>
              </div>
              <button
                className={styles.runButton}
                onClick={() => handleRunScenario(scenario.name)}
                disabled={runningScenario !== null}
              >
                {runningScenario === scenario.name ? 'Running...' : 'Run'}
              </button>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

export default SimulatorPage;
