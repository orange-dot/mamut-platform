import { useState } from 'react';
import { useActivities, useActivityDetail } from '../../hooks/useActivities';
import { LoadingSpinner } from '../common/LoadingSpinner';

export function ActivityList() {
  const { data, isLoading, error, refetch } = useActivities();
  const [selectedActivity, setSelectedActivity] = useState<string | null>(null);
  const [filterTag, setFilterTag] = useState<string>('all');

  // Get unique tags from all activities
  const allTags = data?.activities
    ? [...new Set(data.activities.flatMap(a => a.tags))]
    : [];

  // Filter activities by tag
  const filteredActivities = data?.activities.filter(
    a => filterTag === 'all' || a.tags.includes(filterTag)
  ) ?? [];

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-100">Activities</h1>
          <p className="text-gray-400">
            {data?.count ?? 0} registered activities
          </p>
        </div>
        <button
          onClick={() => refetch()}
          className="px-4 py-2 border border-dark-border text-gray-300 rounded-lg hover:bg-dark-hover"
        >
          Refresh
        </button>
      </div>

      {/* Tag Filter */}
      {allTags.length > 0 && (
        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => setFilterTag('all')}
            className={`px-3 py-1 rounded-full text-sm transition-colors ${
              filterTag === 'all'
                ? 'bg-blue-600 text-white'
                : 'bg-dark-card border border-dark-border text-gray-400 hover:text-gray-200'
            }`}
          >
            All
          </button>
          {allTags.map(tag => (
            <button
              key={tag}
              onClick={() => setFilterTag(tag)}
              className={`px-3 py-1 rounded-full text-sm transition-colors ${
                filterTag === tag
                  ? 'bg-blue-600 text-white'
                  : 'bg-dark-card border border-dark-border text-gray-400 hover:text-gray-200'
              }`}
            >
              {tag}
            </button>
          ))}
        </div>
      )}

      {isLoading && (
        <div className="flex justify-center py-12">
          <LoadingSpinner />
        </div>
      )}

      {error && (
        <div className="bg-red-900/30 text-red-400 p-4 rounded-lg border border-red-800">
          Error loading activities: {error.message}
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Activity List */}
        <div className="lg:col-span-2 space-y-3">
          {filteredActivities.map((activity) => (
            <button
              key={activity.name}
              onClick={() => setSelectedActivity(activity.name)}
              className={`w-full text-left bg-dark-card rounded-lg border p-4 transition-colors ${
                selectedActivity === activity.name
                  ? 'border-blue-600'
                  : 'border-dark-border hover:border-dark-hover'
              }`}
            >
              <div className="flex items-start justify-between">
                <div className="flex-1 min-w-0">
                  <h3 className="text-gray-100 font-semibold font-mono">
                    {activity.name}
                  </h3>
                  {activity.description && (
                    <p className="text-gray-400 text-sm mt-1">
                      {activity.description}
                    </p>
                  )}
                </div>
                {activity.supportsCompensation && (
                  <span className="ml-2 px-2 py-1 bg-orange-900/50 text-orange-400 text-xs rounded-full">
                    Compensatable
                  </span>
                )}
              </div>
              {activity.tags.length > 0 && (
                <div className="flex flex-wrap gap-1 mt-3">
                  {activity.tags.map(tag => (
                    <span
                      key={tag}
                      className="px-2 py-0.5 bg-dark-hover text-gray-400 text-xs rounded"
                    >
                      {tag}
                    </span>
                  ))}
                </div>
              )}
            </button>
          ))}

          {data && filteredActivities.length === 0 && (
            <div className="bg-dark-card rounded-lg border border-dark-border p-12 text-center">
              <p className="text-gray-500">No activities found with tag "{filterTag}".</p>
            </div>
          )}
        </div>

        {/* Activity Detail Panel */}
        <div className="lg:col-span-1">
          {selectedActivity ? (
            <ActivityDetailPanel activityName={selectedActivity} />
          ) : (
            <div className="bg-dark-card rounded-lg border border-dark-border p-6 text-center">
              <p className="text-gray-500">Select an activity to view details</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function ActivityDetailPanel({ activityName }: { activityName: string }) {
  const { data, isLoading, error } = useActivityDetail(activityName);

  if (isLoading) {
    return (
      <div className="bg-dark-card rounded-lg border border-dark-border p-6">
        <LoadingSpinner />
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className="bg-dark-card rounded-lg border border-dark-border p-6">
        <p className="text-red-400">Failed to load activity details</p>
      </div>
    );
  }

  return (
    <div className="bg-dark-card rounded-lg border border-dark-border p-6 space-y-4 sticky top-6">
      <h2 className="text-lg font-semibold text-gray-100 font-mono">{data.name}</h2>

      {data.description && (
        <p className="text-gray-400">{data.description}</p>
      )}

      <dl className="space-y-3">
        <div>
          <dt className="text-sm text-gray-500">Supports Compensation</dt>
          <dd className={data.supportsCompensation ? 'text-green-400' : 'text-gray-400'}>
            {data.supportsCompensation ? 'Yes' : 'No'}
          </dd>
        </div>

        {data.compensatingActivity && (
          <div>
            <dt className="text-sm text-gray-500">Compensating Activity</dt>
            <dd className="text-gray-200 font-mono">{data.compensatingActivity}</dd>
          </div>
        )}

        {data.inputType && (
          <div>
            <dt className="text-sm text-gray-500">Input Type</dt>
            <dd className="text-gray-200 font-mono">{data.inputType}</dd>
          </div>
        )}

        {data.outputType && (
          <div>
            <dt className="text-sm text-gray-500">Output Type</dt>
            <dd className="text-gray-200 font-mono">{data.outputType}</dd>
          </div>
        )}

        {data.tags.length > 0 && (
          <div>
            <dt className="text-sm text-gray-500 mb-1">Tags</dt>
            <dd className="flex flex-wrap gap-1">
              {data.tags.map(tag => (
                <span
                  key={tag}
                  className="px-2 py-0.5 bg-dark-hover text-gray-400 text-xs rounded"
                >
                  {tag}
                </span>
              ))}
            </dd>
          </div>
        )}
      </dl>

      {/* Usage Example */}
      <div className="pt-4 border-t border-dark-border">
        <h3 className="text-sm text-gray-500 mb-2">Usage in Workflow Definition</h3>
        <pre className="bg-dark-bg rounded-lg p-3 text-sm font-mono text-gray-300 overflow-auto">
{`{
  "Type": "Task",
  "Activity": "${data.name}",
  "Next": "NextState"
}`}
        </pre>
      </div>
    </div>
  );
}
