import { useEffect, useRef, useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { workflowApi } from '../api/workflows';

interface PollingOptions {
  instanceId: string;
  enabled?: boolean;
  interval?: number;
  onStatusChange?: (status: string) => void;
  onComplete?: () => void;
  onError?: (error: Error) => void;
}

const TERMINAL_STATUSES = ['Completed', 'Failed', 'Terminated', 'Canceled'];

export function useWorkflowPolling({
  instanceId,
  enabled = true,
  interval = 2000,
  onStatusChange,
  onComplete,
  onError,
}: PollingOptions) {
  const queryClient = useQueryClient();
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const lastStatusRef = useRef<string | null>(null);

  const checkStatus = useCallback(async () => {
    try {
      const workflow = await workflowApi.getInstance(instanceId);
      const currentStatus = workflow.Status || workflow.runtimeStatus || '';

      if (currentStatus !== lastStatusRef.current) {
        lastStatusRef.current = currentStatus;
        onStatusChange?.(currentStatus);

        // Invalidate related queries
        queryClient.invalidateQueries({ queryKey: ['workflow', instanceId] });
        queryClient.invalidateQueries({ queryKey: ['workflows'] });
      }

      if (TERMINAL_STATUSES.includes(currentStatus)) {
        onComplete?.();
        return true; // Signal to stop polling
      }

      return false;
    } catch (error) {
      onError?.(error as Error);
      return false;
    }
  }, [instanceId, queryClient, onStatusChange, onComplete, onError]);

  useEffect(() => {
    if (!enabled || !instanceId) {
      return;
    }

    // Initial check
    checkStatus();

    // Set up polling
    intervalRef.current = setInterval(async () => {
      const shouldStop = await checkStatus();
      if (shouldStop && intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    }, interval);

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    };
  }, [enabled, instanceId, interval, checkStatus]);

  const stopPolling = useCallback(() => {
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
  }, []);

  return { stopPolling };
}

// Hook for polling multiple workflows
export function useWorkflowsPolling({
  enabled = true,
  interval = 5000,
}: {
  enabled?: boolean;
  interval?: number;
} = {}) {
  const queryClient = useQueryClient();

  useEffect(() => {
    if (!enabled) return;

    const intervalId = setInterval(() => {
      queryClient.invalidateQueries({ queryKey: ['workflows'] });
    }, interval);

    return () => clearInterval(intervalId);
  }, [enabled, interval, queryClient]);
}
