import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { workflowApi } from '../../api/workflows';
import styles from './WorkflowHistory.module.css';

interface WorkflowHistoryProps {
  instanceId: string;
}

interface HistoryEvent {
  timestamp: string;
  eventType: string;
  name?: string;
  input?: unknown;
  result?: unknown;
  reason?: string;
  details?: string;
}

export function WorkflowHistory({ instanceId }: WorkflowHistoryProps) {
  const [expandedEvents, setExpandedEvents] = useState<Set<number>>(new Set());

  const { data: history, isLoading, error } = useQuery({
    queryKey: ['workflow-history', instanceId],
    queryFn: () => workflowApi.getHistory(instanceId),
    refetchInterval: 5000, // Poll every 5 seconds
  });

  const toggleEvent = (index: number) => {
    setExpandedEvents(prev => {
      const next = new Set(prev);
      if (next.has(index)) {
        next.delete(index);
      } else {
        next.add(index);
      }
      return next;
    });
  };

  const getEventIcon = (eventType: string): string => {
    if (eventType.includes('Started')) return '▶️';
    if (eventType.includes('Completed')) return '✅';
    if (eventType.includes('Failed')) return '❌';
    if (eventType.includes('Timer')) return '⏱️';
    if (eventType.includes('Event')) return '📨';
    if (eventType.includes('Task')) return '📋';
    if (eventType.includes('SubOrchestration')) return '🔄';
    return '📌';
  };

  const formatTimestamp = (timestamp: string): string => {
    return new Date(timestamp).toLocaleString();
  };

  if (isLoading) {
    return (
      <div className={styles.loading}>
        <div className={styles.spinner} aria-label="Loading history" />
        Loading workflow history...
      </div>
    );
  }

  if (error) {
    return (
      <div className={styles.error} role="alert">
        <span>⚠️</span>
        Failed to load workflow history
      </div>
    );
  }

  const events: HistoryEvent[] = (history as HistoryEvent[]) || [];

  if (events.length === 0) {
    return (
      <div className={styles.empty}>
        No history events available yet
      </div>
    );
  }

  return (
    <div className={styles.container}>
      <h3 className={styles.title}>Execution History</h3>
      <div className={styles.timeline}>
        {events.map((event, index) => (
          <div
            key={index}
            className={`${styles.event} ${expandedEvents.has(index) ? styles.expanded : ''}`}
          >
            <button
              className={styles.eventHeader}
              onClick={() => toggleEvent(index)}
              aria-expanded={expandedEvents.has(index)}
            >
              <span className={styles.eventIcon}>{getEventIcon(event.eventType)}</span>
              <span className={styles.eventType}>{event.eventType}</span>
              {event.name && <span className={styles.eventName}>{event.name}</span>}
              <span className={styles.eventTime}>{formatTimestamp(event.timestamp)}</span>
              <span className={styles.expandIcon}>{expandedEvents.has(index) ? '▼' : '▶'}</span>
            </button>
            {expandedEvents.has(index) && (
              <div className={styles.eventDetails}>
                {event.input !== undefined && (
                  <div className={styles.detailSection}>
                    <span className={styles.detailLabel}>Input:</span>
                    <pre className={styles.detailValue}>
                      {JSON.stringify(event.input, null, 2)}
                    </pre>
                  </div>
                )}
                {event.result !== undefined && (
                  <div className={styles.detailSection}>
                    <span className={styles.detailLabel}>Result:</span>
                    <pre className={styles.detailValue}>
                      {JSON.stringify(event.result, null, 2)}
                    </pre>
                  </div>
                )}
                {event.reason && (
                  <div className={styles.detailSection}>
                    <span className={styles.detailLabel}>Reason:</span>
                    <span className={styles.detailValue}>{event.reason}</span>
                  </div>
                )}
                {event.details && (
                  <div className={styles.detailSection}>
                    <span className={styles.detailLabel}>Details:</span>
                    <span className={styles.detailValue}>{event.details}</span>
                  </div>
                )}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
