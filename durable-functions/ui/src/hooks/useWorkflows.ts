import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { fetchWorkflows, fetchWorkflowDetail, startWorkflow, terminateWorkflow, raiseEvent } from '../api/workflows';
import type { StartWorkflowRequest } from '../api/types';

export function useWorkflows() {
  return useQuery({
    queryKey: ['workflows'],
    queryFn: fetchWorkflows,
    refetchInterval: 10000, // Refetch every 10 seconds
  });
}

export function useWorkflowDetail(instanceId: string) {
  return useQuery({
    queryKey: ['workflow', instanceId],
    queryFn: () => fetchWorkflowDetail(instanceId),
    enabled: !!instanceId,
    refetchInterval: 5000, // Refetch every 5 seconds for running workflows
  });
}

export function useStartWorkflow() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: StartWorkflowRequest) => startWorkflow(request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['workflows'] });
    },
  });
}

export function useTerminateWorkflow() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (instanceId: string) => terminateWorkflow(instanceId),
    onSuccess: (_data, instanceId) => {
      queryClient.invalidateQueries({ queryKey: ['workflows'] });
      queryClient.invalidateQueries({ queryKey: ['workflow', instanceId] });
    },
  });
}

export function useRaiseEvent() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ instanceId, eventName, eventData }: { instanceId: string; eventName: string; eventData: unknown }) =>
      raiseEvent(instanceId, eventName, eventData),
    onSuccess: (_data, { instanceId }) => {
      queryClient.invalidateQueries({ queryKey: ['workflow', instanceId] });
    },
  });
}
