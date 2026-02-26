import { useQuery } from '@tanstack/react-query';
import { fetchActivities, fetchActivityDetail } from '../api/activities';

export function useActivities() {
  return useQuery({
    queryKey: ['activities'],
    queryFn: fetchActivities,
  });
}

export function useActivityDetail(activityName: string) {
  return useQuery({
    queryKey: ['activity', activityName],
    queryFn: () => fetchActivityDetail(activityName),
    enabled: !!activityName,
  });
}
