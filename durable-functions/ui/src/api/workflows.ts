import { apiClient } from './client';
import type {
  WorkflowListResponse,
  WorkflowDetail,
  StartWorkflowRequest,
  StartWorkflowResponse,
} from './types';

export async function fetchWorkflows(): Promise<WorkflowListResponse> {
  const response = await apiClient.get<WorkflowListResponse>('/workflows');
  return response.data;
}

export async function fetchWorkflowDetail(instanceId: string): Promise<WorkflowDetail> {
  const response = await apiClient.get<WorkflowDetail>(`/workflows/${instanceId}`);
  return response.data;
}

export async function startWorkflow(request: StartWorkflowRequest): Promise<StartWorkflowResponse> {
  const response = await apiClient.post<StartWorkflowResponse>('/workflows', request);
  return response.data;
}

export async function raiseEvent(
  instanceId: string,
  eventName: string,
  eventData: unknown
): Promise<void> {
  await apiClient.post(`/workflows/${instanceId}/events/${eventName}`, eventData);
}

export async function terminateWorkflow(instanceId: string): Promise<void> {
  await apiClient.post(`/workflows/${instanceId}/terminate`);
}

export async function purgeWorkflow(instanceId: string): Promise<void> {
  await apiClient.delete(`/workflows/${instanceId}`);
}

export async function fetchWorkflowHistory(instanceId: string): Promise<unknown[]> {
  const response = await apiClient.get<unknown[]>(`/workflows/${instanceId}/history`);
  return response.data;
}

// Convenience object for named imports
export const workflowApi = {
  listInstances: fetchWorkflows,
  getInstance: fetchWorkflowDetail,
  start: startWorkflow,
  raiseEvent,
  terminate: terminateWorkflow,
  purge: purgeWorkflow,
  getHistory: fetchWorkflowHistory,
};
