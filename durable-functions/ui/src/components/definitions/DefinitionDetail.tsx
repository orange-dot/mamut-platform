import { useParams, Link } from '@tanstack/react-router';
import { useDefinitionDetail, useDefinitionVersions } from '../../hooks/useDefinitions';
import { JsonViewer } from '../common/JsonViewer';
import { LoadingSpinner } from '../common/LoadingSpinner';
import { StateMachineDiagram } from './StateMachineDiagram';
import { VersionDiff } from './VersionDiff';

const STATE_TYPE_COLORS: Record<string, string> = {
  Task: 'bg-blue-900/50 text-blue-400',
  Choice: 'bg-purple-900/50 text-purple-400',
  Wait: 'bg-yellow-900/50 text-yellow-400',
  Parallel: 'bg-indigo-900/50 text-indigo-400',
  Succeed: 'bg-green-900/50 text-green-400',
  Fail: 'bg-red-900/50 text-red-400',
  Compensation: 'bg-orange-900/50 text-orange-400',
};

export function DefinitionDetail() {
  const { definitionId } = useParams({ strict: false }) as { definitionId: string };
  const { data, isLoading, error } = useDefinitionDetail(definitionId);
  const { data: versionsData } = useDefinitionVersions(definitionId);

  if (isLoading) {
    return (
      <div className="flex justify-center py-12">
        <LoadingSpinner />
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-900/30 text-red-400 p-4 rounded-lg border border-red-800">
        Error loading definition: {error.message}
      </div>
    );
  }

  if (!data) {
    return (
      <div className="bg-yellow-900/30 text-yellow-400 p-4 rounded-lg border border-yellow-800">
        Definition not found
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <Link
          to="/definitions"
          className="text-gray-400 hover:text-gray-200"
        >
          &larr; Back to definitions
        </Link>
      </div>

      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-100">{data.name}</h1>
          <p className="text-gray-400 font-mono">{data.id}</p>
        </div>
        <div className="flex items-center gap-2">
          <span className="px-3 py-1 bg-blue-900/50 text-blue-400 rounded-full text-sm">
            v{data.version}
          </span>
        </div>
      </div>

      {data.description && (
        <div className="bg-dark-card rounded-lg border border-dark-border p-4">
          <p className="text-gray-300">{data.description}</p>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Info Card */}
        <div className="bg-dark-card rounded-lg border border-dark-border p-6">
          <h2 className="text-lg font-semibold text-gray-100 mb-4">Details</h2>
          <dl className="space-y-3">
            <div>
              <dt className="text-sm text-gray-400">Start State</dt>
              <dd className="text-gray-200 font-mono">{data.startAt}</dd>
            </div>
            <div>
              <dt className="text-sm text-gray-400">Total States</dt>
              <dd className="text-gray-200">{Object.keys(data.states || {}).length}</dd>
            </div>
            {data.config?.timeoutSeconds && (
              <div>
                <dt className="text-sm text-gray-400">Timeout</dt>
                <dd className="text-gray-200">{data.config.timeoutSeconds}s</dd>
              </div>
            )}
            {data.config?.compensationState && (
              <div>
                <dt className="text-sm text-gray-400">Compensation State</dt>
                <dd className="text-gray-200 font-mono">{data.config.compensationState}</dd>
              </div>
            )}
          </dl>
        </div>

        {/* Versions Card */}
        <div className="bg-dark-card rounded-lg border border-dark-border p-6">
          <h2 className="text-lg font-semibold text-gray-100 mb-4">Versions</h2>
          {versionsData?.versions && versionsData.versions.length > 0 ? (
            <ul className="space-y-2">
              {versionsData.versions.map((version) => (
                <li key={version} className="flex items-center justify-between">
                  <span className="font-mono text-sm text-gray-300">{version}</span>
                  {version === data.version && (
                    <span className="text-xs text-green-400">current</span>
                  )}
                </li>
              ))}
            </ul>
          ) : (
            <p className="text-gray-500 text-sm">No version history</p>
          )}
        </div>

        {/* Retry Policy Card */}
        {data.config?.defaultRetryPolicy && (
          <div className="bg-dark-card rounded-lg border border-dark-border p-6">
            <h2 className="text-lg font-semibold text-gray-100 mb-4">Default Retry Policy</h2>
            <dl className="space-y-2">
              <div className="flex justify-between">
                <dt className="text-gray-400">Max Attempts</dt>
                <dd className="text-gray-200">{data.config.defaultRetryPolicy.maxAttempts}</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-gray-400">Initial Interval</dt>
                <dd className="text-gray-200">{data.config.defaultRetryPolicy.initialIntervalSeconds}s</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-gray-400">Backoff Coefficient</dt>
                <dd className="text-gray-200">{data.config.defaultRetryPolicy.backoffCoefficient}x</dd>
              </div>
            </dl>
          </div>
        )}
      </div>

      {/* Version Comparison */}
      <VersionDiff definitionId={definitionId} currentVersion={data.version} />

      {/* State Machine Diagram */}
      <div className="bg-dark-card rounded-lg border border-dark-border p-6 relative">
        <h2 className="text-lg font-semibold text-gray-100 mb-4">State Machine Diagram</h2>
        <StateMachineDiagram definition={data} />
      </div>

      {/* States List */}
      <div className="bg-dark-card rounded-lg border border-dark-border p-6">
        <h2 className="text-lg font-semibold text-gray-100 mb-4">States</h2>
        <div className="space-y-4">
          {Object.entries(data.states || {}).map(([stateName, state]) => (
            <div
              key={stateName}
              className={`border rounded-lg p-4 ${
                stateName === data.startAt ? 'border-green-800 bg-green-900/20' : 'border-dark-border'
              }`}
            >
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-2">
                  <h3 className="font-semibold text-gray-100 font-mono">{stateName}</h3>
                  {stateName === data.startAt && (
                    <span className="text-xs text-green-400">(start)</span>
                  )}
                </div>
                <span className={`px-2 py-1 rounded-full text-xs ${STATE_TYPE_COLORS[state.type || ''] || 'bg-gray-800 text-gray-400'}`}>
                  {state.type}
                </span>
              </div>

              {state.comment && (
                <p className="text-sm text-gray-400 mb-2">{state.comment}</p>
              )}

              <div className="flex flex-wrap gap-2 text-sm">
                {state.activity && (
                  <span className="px-2 py-1 bg-dark-hover text-gray-300 rounded">
                    Activity: {state.activity}
                  </span>
                )}
                {state.next && (
                  <span className="px-2 py-1 bg-dark-hover text-gray-300 rounded">
                    Next: {state.next}
                  </span>
                )}
                {state.default && (
                  <span className="px-2 py-1 bg-dark-hover text-gray-300 rounded">
                    Default: {state.default}
                  </span>
                )}
                {state.end && (
                  <span className="px-2 py-1 bg-green-900/50 text-green-400 rounded">
                    End State
                  </span>
                )}
                {state.externalEvent && (
                  <span className="px-2 py-1 bg-yellow-900/50 text-yellow-400 rounded">
                    Waits for: {state.externalEvent.eventName}
                  </span>
                )}
                {state.compensateWith && (
                  <span className="px-2 py-1 bg-orange-900/50 text-orange-400 rounded">
                    Compensate: {state.compensateWith}
                  </span>
                )}
              </div>

              {state.choices && state.choices.length > 0 && (
                <div className="mt-2 pl-4 border-l-2 border-purple-800">
                  <p className="text-xs text-gray-500 mb-1">Choices:</p>
                  {state.choices.map((choice, idx) => (
                    <div key={idx} className="text-sm text-gray-400">
                      if <code className="text-purple-400">{choice.condition.variable}</code>{' '}
                      {choice.condition.comparisonType} {JSON.stringify(choice.condition.value)}{' '}
                      &rarr; <span className="font-mono text-gray-300">{choice.next}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      </div>

      {/* Raw JSON */}
      <div className="bg-dark-card rounded-lg border border-dark-border p-6">
        <h2 className="text-lg font-semibold text-gray-100 mb-4">Raw Definition</h2>
        <JsonViewer data={data} />
      </div>
    </div>
  );
}
