import { apiClient } from './client';

export interface ActivitySummary {
  name: string;
  description?: string;
  supportsCompensation: boolean;
  tags: string[];
}

export interface ActivityDetail {
  name: string;
  description?: string;
  supportsCompensation: boolean;
  compensatingActivity?: string;
  tags: string[];
  inputType?: string;
  outputType?: string;
}

export interface ActivityListResponse {
  count: number;
  activities: ActivitySummary[];
}

export async function fetchActivities(): Promise<ActivityListResponse> {
  const response = await apiClient.get<ActivityListResponse>('/activities');
  return response.data;
}

export async function fetchActivityDetail(activityName: string): Promise<ActivityDetail> {
  const response = await apiClient.get<ActivityDetail>(`/activities/${activityName}`);
  return response.data;
}
