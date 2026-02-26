import { useQuery } from '@tanstack/react-query';
import { apiClient } from '../api/client';
import type {
  WorkflowDefinitionListResponse,
  WorkflowDefinitionDetail,
  DefinitionVersionsResponse,
} from '../api/types';

export function useDefinitions() {
  return useQuery({
    queryKey: ['definitions'],
    queryFn: async () => {
      const response = await apiClient.get<WorkflowDefinitionListResponse>('/definitions');
      return response.data;
    },
  });
}

export function useDefinitionDetail(definitionId: string, version?: string) {
  return useQuery({
    queryKey: ['definitions', definitionId, version],
    queryFn: async () => {
      const params = version ? { version } : {};
      const response = await apiClient.get<WorkflowDefinitionDetail>(
        `/definitions/${encodeURIComponent(definitionId)}`,
        { params }
      );
      return response.data;
    },
    enabled: !!definitionId,
  });
}

export function useDefinitionVersions(definitionId: string) {
  return useQuery({
    queryKey: ['definitions', definitionId, 'versions'],
    queryFn: async () => {
      const response = await apiClient.get<DefinitionVersionsResponse>(
        `/definitions/${encodeURIComponent(definitionId)}/versions`
      );
      return response.data;
    },
    enabled: !!definitionId,
  });
}
