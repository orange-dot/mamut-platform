import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { workflowApi } from '../../api/workflows';
import styles from './MetricsDashboard.module.css';

interface MetricCardProps {
  title: string;
  value: number | string;
  icon: string;
  trend?: {
    value: number;
    isPositive: boolean;
  };
  color?: 'default' | 'success' | 'warning' | 'error';
}

function MetricCard({ title, value, icon, trend, color = 'default' }: MetricCardProps) {
  return (
    <div className={`${styles.card} ${styles[color]}`}>
      <div className={styles.cardIcon}>{icon}</div>
      <div className={styles.cardContent}>
        <div className={styles.cardTitle}>{title}</div>
        <div className={styles.cardValue}>{value}</div>
        {trend && (
          <div className={`${styles.cardTrend} ${trend.isPositive ? styles.positive : styles.negative}`}>
            {trend.isPositive ? '↑' : '↓'} {Math.abs(trend.value)}%
          </div>
        )}
      </div>
    </div>
  );
}

interface StatusBarProps {
  completed: number;
  running: number;
  failed: number;
  pending: number;
}

function StatusBar({ completed, running, failed, pending }: StatusBarProps) {
  const total = completed + running + failed + pending;
  if (total === 0) return null;

  const segments = [
    { value: completed, color: 'var(--success-color)', label: 'Completed' },
    { value: running, color: 'var(--info-color)', label: 'Running' },
    { value: failed, color: 'var(--error-color)', label: 'Failed' },
    { value: pending, color: 'var(--warning-color)', label: 'Pending' },
  ];

  return (
    <div className={styles.statusBarContainer}>
      <div className={styles.statusBar} role="img" aria-label="Workflow status distribution">
        {segments.map((segment, index) => (
          segment.value > 0 && (
            <div
              key={index}
              className={styles.statusSegment}
              style={{
                width: `${(segment.value / total) * 100}%`,
                backgroundColor: segment.color,
              }}
              title={`${segment.label}: ${segment.value}`}
            />
          )
        ))}
      </div>
      <div className={styles.statusLegend}>
        {segments.map((segment, index) => (
          segment.value > 0 && (
            <div key={index} className={styles.legendItem}>
              <span className={styles.legendDot} style={{ backgroundColor: segment.color }} />
              <span className={styles.legendLabel}>{segment.label}</span>
              <span className={styles.legendValue}>{segment.value}</span>
            </div>
          )
        ))}
      </div>
    </div>
  );
}

function SimpleChart({ data, height = 100 }: { data: number[]; height?: number }) {
  if (data.length === 0) return null;

  const max = Math.max(...data, 1);
  const min = Math.min(...data, 0);
  const range = max - min || 1;

  const points = data.map((value, index) => {
    const x = (index / (data.length - 1)) * 100;
    const y = height - ((value - min) / range) * height;
    return `${x},${y}`;
  }).join(' ');

  return (
    <svg className={styles.chart} viewBox={`0 0 100 ${height}`} preserveAspectRatio="none">
      <polyline
        points={points}
        fill="none"
        stroke="var(--primary-color)"
        strokeWidth="2"
        vectorEffect="non-scaling-stroke"
      />
      <polyline
        points={`0,${height} ${points} 100,${height}`}
        fill="url(#gradient)"
        opacity="0.2"
      />
      <defs>
        <linearGradient id="gradient" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="var(--primary-color)" />
          <stop offset="100%" stopColor="transparent" />
        </linearGradient>
      </defs>
    </svg>
  );
}

export function MetricsDashboard() {
  const { data: workflows } = useQuery({
    queryKey: ['workflows'],
    queryFn: () => workflowApi.listInstances(),
  });

  const metrics = useMemo(() => {
    const workflowList = workflows?.workflows || [];

    if (workflowList.length === 0) {
      return {
        total: 0,
        completed: 0,
        running: 0,
        failed: 0,
        pending: 0,
        successRate: 0,
        avgDuration: 'N/A',
        throughput: [],
      };
    }

    const statusCounts = workflowList.reduce(
      (acc: { completed: number; running: number; failed: number; pending: number }, wf) => {
        const status = wf.Status?.toLowerCase() || 'unknown';
        if (status === 'completed') acc.completed++;
        else if (status === 'running') acc.running++;
        else if (status === 'failed') acc.failed++;
        else if (status === 'pending') acc.pending++;
        return acc;
      },
      { completed: 0, running: 0, failed: 0, pending: 0 }
    );

    const finishedCount = statusCounts.completed + statusCounts.failed;
    const successRate = finishedCount > 0
      ? Math.round((statusCounts.completed / finishedCount) * 100)
      : 0;

    // Calculate throughput over last 7 days (mock data for now)
    const throughput = [12, 18, 15, 22, 19, 25, workflowList.length];

    return {
      total: workflowList.length,
      ...statusCounts,
      successRate,
      avgDuration: '2.5s',
      throughput,
    };
  }, [workflows]);

  return (
    <div className={styles.dashboard}>
      <div className={styles.header}>
        <h2 className={styles.title}>Workflow Metrics</h2>
        <span className={styles.subtitle}>Last 24 hours</span>
      </div>

      <div className={styles.metricsGrid}>
        <MetricCard
          title="Total Workflows"
          value={metrics.total}
          icon="📊"
        />
        <MetricCard
          title="Success Rate"
          value={`${metrics.successRate}%`}
          icon="✅"
          color={metrics.successRate >= 90 ? 'success' : metrics.successRate >= 70 ? 'warning' : 'error'}
        />
        <MetricCard
          title="Running"
          value={metrics.running}
          icon="⚡"
          color="default"
        />
        <MetricCard
          title="Failed"
          value={metrics.failed}
          icon="❌"
          color={metrics.failed > 0 ? 'error' : 'default'}
        />
      </div>

      <div className={styles.statusSection}>
        <h3 className={styles.sectionTitle}>Status Distribution</h3>
        <StatusBar
          completed={metrics.completed}
          running={metrics.running}
          failed={metrics.failed}
          pending={metrics.pending}
        />
      </div>

      <div className={styles.chartSection}>
        <h3 className={styles.sectionTitle}>Throughput (7 days)</h3>
        <div className={styles.chartContainer}>
          <SimpleChart data={metrics.throughput} />
        </div>
      </div>
    </div>
  );
}
